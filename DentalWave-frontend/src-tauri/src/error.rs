use rusqlite::{ffi, Error as SqliteError};
use serde::Serialize;
use std::fmt::{Display, Formatter};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
  pub code: String,
  pub message: String,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub conflicts: Vec<crate::domain::Conflict>,
}

impl AppError {
  pub fn database_unavailable() -> Self {
    Self::new("DATABASE_UNAVAILABLE", "The desktop database is unavailable.")
  }

  pub fn initialization_failed() -> Self {
    Self::new(
      "DATABASE_INITIALIZATION_FAILED",
      "The desktop database could not be initialized.",
    )
  }

  pub fn invalid_input(message: impl Into<String>) -> Self {
    Self::new("INVALID_INPUT", message)
  }

  pub fn not_found(entity: &str) -> Self {
    Self::new("RECORD_NOT_FOUND", format!("{entity} was not found."))
  }

  pub fn conflict(message: impl Into<String>) -> Self {
    Self::new("RECORD_CONFLICT", message)
  }

  pub fn write_failed() -> Self {
    Self::new("DATABASE_WRITE_FAILED", "The database change could not be saved.")
  }

  pub fn incompatible_schema(found: i64, supported: i64) -> Self {
    Self::new(
      "INCOMPATIBLE_SCHEMA_VERSION",
      format!(
        "This database uses schema version {found}, but this application supports up to version {supported}."
      ),
    )
  }

  pub fn integrity_failure() -> Self {
    Self::new(
      "INTEGRITY_FAILURE",
      "The desktop database did not pass its integrity check.",
    )
  }

  pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
    Self {
      code: code.into(),
      message: message.into(),
      conflicts: Vec::new(),
    }
  }

  pub fn from_read_error(error: SqliteError) -> Self {
    if let Some(code)=storage_error(&error){return crate::storage::failure(code);}
    Self::database_unavailable()
  }

  pub fn from_write_error(error: SqliteError) -> Self {
    if let Some(code)=storage_error(&error){return crate::storage::failure(code);}
    match &error {
      SqliteError::SqliteFailure(details, _)
        if details.extended_code == ffi::SQLITE_CONSTRAINT_UNIQUE
          || details.extended_code == ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
      {
        Self::conflict("A record with the same identifying information already exists.")
      }
      SqliteError::SqliteFailure(details, _)
        if details.code == rusqlite::ErrorCode::ConstraintViolation =>
      {
        Self::conflict("This record is still referenced by other scheduler data.")
      }
      _ => Self::write_failed(),
    }
  }
}

impl Display for AppError {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "{}: {}", self.code, self.message)
  }
}

impl std::error::Error for AppError {}

fn storage_error(error:&SqliteError)->Option<&'static str> {
  use rusqlite::ErrorCode::*;
  match error.sqlite_error_code()? {
    ReadOnly=>Some("DATABASE_READ_ONLY"),
    SystemIoFailure|DiskFull=>Some("DATABASE_WRITE_FAILED"),
    DatabaseCorrupt|NotADatabase=>Some("DATABASE_DAMAGED"),
    CannotOpen=>Some("PORTABLE_STORAGE_REMOVED"),
    DatabaseBusy|DatabaseLocked=>Some("DATABASE_ALREADY_IN_USE"),
    _=>None,
  }
}
