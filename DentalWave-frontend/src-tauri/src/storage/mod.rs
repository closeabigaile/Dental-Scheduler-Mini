pub mod backup;
pub mod location;
#[cfg(test)]
mod tests;
pub mod validate;
use crate::{
    database::Database,
    error::{AppError, AppResult},
};
use location::{DatabaseLocationService, Metadata, Mode, DATABASE, LOCK};
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn write_log(dir: &Path, code: &str) {
        if fs::create_dir_all(dir).is_ok() {
            let path = dir.join("storage.log");
            if fs::metadata(&path).is_ok_and(|m| m.len() > 1_000_000) {
                let _ = fs::rename(&path, dir.join("storage.previous.log"));
            }
            if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "{} version={} {}", backup::timestamp(), env!("CARGO_PKG_VERSION"), code);
            }
        }
    }

pub fn failure(code: &str) -> AppError {
    let message=match code {
    "APPLICATION_DATA_DIRECTORY_UNAVAILABLE"=>"The application data directory is unavailable. Ask your support team to check your user profile and folder access.",
    "LOCAL_SETTINGS_FAILED"=>"Local application settings could not be read or saved. Ask your support team to check folder access and settings integrity.",
    "INVALID_PORTABLE_PACKAGE"=>"The portable application marker is invalid. Restore the application files from the original ZIP without replacing your schedule data folder.",
    "INVALID_STORAGE_PATH"=>"Choose a full path on a directly attached local or removable drive. Relative paths and network shares are not supported.",
    "USB_NOT_FOUND"=>"No schedule drive was found. Connect the drive or select its data folder.",
    "PORTABLE_DATABASE_NOT_FOUND"=>"The schedule database was not found. Select the existing data folder; no replacement has been created.",
    "INVALID_DATA_METADATA"=>"This folder does not identify a Dental Scheduler Mini database.",
    "MULTIPLE_DATABASES_FOUND"=>"More than one schedule database was found. Select the intended folder.",
    "DATABASE_READ_ONLY"=>"This database is read-only. You can view schedules, but changes cannot be saved.",
    "DATABASE_ALREADY_IN_USE"=>"Another application session is using this database. Close it before trying again.",
    "DATABASE_DAMAGED"=>"The database did not pass validation. Close it or restore a verified backup.",
    "DATABASE_VERSION_TOO_NEW"=>"This database needs a newer version of Dental Scheduler Mini. It has not been opened for editing.",
    "DATABASE_RECOVERY_REQUIRED"=>"The database needs recovery after an interrupted session. Use a writable copy or restore a verified backup.",
    "PORTABLE_STORAGE_REMOVED"=>"The schedule drive is unavailable or has changed. Your unsaved work remains on screen. Reconnect it, then close and reopen the database.",
    "DATABASE_WRITE_FAILED"=>"The change could not be saved. Keep this window open and check the schedule drive.",
    "BACKUP_FAILED"=>"A verified backup could not be completed. Check free space and drive access, then retry.",
    "INVALID_BACKUP"=>"This is not a verified, compatible backup for this database.",
    "RESTORE_FAILED"=>"Restore did not finish. Original data and recovery files have been preserved. Retry with a verified backup.",
    "RESTORE_INTERRUPTED"=>"A previous restore was interrupted. Editing is blocked. Select a verified backup to complete recovery.",
    "SESSION_CLOSED"=>"Open a schedule database in Data before continuing.",
    "SESSION_OPEN"=>"Close the current database before selecting another location.",
    "INITIALIZATION_TARGET_EXISTS"=>"Choose a new data folder. Existing data folders will not be overwritten.",
    "UPGRADE_REQUIRES_WRITE"=>"This database needs an upgrade. Use a writable drive so a verified backup can be made first.",
    _=>"The storage operation could not be completed. Check the drive and retry.",
  };
    AppError::new(code, message)
}
#[derive(Debug)]
struct Session {
    db: Database,
    metadata: Metadata,
    mode: Mode,
    _lock: fs::File,
    volume_protected: bool,
    identity: Option<same_file::Handle>,
    fault: Option<String>,
    read_only: bool,
    schema: Option<i64>,
    key: String,
    last_backup: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub mode: Mode,
    pub active: bool,
    pub directory: Option<String>,
    pub data_id: Option<String>,
    pub access: &'static str,
    pub error_code: Option<String>,
    pub message: Option<String>,
    pub schema_version: Option<i64>,
    pub session_key: Option<String>,
    pub last_backup: Option<String>,
    pub portable_package_directory: Option<String>,
}
#[derive(Debug)]
pub struct StorageManager {
    pub locations: DatabaseLocationService,
    session: Option<Session>,
    last_error: Option<String>,
    recent: Vec<String>,
}
impl StorageManager {
    pub fn new(locations: DatabaseLocationService) -> Self {
        Self {
            locations,
            session: None,
            last_error: None,
            recent: vec![],
        }
    }
    pub fn active(&self) -> bool {
        self.session.is_some()
    }
    pub fn event(&mut self, code: &str) {
        // Codes are authored by this application, never SQLite messages or row data.
        self.recent.push(code.into());
        if self.recent.len() > 30 {
            self.recent.remove(0);
        }
        write_log(&self.locations.logs, code);
    }
    pub fn status(&mut self) -> Status {
        let mut detected = None;
        if let Some(s) = &mut self.session {
            if s.fault.is_none() {
                if let Err(e) = Self::preflight(s, false) {
                    detected = Some(e.code.clone());
                    s.fault = Some(e.code);
                }
            }
        }
        if let Some(code) = detected {
            self.event(&code);
        }
        let s = self.session.as_ref();
        let error = s
            .and_then(|s| s.fault.clone())
            .or_else(|| self.last_error.clone());
        Status {
            mode: s.map_or_else(|| self.locations.settings.mode.clone(), |s| s.mode.clone()),
            active: s.is_some(),
            directory: s.map(|s| s.db.path().parent().unwrap().to_string_lossy().into_owned()),
            data_id: s.map(|s| s.metadata.data_id.clone()),
            access: match s {
                None => "UNAVAILABLE",
                Some(s) if s.fault.is_some() => "UNAVAILABLE",
                Some(s) if s.read_only => "READ_ONLY",
                _ => "READABLE_AND_WRITABLE",
            },
            message: error.as_deref().map(|c| failure(c).message),
            error_code: error,
            schema_version: s.and_then(|s| s.schema),
            session_key: s.map(|s| s.key.clone()),
            last_backup: s.and_then(|s| s.last_backup.clone()),
            portable_package_directory: self.locations.portable_package_directory.as_ref().map(|p| p.to_string_lossy().into_owned()),
        }
    }
    pub fn open(&mut self, mode: Mode, directory: Option<PathBuf>) -> AppResult<Status> {
        if self.active() {
            return Err(failure("SESSION_OPEN"));
        }
        let result = self.open_inner(mode, directory);
        if let Err(e) = &result {
            self.last_error = Some(e.code.clone());
            self.event(&e.code);
        }
        result?;
        Ok(self.status())
    }
    fn open_inner(&mut self, mode: Mode, directory: Option<PathBuf>) -> AppResult<()> {
        let path = match mode {
            Mode::Portable => match directory {
                Some(dir) => dir,
                None => match &self.locations.portable_package_directory {
                    Some(dir) => dir.clone(),
                    None => self.locations.choose(&self.locations.discover())?,
                },
            }
            .join(DATABASE),
            Mode::Local => self.locations.local_path(),
        };
        location::validate_path(&path)?;
        let dir = path
            .parent()
            .ok_or_else(|| failure("PORTABLE_DATABASE_NOT_FOUND"))?;
        if mode == Mode::Local {
            fs::create_dir_all(dir).map_err(|_| failure("LOCAL_SETTINGS_FAILED"))?;
        }
        if dir.is_symlink() {
            return Err(failure("INVALID_DATA_METADATA"));
        }
        // Portable metadata MUST already exist. Local setup is the sole automatic
        // creation path; an existing local database is still validated first.
        let metadata = match Metadata::read(dir) {
            Ok(m) => m,
            Err(e) if mode == Mode::Portable => return Err(e),
            Err(_) => {
                if dir.join(location::METADATA).exists() {
                    return Err(failure("INVALID_DATA_METADATA"));
                }
                let m = Metadata::fresh();
                location::atomic_json(&dir.join(location::METADATA), &m)?;
                m
            }
        };
        let (lock, volume_protected) =
            acquire_lock(dir, &self.locations.app_data, &metadata.data_id)?;
        if mode == Mode::Local && !path.exists() && self.locations.local_override.is_none() {
            Database::session(path.clone(), false).initialize()?;
        }
        let mut session = Session {
            db: Database::session(path.clone(), false),
            metadata,
            mode: mode.clone(),
            _lock: lock,
            volume_protected,
            identity: None,
            fault: None,
            read_only: !volume_protected || !validate::writable(&path),
            schema: None,
            key: uuid::Uuid::new_v4().to_string(),
            last_backup: None,
        };
        let checked = if dir.join(".restore-pending.json").exists() {
            Err(failure("RESTORE_INTERRUPTED"))
        } else {
            let inspected = validate::inspect(&path);
            if inspected
                .as_ref()
                .is_err_and(|e| e.code == "DATABASE_RECOVERY_REQUIRED")
                && !session.read_only
            {
                self.recover_journal(&mut session)
                    .and_then(|_| validate::inspect(&path))
            } else {
                inspected
            }
        };
        match checked {
            Ok(version) => {
                session.schema = Some(version);
                if version < crate::database::migrations::SUPPORTED_SCHEMA_VERSION {
                    if session.read_only {
                        return Err(failure("UPGRADE_REQUIRES_WRITE"));
                    }
                    let b = backup::create(&path, &session.metadata.data_id, "before-upgrade")?;
                    session.last_backup = Some(b.filename);
                    session.db.initialize()?;
                    session.schema = Some(validate::inspect(&path)?);
                }
                session.db = Database::session(path.clone(), session.read_only);
                // Transition a validated writable WAL database to rollback mode. SQLite
                // checkpoints committed WAL frames as part of this mode transition.
                drop(session.db.connect()?);
                session.identity = Some(
                    same_file::Handle::from_path(&path)
                        .map_err(|_| failure("PORTABLE_STORAGE_REMOVED"))?,
                );
                if !session.read_only {
                    match backup::create(&path, &session.metadata.data_id, "open") {
                        Ok(b) => session.last_backup = Some(b.filename),
                        Err(e) => session.fault = Some(e.code),
                    }
                }
            }
            Err(e)
                if [
                    "DATABASE_DAMAGED",
                    "RESTORE_INTERRUPTED",
                    "PORTABLE_DATABASE_NOT_FOUND",
                    "DATABASE_RECOVERY_REQUIRED",
                ]
                .contains(&e.code.as_str()) =>
            {
                session.fault = Some(e.code)
            }
            Err(e) => return Err(e),
        }
        let preferred = if mode == Mode::Portable {
            Some(session.metadata.data_id.clone())
        } else {
            self.locations.settings.preferred_id.clone()
        };
        self.locations.remember(mode.clone(), preferred)?;
        self.event(match mode { Mode::Local => "MODE_LOCAL", Mode::Portable => "MODE_PORTABLE" });
        self.event("DATABASE_OPENED");
        if let Some(version) = session.schema {
            self.event(&format!("SCHEMA_{version}"));
        }
        self.event(if session.read_only {
            "READ_ONLY"
        } else {
            "READABLE_AND_WRITABLE"
        });
        if session.last_backup.is_some() {
            self.event("BACKUP_SUCCESSFUL");
        }
        if let Some(code) = &session.fault {
            self.event(code);
        }
        self.last_error = None;
        self.session = Some(session);
        Ok(())
    }
    fn recover_journal(&mut self, s: &mut Session) -> AppResult<()> {
        // Only an inactive crashed database is copied, under the process lock. Never
        // use raw copies as live backups. Recover and validate a private snapshot
        // before permitting SQLite to recover the selected original.
        let temp = tempfile::tempdir_in(&self.locations.app_data)
            .map_err(|_| failure("DATABASE_RECOVERY_REQUIRED"))?;
        let copy = temp.path().join(DATABASE);
        fs::copy(s.db.path(), &copy).map_err(|_| failure("DATABASE_RECOVERY_REQUIRED"))?;
        for suffix in ["-journal", "-wal", "-shm"] {
            let from = PathBuf::from(format!("{}{suffix}", s.db.path().display()));
            if from.exists() {
                fs::copy(from, PathBuf::from(format!("{}{suffix}", copy.display())))
                    .map_err(|_| failure("DATABASE_RECOVERY_REQUIRED"))?;
            }
        }
        let c = rusqlite::Connection::open_with_flags(
            &copy,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .map_err(|_| failure("DATABASE_RECOVERY_REQUIRED"))?;
        validate::inspect_connection(&c)?;
        drop(c);
        // Keep a verified recovered snapshot on the original volume before recovery.
        let recovered = backup::create(&copy, &s.metadata.data_id, "crash-recovery")?;
        let target = s.db.path().parent().unwrap().join("Backups");
        fs::create_dir_all(&target).map_err(|_| failure("BACKUP_FAILED"))?;
        let source = temp.path().join("Backups").join(&recovered.filename);
        backup::snapshot(&source, &target.join(&recovered.filename))?;
        location::atomic_json(
            &target
                .join(&recovered.filename)
                .with_extension("verified.json"),
            &recovered,
        )?;
        let c = rusqlite::Connection::open_with_flags(
            s.db.path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .map_err(|_| failure("DATABASE_RECOVERY_REQUIRED"))?;
        validate::inspect_connection(&c)?;
        drop(c);
        self.event("JOURNAL_RECOVERED");
        Ok(())
    }
    fn preflight(s: &mut Session, write: bool) -> AppResult<()> {
        if let Some(code) = &s.fault {
            return Err(failure(code));
        }
        let dir = s.db.path().parent().unwrap();
        if Metadata::read(dir).map_or(true, |m| m.data_id != s.metadata.data_id)
            || same_file::Handle::from_path(s.db.path()).ok().as_ref() != s.identity.as_ref()
        {
            return Err(failure("PORTABLE_STORAGE_REMOVED"));
        }
        if !s.read_only && !validate::writable(s.db.path()) {
            s.read_only = true;
            s.db = Database::session(s.db.path().to_owned(), true);
        }
        if write && s.read_only {
            return Err(failure("DATABASE_READ_ONLY"));
        }
        Ok(())
    }
    pub fn with_database<T>(
        &mut self,
        write: bool,
        destructive: bool,
        operation: impl FnOnce(&Database) -> AppResult<T>,
    ) -> AppResult<T> {
        let result = (|| {
            let s = self
                .session
                .as_mut()
                .ok_or_else(|| failure("SESSION_CLOSED"))?;
            Self::preflight(s, write)?;
            if destructive {
                let b = backup::create(
                    s.db.path(),
                    &s.metadata.data_id,
                    "before-destructive-change",
                )?;
                s.last_backup = Some(b.filename);
            }
            let result = operation(&s.db);
            // Re-check even after commit: a removed/remounted volume must never yield
            // a false Saved acknowledgement. Commit outcome can be uncertain on I/O.
            if write {
                Self::preflight(s, true)?;
            }
            result
        })();
        if let Err(e) = &result {
            if [
                "PORTABLE_STORAGE_REMOVED",
                "DATABASE_UNAVAILABLE",
                "DATABASE_WRITE_FAILED",
                "DATABASE_DAMAGED",
                "BACKUP_FAILED",
            ]
            .contains(&e.code.as_str())
            {
                if let Some(s) = &mut self.session {
                    s.fault = Some(e.code.clone());
                }
            }
            self.event(&e.code);
        }
        result
    }
    pub fn manual_backup(&mut self) -> AppResult<backup::Record> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| failure("SESSION_CLOSED"))?;
        if s.fault.as_deref() == Some("BACKUP_FAILED") {
            s.fault = None;
        }
        Self::preflight(s, true)?;
        let result = backup::create(s.db.path(), &s.metadata.data_id, "manual");
        match &result {
            Ok(b) => {
                s.last_backup = Some(b.filename.clone());
                self.event("BACKUP_SUCCESSFUL");
            }
            Err(e) => {
                s.fault = Some(e.code.clone());
                self.event(&e.code);
            }
        }
        result
    }
    pub fn backups(&self) -> AppResult<Vec<backup::Record>> {
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| failure("SESSION_CLOSED"))?;
        Ok(backup::list(
            &s.db.path().parent().unwrap().join("Backups"),
            &s.metadata.data_id,
        ))
    }
    pub fn close(&mut self, without_backup: bool) -> AppResult<Status> {
        if let Some(s) = &mut self.session {
            if !without_backup && !s.read_only {
                Self::preflight(s, true)?;
                let b = backup::create(s.db.path(), &s.metadata.data_id, "close")?;
                s.last_backup = Some(b.filename);
            }
        }
        // All service connections are scoped to the manager mutex; no active
        // transaction can outlive this call. Dropping releases identity and OS lock.
        self.session = None;
        self.last_error = None;
        self.event("DATABASE_CLOSED");
        Ok(self.status())
    }
    pub fn initialize_portable(
        &mut self,
        dir: PathBuf,
        source: Option<PathBuf>,
    ) -> AppResult<Status> {
        if self.active() {
            return Err(failure("SESSION_OPEN"));
        }
        location::validate_path(&dir)?;
        if let Some(source) = &source { location::validate_path(source)?; }
        fs::create_dir(&dir).map_err(|_| failure("INITIALIZATION_TARGET_EXISTS"))?;
        let path = dir.join(DATABASE);
        if let Some(source) = source {
            validate::inspect(&source)?;
            backup::snapshot(&source, &path)?;
        } else {
            Database::session(path, false).initialize()?;
        }
        let metadata = Metadata::fresh();
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(LOCK))
            .map_err(|_| failure("DATABASE_READ_ONLY"))?;
        location::atomic_json(&dir.join(location::METADATA), &metadata)?;
        self.open(Mode::Portable, Some(dir))
    }
    pub fn restore(&mut self, name: &str) -> AppResult<Status> {
        let result = self.restore_inner(name);
        if let Err(e) = &result {
            if let Some(s) = &mut self.session {
                if s.db
                    .path()
                    .parent()
                    .unwrap()
                    .join(".restore-pending.json")
                    .exists()
                {
                    s.fault = Some("RESTORE_INTERRUPTED".into());
                }
            }
            self.event(&e.code);
        }
        result?;
        self.event("RESTORE_SUCCESSFUL");
        Ok(self.status())
    }
    fn restore_inner(&mut self, name: &str) -> AppResult<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| failure("SESSION_CLOSED"))?;
        if !s.volume_protected {
            return Err(failure("DATABASE_READ_ONLY"));
        }
        let path = s.db.path().to_owned();
        let dir = path.parent().unwrap();
        if Metadata::read(dir)?.data_id != s.metadata.data_id {
            return Err(failure("PORTABLE_STORAGE_REMOVED"));
        }
        let candidate = backup::selected(&dir.join("Backups"), &s.metadata.data_id, name)?;
        // Restore also supports damaged/missing originals, but never writes newer data.
        let current = validate::inspect(&path);
        if path.exists() && !validate::writable(&path) {
            return Err(failure("DATABASE_READ_ONLY"));
        }
        if current
            .as_ref()
            .is_err_and(|e| e.code == "DATABASE_VERSION_TOO_NEW")
        {
            return Err(failure("DATABASE_VERSION_TOO_NEW"));
        }
        let token = uuid::Uuid::new_v4();
        let stage = dir.join(format!(".restore-{token}.sqlite"));
        backup::snapshot(&candidate, &stage)?;
        if current.is_ok() {
            backup::create(&path, &s.metadata.data_id, "before-restore")?;
        }
        let preserved = dir.join(format!("preserved-{token}.sqlite"));
        let pending = dir.join(".restore-pending.json");
        location::atomic_json(
            &pending,
            &serde_json::json!({"stage":stage.file_name(),"preserved":preserved.file_name()}),
        )
        .map_err(|_| failure("RESTORE_FAILED"))?;
        s.identity = None;
        for suffix in ["", "-journal", "-wal", "-shm"] {
            let original = PathBuf::from(format!("{}{suffix}", path.display()));
            let saved = PathBuf::from(format!("{}{suffix}", preserved.display()));
            if original.exists() {
                fs::rename(original, saved).map_err(|_| failure("RESTORE_FAILED"))?;
            }
        }
        location::sync_parent(&path).map_err(|_| failure("RESTORE_FAILED"))?;
        fs::rename(&stage, &path).map_err(|_| failure("RESTORE_FAILED"))?;
        location::sync_parent(&path).map_err(|_| failure("RESTORE_FAILED"))?;
        let version = validate::inspect(&path)?;
        if version < crate::database::migrations::SUPPORTED_SCHEMA_VERSION {
            backup::create(&path, &s.metadata.data_id, "before-upgrade")?;
            Database::session(path.clone(), false).initialize()?;
        }
        s.schema = Some(validate::inspect(&path)?);
        s.identity =
            Some(same_file::Handle::from_path(&path).map_err(|_| failure("RESTORE_FAILED"))?);
        s.db = Database::session(path, false);
        s.read_only = false;
        s.fault = None;
        s.key = uuid::Uuid::new_v4().to_string();
        fs::remove_file(pending).map_err(|_| failure("RESTORE_FAILED"))?;
        location::sync_parent(s.db.path()).map_err(|_| failure("RESTORE_FAILED"))?;
        Ok(())
    }
    pub fn diagnostics(&mut self) -> String {
        let status = self.status();
        serde_json::to_string_pretty(&serde_json::json!({"application":"Dental Scheduler Mini","networkRequired":false,"lastErrorCode":status.error_code,"appVersion":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"schemaVersion":status.schema_version,"mode":status.mode,"reachable":status.active && status.access!="UNAVAILABLE","writable":status.access=="READABLE_AND_WRITABLE","integrity":if status.error_code.as_deref()==Some("DATABASE_DAMAGED"){"failed"}else if status.schema_version.is_some(){"passed at open"}else{"not checked"},"lastBackup":status.last_backup,"recentCodes":self.recent})).unwrap_or_default()
    }
}
fn acquire_lock(dir: &Path, local: &Path, id: &str) -> AppResult<(fs::File, bool)> {
    let path = dir.join(LOCK);
    if path.is_symlink() {
        return Err(failure("DATABASE_ALREADY_IN_USE"));
    }
    let volume = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .or_else(|_| fs::File::open(&path));
    let (file, volume_protected) = match volume {
        Ok(file) => (file, true),
        Err(_) => {
            let locks = local.join("read-only-locks");
            fs::create_dir_all(&locks).map_err(|_| failure("DATABASE_ALREADY_IN_USE"))?;
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(locks.join(id))
                .map_err(|_| failure("DATABASE_ALREADY_IN_USE"))?;
            // A local lock cannot coordinate another computer. This fallback
            // therefore NEVER grants editing, even if a later write probe works.
            (file, false)
        }
    };
    file.try_lock()
        .map_err(|_| failure("DATABASE_ALREADY_IN_USE"))?;
    Ok((file, volume_protected))
}
