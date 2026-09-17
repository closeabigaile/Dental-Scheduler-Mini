use crate::error::{AppError, AppResult};
use crate::models::{Office, OfficeInput};
use rusqlite::{params, Connection, OptionalExtension};

pub struct OfficeRepository;

impl OfficeRepository {
  pub fn list(connection: &Connection) -> AppResult<Vec<Office>> {
    let mut statement = connection
      .prepare(
        "SELECT id, name, address, phone_number FROM offices ORDER BY name COLLATE NOCASE",
      )
      .map_err(AppError::from_read_error)?;
    let offices = statement
      .query_map([], Self::map_row)
      .map_err(AppError::from_read_error)?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(AppError::from_read_error)?;
    Ok(offices)
  }

  pub fn get(connection: &Connection, id: i64) -> AppResult<Office> {
    connection
      .query_row(
        "SELECT id, name, address, phone_number FROM offices WHERE id = ?1",
        [id],
        Self::map_row,
      )
      .optional()
      .map_err(AppError::from_read_error)?
      .ok_or_else(|| AppError::not_found("Office"))
  }

  pub fn create(connection: &Connection, input: &OfficeInput) -> AppResult<Office> {
    connection
      .execute(
        "INSERT INTO offices (name, address, phone_number) VALUES (?1, ?2, ?3)",
        params![input.name, input.address, input.phone_number],
      )
      .map_err(AppError::from_write_error)?;
    Self::get(connection, connection.last_insert_rowid())
  }

  pub fn update(connection: &Connection, id: i64, input: &OfficeInput) -> AppResult<Office> {
    let changed = connection
      .execute(
        "UPDATE offices
         SET name = ?1, address = ?2, phone_number = ?3, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![input.name, input.address, input.phone_number, id],
      )
      .map_err(AppError::from_write_error)?;
    if changed == 0 {
      return Err(AppError::not_found("Office"));
    }
    Self::get(connection, id)
  }

  pub fn delete(connection: &Connection, id: i64) -> AppResult<()> {
    let changed = connection
      .execute("DELETE FROM offices WHERE id = ?1", [id])
      .map_err(AppError::from_write_error)?;
    if changed == 0 {
      return Err(AppError::not_found("Office"));
    }
    Ok(())
  }

  pub fn exists(connection: &Connection, id: i64) -> AppResult<bool> {
    connection
      .query_row(
        "SELECT EXISTS(SELECT 1 FROM offices WHERE id = ?1)",
        [id],
        |row| row.get(0),
      )
      .map_err(AppError::from_read_error)
  }

  fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Office> {
    Ok(Office {
      id: row.get(0)?,
      name: row.get(1)?,
      address: row.get(2)?,
      phone_number: row.get(3)?,
    })
  }
}
