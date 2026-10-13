pub(crate) mod migrations;

use crate::error::{AppError, AppResult};
use crate::models::DatabaseInfo;
use migrations::{SUPPORTED_SCHEMA_VERSION, VERSION_1, VERSION_2, VERSION_3};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Database {
  path: PathBuf,
  session_mode: bool,
  read_only: bool,
}

impl Database {
  pub fn new(path: PathBuf) -> Self {
    Self { path, session_mode:false, read_only:false }
  }

  /// Explicit development-only entry point for an already migrated COPY.
  /// Validation happens read-only before normal application connections exist.
  #[cfg(debug_assertions)]
  pub fn migrated_copy(path: PathBuf) -> AppResult<Self> {
    let c = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
      .map_err(|_| AppError::database_unavailable())?;
    let version = Self::read_schema_version(&c).map_err(|_| AppError::initialization_failed())?;
    if version != SUPPORTED_SCHEMA_VERSION {
      return Err(AppError::incompatible_schema(version, SUPPORTED_SCHEMA_VERSION));
    }
    let migrated: i64 = c.query_row("SELECT COUNT(*) FROM migration_history WHERE source_type='H2' AND tool_version=1", [], |r| r.get(0))
      .map_err(|_| AppError::new("NOT_A_MIGRATED_COPY", "Select a verified migration output copy."))?;
    if migrated != 1 { return Err(AppError::new("NOT_A_MIGRATED_COPY", "Select a verified migration output copy.")); }
    let db = Self::new(path);
    db.integrity_check_with_connection(&c)?;
    if c.prepare("PRAGMA foreign_key_check").map_err(AppError::from_read_error)?
      .exists([]).map_err(AppError::from_read_error)? {
      return Err(AppError::integrity_failure());
    }
    Ok(db)
  }

  pub fn session(path:PathBuf,read_only:bool)->Self {Self{path,session_mode:true,read_only}}

  pub fn path(&self) -> &std::path::Path {
    &self.path
  }

  pub fn initialize(&self) -> AppResult<()> {
    let parent = self.path.parent().ok_or_else(AppError::initialization_failed)?;
    fs::create_dir_all(parent).map_err(|_error| {
      eprintln!("Could not create desktop database directory");
      AppError::initialization_failed()
    })?;

    let mut connection = self.connect_for_initialization()?;
    let metadata_exists: i64 = connection
      .query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_metadata')",
        [],
        |row| row.get(0),
      )
      .map_err(|_error| {
        eprintln!("Could not inspect SQLite schema metadata");
        AppError::initialization_failed()
      })?;

    if metadata_exists == 0 {
      let transaction = connection.transaction().map_err(|_error| {
        eprintln!("Could not begin SQLite schema transaction");
        AppError::initialization_failed()
      })?;
      transaction.execute_batch(VERSION_1).map_err(|_error| {
        eprintln!("Could not apply SQLite schema version 1");
        AppError::initialization_failed()
      })?;
      transaction.commit().map_err(|_error| {
        eprintln!("Could not commit SQLite schema version 1");
        AppError::initialization_failed()
      })?;
    }

    let version = Self::read_schema_version(&connection).map_err(|_error| {
      eprintln!("Could not read SQLite schema version");
      AppError::initialization_failed()
    })?;
    if version > SUPPORTED_SCHEMA_VERSION {
      return Err(AppError::incompatible_schema(
        version,
        SUPPORTED_SCHEMA_VERSION,
      ));
    }
    if version == 1 {
      let transaction = connection.transaction().map_err(AppError::from_write_error)?;
      transaction.execute_batch(VERSION_2).map_err(AppError::from_write_error)?;
      transaction.commit().map_err(AppError::from_write_error)?;
    } else if version != 2 && version != SUPPORTED_SCHEMA_VERSION {
      return Err(AppError::initialization_failed());
    }
    if version <= 2 {
      let transaction = connection.transaction().map_err(AppError::from_write_error)?;
      transaction.execute_batch(VERSION_3).map_err(AppError::from_write_error)?;
      transaction.commit().map_err(AppError::from_write_error)?;
    }

    self.integrity_check_with_connection(&connection)?;
    Ok(())
  }

  pub fn connect(&self) -> AppResult<Connection> {
    let connection = Connection::open_with_flags(&self.path, if self.read_only {OpenFlags::SQLITE_OPEN_READ_ONLY}else{OpenFlags::SQLITE_OPEN_READ_WRITE}).map_err(|_error| {
      eprintln!("Could not open SQLite database");
      AppError::database_unavailable()
    })?;
    let version = Self::read_schema_version(&connection).map_err(AppError::from_read_error)?;
    if version > SUPPORTED_SCHEMA_VERSION {
      return Err(AppError::incompatible_schema(
        version,
        SUPPORTED_SCHEMA_VERSION,
      ));
    }
    self.configure_connection(&connection)?;
    Ok(connection)
  }

  pub fn info(&self) -> AppResult<DatabaseInfo> {
    let connection = self.connect()?;
    let schema_version = Self::read_schema_version(&connection).map_err(AppError::from_read_error)?;
    let integrity_status = self.integrity_check_with_connection(&connection)?;
    Ok(DatabaseInfo {
      path: self.path.to_string_lossy().into_owned(),
      schema_version,
      integrity_status,
    })
  }

  pub fn integrity_check(&self) -> AppResult<String> {
    let connection = self.connect()?;
    self.integrity_check_with_connection(&connection)
  }

  fn connect_for_initialization(&self) -> AppResult<Connection> {
    let connection = Connection::open(&self.path).map_err(|_error| {
      eprintln!("Could not open SQLite database during initialization");
      AppError::initialization_failed()
    })?;
    self.configure_connection(&connection).map_err(|_error| {
      eprintln!("Could not configure SQLite during initialization");
      AppError::initialization_failed()
    })?;
    Ok(connection)
  }

  fn configure_connection(&self, connection: &Connection) -> AppResult<()> {
    if self.session_mode {return crate::storage::validate::configure(connection,self.read_only);}
    connection
      .busy_timeout(Duration::from_secs(5))
      .map_err(AppError::from_read_error)?;
    connection
      .pragma_update(None, "foreign_keys", "ON")
      .map_err(AppError::from_read_error)?;
    connection
      .pragma_update(None, "journal_mode", "WAL")
      .map_err(AppError::from_read_error)?;
    connection
      .pragma_update(None, "synchronous", "FULL")
      .map_err(AppError::from_read_error)?;
    Ok(())
  }

  fn read_schema_version(connection: &Connection) -> rusqlite::Result<i64> {
    connection
      .query_row(
        "SELECT schema_version FROM schema_metadata WHERE id = 1",
        [],
        |row| row.get(0),
      )
      .optional()?
      .ok_or(rusqlite::Error::QueryReturnedNoRows)
  }

  fn integrity_check_with_connection(&self, connection: &Connection) -> AppResult<String> {
    let status: String = connection
      .query_row("PRAGMA integrity_check", [], |row| row.get(0))
      .map_err(AppError::from_read_error)?;
    if status != "ok" {
      eprintln!("SQLite integrity check failed");
      return Err(AppError::integrity_failure());
    }
    Ok(status)
  }
}

#[derive(Debug)]
pub struct AppState {
  pub storage: std::sync::Mutex<crate::storage::StorageManager>,
}

impl AppState {
  pub fn with_database<T>(&self,write:bool,destructive:bool,f:impl FnOnce(&Database)->AppResult<T>)->AppResult<T> {
    self.storage.lock().map_err(|_|crate::storage::failure("DATABASE_UNAVAILABLE"))?.with_database(write,destructive,f)
  }
}
