use super::failure;
use crate::{
    database::migrations::{SUPPORTED_SCHEMA_VERSION, VERSION_1, VERSION_2, VERSION_3},
    error::AppResult,
};
use rusqlite::{Connection, OpenFlags};
use std::{fs, path::Path, time::Duration};

pub fn read_only(path: &Path) -> AppResult<Connection> {
    if fs::symlink_metadata(path).map_or(true, |m| !m.is_file()) {
        return Err(failure("PORTABLE_DATABASE_NOT_FOUND"));
    }
    let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| failure("DATABASE_DAMAGED"))?;
    c.busy_timeout(Duration::from_secs(5))
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?;
    Ok(c)
}
pub fn inspect(path: &Path) -> AppResult<i64> {
    let c = read_only(path)?;
    inspect_connection(&c)
}
pub fn inspect_connection(c: &Connection) -> AppResult<i64> {
    let version: i64 = c
        .query_row(
            "SELECT schema_version FROM schema_metadata WHERE id=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| {
            if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ReadOnly) {
                failure("DATABASE_RECOVERY_REQUIRED")
            } else {
                failure("DATABASE_DAMAGED")
            }
        })?;
    if version > SUPPORTED_SCHEMA_VERSION {
        return Err(failure("DATABASE_VERSION_TOO_NEW"));
    }
    if version < 1 {
        return Err(failure("DATABASE_DAMAGED"));
    }
    let expected = Connection::open_in_memory().map_err(|_| failure("DATABASE_UNAVAILABLE"))?;
    expected
        .execute_batch(VERSION_1)
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?;
    if version >= 2 {
        expected
            .execute_batch(VERSION_2)
            .map_err(|_| failure("DATABASE_UNAVAILABLE"))?;
    }
    if version >= 3 {
        expected
            .execute_batch(VERSION_3)
            .map_err(|_| failure("DATABASE_UNAVAILABLE"))?;
    }
    let tables = expected
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|_| failure("DATABASE_DAMAGED"))?
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| failure("DATABASE_DAMAGED"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| failure("DATABASE_DAMAGED"))?;
    for table in tables {
        // Names originate from our compiled schema, never from user SQL.
        for pragma in ["table_info", "foreign_key_list"] {
            let sql = format!("PRAGMA {pragma}({table})");
            if signature(c, &sql)? != signature(&expected, &sql)? {
                return Err(failure("DATABASE_DAMAGED"));
            }
        }
    }
    let mut q = c
        .prepare("PRAGMA integrity_check")
        .map_err(|_| failure("DATABASE_DAMAGED"))?;
    let results = q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| failure("DATABASE_DAMAGED"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| failure("DATABASE_DAMAGED"))?;
    if results != ["ok"]
        || c.prepare("PRAGMA foreign_key_check")
            .map_err(|_| failure("DATABASE_DAMAGED"))?
            .exists([])
            .map_err(|_| failure("DATABASE_DAMAGED"))?
    {
        return Err(failure("DATABASE_DAMAGED"));
    }
    Ok(version)
}
fn signature(c: &Connection, sql: &str) -> AppResult<Vec<Vec<rusqlite::types::Value>>> {
    let mut q = c.prepare(sql).map_err(|_| failure("DATABASE_DAMAGED"))?;
    let n = q.column_count();
    let rows = q
        .query_map([], |r| (0..n).map(|i| r.get(i)).collect())
        .map_err(|_| failure("DATABASE_DAMAGED"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| failure("DATABASE_DAMAGED"))?;
    Ok(rows)
}
pub fn writable(path: &Path) -> bool {
    if fs::metadata(path).map_or(true, |m| m.permissions().readonly()) {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if fs::metadata(parent).map_or(true, |m| m.permissions().readonly()) {
        return false;
    }
    let probe = parent.join(format!(".write-check-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&probe)?;
        f.write_all(b"check")?;
        f.sync_all()?;
        fs::OpenOptions::new().write(true).open(path)?;
        Ok::<_, std::io::Error>(())
    })();
    let removed = fs::remove_file(&probe);
    result.is_ok() && removed.is_ok()
}
pub fn configure(c: &Connection, read_only: bool) -> AppResult<()> {
    c.busy_timeout(Duration::from_secs(5))
        .map_err(crate::error::AppError::from_read_error)?;
    c.pragma_update(None, "foreign_keys", "ON")
        .map_err(crate::error::AppError::from_read_error)?;
    if read_only {
        c.pragma_update(None, "query_only", "ON")
            .map_err(crate::error::AppError::from_read_error)?;
    } else {
        let mode: String = c
            .query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))
            .map_err(crate::error::AppError::from_write_error)?;
        if mode != "delete" {
            return Err(failure("STORAGE_CONFIGURATION_FAILED"));
        }
        c.pragma_update(None, "synchronous", "EXTRA")
            .map_err(crate::error::AppError::from_write_error)?;
        #[cfg(target_os = "macos")]
        c.pragma_update(None, "fullfsync", true)
            .map_err(crate::error::AppError::from_write_error)?;
    }
    Ok(())
}
