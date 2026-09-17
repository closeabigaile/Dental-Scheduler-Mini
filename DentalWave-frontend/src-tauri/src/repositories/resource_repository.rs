use crate::error::{AppError, AppResult};
use crate::models::{Office, ResourceInput, ResourceKind, SchedulingResource};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeSet;

pub struct ResourceRepository;

impl ResourceRepository {
  pub fn list(connection: &Connection, kind: ResourceKind) -> AppResult<Vec<SchedulingResource>> {
    let sql = format!(
      "SELECT r.id, r.first_name, r.last_name, r.display_name, r.active,
              r.normal_workdays, r.color, r.notes,
              o.id, o.name, o.address, o.phone_number
       FROM {} r
       LEFT JOIN offices o ON o.id = r.default_office_id
       ORDER BY r.display_name COLLATE NOCASE",
      Self::table(kind)
    );
    let mut statement = connection.prepare(&sql).map_err(AppError::from_read_error)?;
    let mut resources = statement
      .query_map([], |row| Self::map_row(row, kind))
      .map_err(AppError::from_read_error)?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(AppError::from_read_error)?;
    drop(statement);

    for resource in &mut resources {
      resource.offices = Self::list_offices(connection, kind, resource.id)?;
    }
    Ok(resources)
  }

  pub fn get(
    connection: &Connection,
    kind: ResourceKind,
    id: i64,
  ) -> AppResult<SchedulingResource> {
    let sql = format!(
      "SELECT r.id, r.first_name, r.last_name, r.display_name, r.active,
              r.normal_workdays, r.color, r.notes,
              o.id, o.name, o.address, o.phone_number
       FROM {} r
       LEFT JOIN offices o ON o.id = r.default_office_id
       WHERE r.id = ?1",
      Self::table(kind)
    );
    let mut resource = connection
      .query_row(&sql, [id], |row| Self::map_row(row, kind))
      .optional()
      .map_err(AppError::from_read_error)?
      .ok_or_else(|| AppError::not_found(kind.entity_name()))?;
    resource.offices = Self::list_offices(connection, kind, id)?;
    Ok(resource)
  }

  pub fn create(
    connection: &mut Connection,
    kind: ResourceKind,
    input: &ResourceInput,
  ) -> AppResult<SchedulingResource> {
    let transaction = connection.transaction().map_err(AppError::from_write_error)?;
    let sql = format!(
      "INSERT INTO {} (
         first_name, last_name, display_name, active, default_office_id,
         normal_workdays, color, notes
       ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      Self::table(kind)
    );
    transaction
      .execute(
        &sql,
        params![
          input.first_name,
          input.last_name,
          input.display_name,
          input.active,
          input.default_office.as_ref().map(|office| office.id),
          input.normal_workdays,
          input.color,
          input.notes,
        ],
      )
      .map_err(AppError::from_write_error)?;
    let id = transaction.last_insert_rowid();
    Self::replace_offices(&transaction, kind, id, input)?;
    transaction.commit().map_err(AppError::from_write_error)?;
    Self::get(connection, kind, id)
  }

  pub fn update(
    connection: &mut Connection,
    kind: ResourceKind,
    id: i64,
    input: &ResourceInput,
  ) -> AppResult<SchedulingResource> {
    let transaction = connection.transaction().map_err(AppError::from_write_error)?;
    let sql = format!(
      "UPDATE {}
       SET first_name = ?1, last_name = ?2, display_name = ?3, active = ?4,
           default_office_id = ?5, normal_workdays = ?6, color = ?7, notes = ?8,
           updated_at = CURRENT_TIMESTAMP
       WHERE id = ?9",
      Self::table(kind)
    );
    let changed = transaction
      .execute(
        &sql,
        params![
          input.first_name,
          input.last_name,
          input.display_name,
          input.active,
          input.default_office.as_ref().map(|office| office.id),
          input.normal_workdays,
          input.color,
          input.notes,
          id,
        ],
      )
      .map_err(AppError::from_write_error)?;
    if changed == 0 {
      return Err(AppError::not_found(kind.entity_name()));
    }
    Self::replace_offices(&transaction, kind, id, input)?;
    transaction.commit().map_err(AppError::from_write_error)?;
    Self::get(connection, kind, id)
  }

