use super::{failure, validate};
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const FOLDER: &str = "DentalSchedulerMiniData";
pub const DATABASE: &str = "scheduler.sqlite";
pub const METADATA: &str = "dental-scheduler-data.json";
pub const LOCK: &str = ".scheduler-session.lock";
pub const PORTABLE_MARKER: &str = "dental-scheduler-portable.json";

/// A packaged USB application follows its executable, never the working directory
/// or a different database remembered on the host computer. Reading this marker
/// does not create or replace a database.
pub fn package_directory(executable: &Path) -> AppResult<Option<PathBuf>> {
    let parent = executable.parent().ok_or_else(|| failure("INVALID_STORAGE_PATH"))?;
    let marker = parent.join(PORTABLE_MARKER);
    let metadata = match fs::symlink_metadata(&marker) {
        Ok(value) => value,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(failure("INVALID_PORTABLE_PACKAGE")),
    };
    validate_path(&marker)?;
    if !metadata.is_file() || metadata.len() > 4096 {
        return Err(failure("INVALID_PORTABLE_PACKAGE"));
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Marker { format_version: u32, data_folder: String }
    let value: Marker = serde_json::from_slice(&fs::read(marker).map_err(|_| failure("INVALID_PORTABLE_PACKAGE"))?)
        .map_err(|_| failure("INVALID_PORTABLE_PACKAGE"))?;
    if value.format_version != 1 || value.data_folder != FOLDER {
        return Err(failure("INVALID_PORTABLE_PACKAGE"));
    }
    let directory = parent.join(FOLDER);
    validate_path(&directory)?;
    Ok(Some(directory))
}

// Manual selections are explicit, fully qualified, directly attached paths.
// Reject traversal and existing symlink/reparse ancestors before opening files.
pub fn validate_path(path: &Path) -> AppResult<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(failure("INVALID_STORAGE_PATH"));
    }
    #[cfg(target_os = "windows")]
    {
        use std::path::{Component, Prefix};
        let drive = match path.components().next() {
            Some(Component::Prefix(p)) => match p.kind() {
                Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d,
                _ => return Err(failure("INVALID_STORAGE_PATH")),
            },
            _ => return Err(failure("INVALID_STORAGE_PATH")),
        };
        #[link(name = "kernel32")]
        extern "system" { fn GetDriveTypeW(root: *const u16) -> u32; }
        let root: Vec<u16> = format!("{}:\\", drive as char).encode_utf16().chain(Some(0)).collect();
        if !matches!(unsafe { GetDriveTypeW(root.as_ptr()) }, 2 | 3) {
            return Err(failure("INVALID_STORAGE_PATH"));
        }
    }
    for ancestor in path.ancestors() {
        // macOS exposes its platform temporary directories through these two
        // system aliases. They are not user-selected redirects to other data.
        #[cfg(target_os = "macos")]
        if (ancestor == Path::new("/var") && fs::read_link(ancestor).ok().as_deref() == Some(Path::new("private/var")))
            || (ancestor == Path::new("/tmp") && fs::read_link(ancestor).ok().as_deref() == Some(Path::new("private/tmp"))) { continue; }
        if let Ok(metadata) = fs::symlink_metadata(ancestor) {
            if metadata.is_symlink() { return Err(failure("INVALID_STORAGE_PATH")); }
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 { return Err(failure("INVALID_STORAGE_PATH")); }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metadata {
    #[serde(rename = "type")]
    pub kind: String,
    pub format_version: u32,
    pub database_filename: String,
    pub data_id: String,
}
impl Metadata {
    pub fn fresh() -> Self {
        Self {
            kind: "DentalSchedulerMiniData".into(),
            format_version: 1,
            database_filename: DATABASE.into(),
            data_id: uuid::Uuid::new_v4().to_string(),
        }
    }
    pub fn read(dir: &Path) -> AppResult<Self> {
        let path = dir.join(METADATA);
        if fs::symlink_metadata(&path).map_or(true, |m| !m.is_file() || m.len() > 4096) {
            return Err(failure("INVALID_DATA_METADATA"));
        }
        let value: Self =
            serde_json::from_slice(&fs::read(&path).map_err(|_| failure("INVALID_DATA_METADATA"))?)
                .map_err(|_| failure("INVALID_DATA_METADATA"))?;
        if value.kind != "DentalSchedulerMiniData"
            || value.format_version != 1
            || value.database_filename != DATABASE
            || uuid::Uuid::parse_str(&value.data_id).is_err()
        {
            return Err(failure("INVALID_DATA_METADATA"));
        }
        Ok(value)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Local,
    Portable,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub mode: Mode,
    pub preferred_id: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub directory: String,
    pub data_id: String,
    pub schema_version: Option<i64>,
    pub error_code: Option<String>,
}

#[derive(Debug)]
pub struct DatabaseLocationService {
    pub app_data: PathBuf,
    pub logs: PathBuf,
    pub settings: Settings,
    pub local_override: Option<PathBuf>,
    pub portable_package_directory: Option<PathBuf>,
}
impl DatabaseLocationService {
    pub fn new(app_data: PathBuf, logs: PathBuf, default: Mode) -> AppResult<Self> {
        // Platform-resolved paths are independent of process working directory.
        if !app_data.is_absolute() || !logs.is_absolute() {
            return Err(failure("APPLICATION_DATA_DIRECTORY_UNAVAILABLE"));
        }
        fs::create_dir_all(&app_data).map_err(|_| failure("LOCAL_SETTINGS_FAILED"))?;
        let path = app_data.join("storage-settings.json");
        let settings = if path.exists() {
            serde_json::from_slice(&fs::read(path).map_err(|_| failure("LOCAL_SETTINGS_FAILED"))?)
                .map_err(|_| failure("LOCAL_SETTINGS_FAILED"))?
        } else {
            Settings {
                mode: default,
                preferred_id: None,
            }
        };
        Ok(Self {
            app_data,
            logs,
            settings,
            local_override: None,
            portable_package_directory: None,
        })
    }
    pub fn local_path(&self) -> PathBuf {
        self.local_override
            .clone()
            .unwrap_or_else(|| self.app_data.join("development").join(DATABASE))
    }
    pub fn remember(&mut self, mode: Mode, id: Option<String>) -> AppResult<()> {
        let next = Settings {
            mode,
            preferred_id: id,
        };
        atomic_json(&self.app_data.join("storage-settings.json"), &next)?;
        self.settings = next;
        Ok(())
    }
    pub fn discover(&self) -> Vec<Candidate> {
        Self::discover_on(&volume_roots())
    }
    pub fn discover_on(roots: &[PathBuf]) -> Vec<Candidate> {
        let mut result = Vec::new();
        for root in roots {
            for folder in [FOLDER, "Dental Scheduler Mini Data"] {
                let dir = root.join(folder);
                if validate_path(&dir).is_err() { continue; }
                if let Ok(metadata) = Metadata::read(&dir) {
                    let valid = validate::inspect(&dir.join(DATABASE));
                    result.push(Candidate {
                        directory: dir.to_string_lossy().into_owned(),
                        data_id: metadata.data_id,
                        schema_version: valid.as_ref().ok().copied(),
                        error_code: valid.err().map(|e| e.code),
                    });
                }
            }
        }
        result.sort_by(|a, b| a.directory.cmp(&b.directory));
        result.dedup_by(|a, b| a.directory == b.directory);
        result
    }
    pub fn choose(&self, candidates: &[Candidate]) -> AppResult<PathBuf> {
        let matching: Vec<_> = candidates
            .iter()
            .filter(|c| {
                self.settings
                    .preferred_id
                    .as_ref()
                    .map_or(true, |id| id == &c.data_id)
            })
            .collect();
        match matching.as_slice() {
            [] => Err(failure(if candidates.is_empty() {
                "USB_NOT_FOUND"
            } else {
                "PORTABLE_DATABASE_NOT_FOUND"
            })),
            [one] => Ok(PathBuf::from(&one.directory)),
            _ => Err(failure("MULTIPLE_DATABASES_FOUND")),
        }
    }
}
pub fn atomic_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(&serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?)?;
        f.sync_all()?;
        fs::rename(&tmp, path)?;
        sync_parent(path)?;
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result.map_err(|_| failure("LOCAL_SETTINGS_FAILED"))
}
pub fn sync_parent(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(
            path.parent()
                .ok_or_else(|| std::io::Error::other("parent"))?,
        )?
        .sync_all()?;
    }
    Ok(())
}
#[cfg(target_os = "windows")]
fn volume_roots() -> Vec<PathBuf> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLogicalDrives() -> u32;
        fn GetDriveTypeW(root: *const u16) -> u32;
    }
    let mask = unsafe { GetLogicalDrives() };
    let mut result = vec![];
    for bit in 0..26 {
        if mask & (1 << bit) != 0 {
            let root = format!("{}:\\", (b'A' + bit) as char);
            let wide: Vec<_> = root.encode_utf16().chain(Some(0)).collect();
            if unsafe { GetDriveTypeW(wide.as_ptr()) } == 2 {
                result.push(PathBuf::from(root));
            }
        }
    }
    result
}
#[cfg(target_os = "macos")]
fn volume_roots() -> Vec<PathBuf> {
    children(Path::new("/Volumes"))
}
#[cfg(all(unix, not(target_os = "macos")))]
fn volume_roots() -> Vec<PathBuf> {
    let user = std::env::var("USER").unwrap_or_default();
    if user.is_empty() || user.contains('/') {
        return vec![];
    }
    [
        PathBuf::from("/media").join(&user),
        PathBuf::from("/run/media").join(&user),
    ]
    .iter()
    .flat_map(|p| children(p))
    .collect()
}
#[cfg(unix)]
fn children(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|f| f.is_dir()))
        .map(|e| e.path())
        .collect()
}
#[cfg(not(any(unix, target_os = "windows")))]
fn volume_roots() -> Vec<PathBuf> {
    vec![]
}
