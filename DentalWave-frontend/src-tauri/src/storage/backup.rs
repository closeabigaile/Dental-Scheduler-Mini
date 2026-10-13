use super::{
    failure,
    location::{atomic_json, sync_parent},
    validate,
};
use crate::error::AppResult;
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

pub const RETENTION: usize = 20;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub filename: String,
    pub data_id: String,
    pub schema_version: i64,
    pub created_at: String,
    pub reason: String,
}
fn marker(path: &Path) -> PathBuf {
    path.with_extension("verified.json")
}
pub fn timestamp() -> String {
    let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
    now.format("%Y-%m-%d-%H%M%S-%3f").to_string()
}
/// SQLite copies a consistent snapshot, including committed WAL contents. The
/// destination is new, never an existing backup. All SQLite handles close here.
pub fn snapshot(source: &Path, dest: &Path) -> AppResult<i64> {
    let result = (|| {
        let source = validate::read_only(source)?;
        validate::inspect_connection(&source)?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)
            .map_err(|_| failure("BACKUP_FAILED"))?;
        let mut target = Connection::open_with_flags(dest, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(|_| failure("BACKUP_FAILED"))?;
        validate::configure(&target, false)?;
        {
            let job = Backup::new(&source, &mut target).map_err(|_| failure("BACKUP_FAILED"))?;
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                match job.step(256).map_err(|_| failure("BACKUP_FAILED"))? {
                    StepResult::Done => break,
                    _ => {
                        if Instant::now() > deadline {
                            return Err(failure("BACKUP_FAILED"));
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }
        // A source's persistent journal mode may be copied too.
        validate::configure(&target, false)?;
        let version = validate::inspect_connection(&target)?;
        target.close().map_err(|_| failure("BACKUP_FAILED"))?;
        fs::OpenOptions::new()
            .write(true)
            .open(dest)
            .and_then(|f| f.sync_all())
            .map_err(|_| failure("BACKUP_FAILED"))?;
        sync_parent(dest).map_err(|_| failure("BACKUP_FAILED"))?;
        Ok(version)
    })();
    // Incomplete artifacts are deliberately not eligible for restore/retention.
    result.map_err(|_| failure("BACKUP_FAILED"))
}
pub fn create(source: &Path, id: &str, reason: &str) -> AppResult<Record> {
    let dir = source
        .parent()
        .ok_or_else(|| failure("BACKUP_FAILED"))?
        .join("Backups");
    if dir.is_symlink() {
        return Err(failure("BACKUP_FAILED"));
    }
    fs::create_dir_all(&dir).map_err(|_| failure("BACKUP_FAILED"))?;
    let created_at = timestamp();
    let filename = format!("scheduler-{created_at}-{}.sqlite", uuid::Uuid::new_v4());
    let path = dir.join(&filename);
    let partial = path.with_extension("partial");
    let schema_version = snapshot(source, &partial)?;
    // The UUID and create_new reserve the destination without replacing anything.
    fs::hard_link(&partial, &path)
        .or_else(|_| {
            // FAT/exFAT lack hard links. Reserve the name before an atomic rename.
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            fs::rename(&partial, &path)
        })
        .map_err(|_| failure("BACKUP_FAILED"))?;
    if partial.exists() {
        fs::remove_file(&partial).map_err(|_| failure("BACKUP_FAILED"))?;
    }
    validate::inspect(&path).map_err(|_| failure("BACKUP_FAILED"))?;
    let record = Record {
        filename,
        data_id: id.into(),
        schema_version,
        created_at,
        reason: reason.into(),
    };
    atomic_json(&marker(&path), &record).map_err(|_| failure("BACKUP_FAILED"))?;
    sync_parent(&path).map_err(|_| failure("BACKUP_FAILED"))?;
    // A retention failure does not invalidate the new verified backup.
    let _ = retain(&dir, id, RETENTION);
    Ok(record)
}
pub fn list(dir: &Path, id: &str) -> Vec<Record> {
    let mut records = vec![];
    if dir.is_symlink() {
        return records;
    }
    for entry in fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.to_string_lossy().ends_with(".verified.json") || path.is_symlink() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        let Ok(record) = serde_json::from_slice::<Record>(&bytes) else {
            continue;
        };
        if record.data_id == id
            && safe_name(&record.filename)
            && marker(&dir.join(&record.filename)) == path
            && validate::inspect(&dir.join(&record.filename))
                .is_ok_and(|v| v == record.schema_version)
        {
            records.push(record);
        }
    }
    records.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then(b.filename.cmp(&a.filename))
    });
    records
}
fn safe_name(name: &str) -> bool {
    name.starts_with("scheduler-")
        && name.ends_with(".sqlite")
        && !name.contains(['/', '\\', ':'])
        && !name.contains("..")
}
pub fn selected(dir: &Path, id: &str, name: &str) -> AppResult<PathBuf> {
    if !safe_name(name) || !list(dir, id).iter().any(|r| r.filename == name) {
        return Err(failure("INVALID_BACKUP"));
    }
    Ok(dir.join(name))
}
pub fn retain(dir: &Path, id: &str, keep: usize) -> AppResult<()> {
    let records = list(dir, id); // Reverify before considering removal; never touch unknown files.
    for record in records.into_iter().skip(keep.max(1)) {
        let path = dir.join(record.filename);
        fs::remove_file(&path).map_err(|_| failure("BACKUP_RETENTION_FAILED"))?;
        fs::remove_file(marker(&path)).map_err(|_| failure("BACKUP_RETENTION_FAILED"))?;
    }
    Ok(())
}