  pub fn delete(connection: &Connection, kind: ResourceKind, id: i64) -> AppResult<()> {
    let sql = format!("DELETE FROM {} WHERE id = ?1", Self::table(kind));
    let changed = connection
      .execute(&sql, [id])
      .map_err(AppError::from_write_error)?;
    if changed == 0 {
      return Err(AppError::not_found(kind.entity_name()));
    }
    Ok(())
  }

  pub fn exists(connection: &Connection, kind: ResourceKind, id: i64) -> AppResult<bool> {
    let sql = format!(
      "SELECT EXISTS(SELECT 1 FROM {} WHERE id = ?1)",
      Self::table(kind)
    );
    connection
      .query_row(&sql, [id], |row| row.get(0))
      .map_err(AppError::from_read_error)
  }

  fn list_offices(
    connection: &Connection,
    kind: ResourceKind,
    id: i64,
  ) -> AppResult<Vec<Office>> {
    let sql = format!(
      "SELECT o.id, o.name, o.address, o.phone_number
       FROM offices o
       INNER JOIN {} ro ON ro.office_id = o.id
       WHERE ro.{} = ?1
       ORDER BY o.name COLLATE NOCASE",
      Self::join_table(kind),
      Self::join_resource_column(kind)
    );
    let mut statement = connection.prepare(&sql).map_err(AppError::from_read_error)?;
    let offices = statement
      .query_map([id], |row| {
        Ok(Office {
          id: row.get(0)?,
          name: row.get(1)?,
          address: row.get(2)?,
          phone_number: row.get(3)?,
        })
      })
      .map_err(AppError::from_read_error)?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(AppError::from_read_error)?;
    Ok(offices)
  }

  fn replace_offices(
    transaction: &rusqlite::Transaction<'_>,
    kind: ResourceKind,
    id: i64,
    input: &ResourceInput,
  ) -> AppResult<()> {
    let delete_sql = format!(
      "DELETE FROM {} WHERE {} = ?1",
      Self::join_table(kind),
      Self::join_resource_column(kind)
    );
    transaction
      .execute(&delete_sql, [id])
      .map_err(AppError::from_write_error)?;

    let office_ids = input
      .offices
      .iter()
      .map(|office| office.id)
      .chain(input.default_office.iter().map(|office| office.id))
      .collect::<BTreeSet<_>>();
    let insert_sql = format!(
      "INSERT INTO {} ({}, office_id) VALUES (?1, ?2)",
      Self::join_table(kind),
      Self::join_resource_column(kind)
    );
    for office_id in office_ids {
      transaction
        .execute(&insert_sql, params![id, office_id])
        .map_err(AppError::from_write_error)?;
    }
    Ok(())
  }

  fn map_row(
    row: &rusqlite::Row<'_>,
    kind: ResourceKind,
  ) -> rusqlite::Result<SchedulingResource> {
    let default_office_id: Option<i64> = row.get(8)?;
    Ok(SchedulingResource {
      id: row.get(0)?,
      resource_type: kind.as_str().to_string(),
      first_name: row.get(1)?,
      last_name: row.get(2)?,
      display_name: row.get(3)?,
      active: row.get(4)?,
      normal_workdays: row.get(5)?,
      color: row.get(6)?,
      notes: row.get(7)?,
      default_office: default_office_id.map(|id| Office {
        id,
        name: row.get(9).unwrap_or_default(),
        address: row.get(10).unwrap_or_default(),
        phone_number: row.get(11).unwrap_or_default(),
      }),
      offices: Vec::new(),
    })
  }

  fn table(kind: ResourceKind) -> &'static str {
    match kind {
      ResourceKind::Doctor => "doctors",
      ResourceKind::Assistant => "assistants",
    }
  }

  fn join_table(kind: ResourceKind) -> &'static str {
    match kind {
      ResourceKind::Doctor => "doctor_offices",
      ResourceKind::Assistant => "assistant_offices",
    }
  }

  fn join_resource_column(kind: ResourceKind) -> &'static str {
    match kind {
      ResourceKind::Doctor => "doctor_id",
      ResourceKind::Assistant => "assistant_id",
    }
  }
}
