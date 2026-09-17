mod migrations;

use crate::error::{AppError, AppResult};
use crate::models::DatabaseInfo;
use migrations::{SUPPORTED_SCHEMA_VERSION, VERSION_1};
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Database {
  path: PathBuf,
}

impl Database {
  pub fn new(path: PathBuf) -> Self {
    Self { path }
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn initialize(&self) -> AppResult<()> {
    let parent = self.path.parent().ok_or_else(AppError::initialization_failed)?;
    fs::create_dir_all(parent).map_err(|error| {
      eprintln!("Could not create desktop database directory: {error}");
      AppError::initialization_failed()
    })?;

    let mut connection = self.connect_for_initialization()?;
    let metadata_exists: i64 = connection
      .query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_metadata')",
        [],
        |row| row.get(0),
      )
      .map_err(|error| {
        eprintln!("Could not inspect SQLite schema metadata: {error}");
        AppError::initialization_failed()
      })?;

    if metadata_exists == 0 {
      let transaction = connection.transaction().map_err(|error| {
        eprintln!("Could not begin SQLite schema transaction: {error}");
        AppError::initialization_failed()
      })?;
      transaction.execute_batch(VERSION_1).map_err(|error| {
        eprintln!("Could not apply SQLite schema version 1: {error}");
        AppError::initialization_failed()
      })?;
      transaction.commit().map_err(|error| {
        eprintln!("Could not commit SQLite schema version 1: {error}");
        AppError::initialization_failed()
      })?;
    }

    let version = Self::read_schema_version(&connection).map_err(|error| {
      eprintln!("Could not read SQLite schema version: {error}");
      AppError::initialization_failed()
    })?;
    if version > SUPPORTED_SCHEMA_VERSION {
      return Err(AppError::incompatible_schema(
        version,
        SUPPORTED_SCHEMA_VERSION,
      ));
    }
    if version < SUPPORTED_SCHEMA_VERSION {
      return Err(AppError::new(
        "DATABASE_INITIALIZATION_FAILED",
        format!(
          "Database schema version {version} cannot be upgraded by this application."
        ),
      ));
    }

    self.integrity_check_with_connection(&connection)?;
    Ok(())
  }

  pub fn connect(&self) -> AppResult<Connection> {
    let connection = Connection::open(&self.path).map_err(|error| {
      eprintln!("Could not open SQLite database: {error}");
      AppError::database_unavailable()
    })?;
    self.configure_connection(&connection)?;
    let version = Self::read_schema_version(&connection).map_err(AppError::from_read_error)?;
    if version > SUPPORTED_SCHEMA_VERSION {
      return Err(AppError::incompatible_schema(
        version,
        SUPPORTED_SCHEMA_VERSION,
      ));
    }
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
    let connection = Connection::open(&self.path).map_err(|error| {
      eprintln!("Could not open SQLite database during initialization: {error}");
      AppError::initialization_failed()
    })?;
    self.configure_connection(&connection).map_err(|error| {
      eprintln!("Could not configure SQLite during initialization: {error}");
      AppError::initialization_failed()
    })?;
    Ok(connection)
  }

  fn configure_connection(&self, connection: &Connection) -> AppResult<()> {
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
      eprintln!("SQLite integrity check failed: {status}");
      return Err(AppError::integrity_failure());
    }
    Ok(status)
  }
}

#[derive(Debug)]
pub struct AppState {
  pub database: Database,
}
