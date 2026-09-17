use crate::error::{AppError, AppResult};
use crate::models::{
  DoctorReference, DoctorWorkRule, DoctorWorkRuleInput, RotationGroup, RotationGroupInput,
};
use crate::repositories::office_repository::OfficeRepository;
use rusqlite::{params, Connection, OptionalExtension};

pub struct DoctorWorkRuleRepository;

#[derive(Debug)]
struct RawRule {
  id: i64,
  doctor_id: i64,
  day_of_week: String,
  office_id: Option<i64>,
  work_status: String,
  recurrence_type: String,
  rotation_group_id: Option<i64>,
  rotation_position: Option<i64>,
  effective_start_date: Option<String>,
  effective_end_date: Option<String>,
  active: bool,
}

impl DoctorWorkRuleRepository {
  pub fn list_rules(connection: &Connection, doctor_id: i64) -> AppResult<Vec<DoctorWorkRule>> {
    let mut statement = connection
      .prepare(
        "SELECT id, doctor_id, day_of_week, office_id, work_status,
                recurrence_type, rotation_group_id, rotation_position,
                effective_start_date, effective_end_date, active
         FROM doctor_work_rules
         WHERE doctor_id = ?1
         ORDER BY day_of_week, rotation_position",
      )
      .map_err(AppError::from_read_error)?;
    let raw_rules = statement
      .query_map([doctor_id], |row| {
        Ok(RawRule {
          id: row.get(0)?,
          doctor_id: row.get(1)?,
          day_of_week: row.get(2)?,
          office_id: row.get(3)?,
          work_status: row.get(4)?,
          recurrence_type: row.get(5)?,
          rotation_group_id: row.get(6)?,
          rotation_position: row.get(7)?,
          effective_start_date: row.get(8)?,
          effective_end_date: row.get(9)?,
          active: row.get(10)?,
        })
      })
      .map_err(AppError::from_read_error)?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(AppError::from_read_error)?;
    drop(statement);

    raw_rules
      .into_iter()
      .map(|rule| Self::hydrate_rule(connection, rule))
      .collect()
  }

  pub fn replace_rules(
    connection: &mut Connection,
    doctor_id: i64,
    rules: &[DoctorWorkRuleInput],
  ) -> AppResult<Vec<DoctorWorkRule>> {
    let transaction = connection.transaction().map_err(AppError::from_write_error)?;
    transaction
      .execute(
        "DELETE FROM doctor_work_rules WHERE doctor_id = ?1",
        [doctor_id],
      )
      .map_err(AppError::from_write_error)?;
    for rule in rules {
      Self::insert_rule(&transaction, doctor_id, rule)?;
    }
    transaction.commit().map_err(AppError::from_write_error)?;
    Self::list_rules(connection, doctor_id)
  }

  pub fn create_rule(
    connection: &Connection,
    doctor_id: i64,
    rule: &DoctorWorkRuleInput,
  ) -> AppResult<DoctorWorkRule> {
    let id = Self::insert_rule(connection, doctor_id, rule)?;
    Self::get_rule(connection, id)
  }

  pub fn delete_rule(connection: &Connection, id: i64) -> AppResult<()> {
    let changed = connection
      .execute("DELETE FROM doctor_work_rules WHERE id = ?1", [id])
      .map_err(AppError::from_write_error)?;
    if changed == 0 {
      return Err(AppError::not_found("Doctor work rule"));
    }
    Ok(())
  }

  pub fn list_rotation_groups(connection: &Connection) -> AppResult<Vec<RotationGroup>> {
    let mut statement = connection
      .prepare(
        "SELECT id, name, number_of_weeks, anchor_date, active
         FROM doctor_rotation_groups ORDER BY name COLLATE NOCASE",
      )
      .map_err(AppError::from_read_error)?;
    let groups = statement
      .query_map([], Self::map_rotation_group)
      .map_err(AppError::from_read_error)?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(AppError::from_read_error)?;
    Ok(groups)
  }

  pub fn create_rotation_group(
    connection: &Connection,
    input: &RotationGroupInput,
  ) -> AppResult<RotationGroup> {
    connection
      .execute(
        "INSERT INTO doctor_rotation_groups (name, number_of_weeks, anchor_date, active)
         VALUES (?1, ?2, ?3, ?4)",
        params![input.name, input.number_of_weeks, input.anchor_date, input.active],
      )
      .map_err(AppError::from_write_error)?;
    Self::get_rotation_group(connection, connection.last_insert_rowid())?
      .ok_or_else(|| AppError::not_found("Doctor rotation group"))
  }

  pub fn get_rotation_group(
    connection: &Connection,
    id: i64,
  ) -> AppResult<Option<RotationGroup>> {
    connection
      .query_row(
        "SELECT id, name, number_of_weeks, anchor_date, active
         FROM doctor_rotation_groups WHERE id = ?1",
        [id],
        Self::map_rotation_group,
      )
      .optional()
      .map_err(AppError::from_read_error)
  }

  fn insert_rule(
    connection: &Connection,
    doctor_id: i64,
    rule: &DoctorWorkRuleInput,
  ) -> AppResult<i64> {
    connection
      .execute(
        "INSERT INTO doctor_work_rules (
           doctor_id, day_of_week, office_id, work_status, recurrence_type,
           rotation_group_id, rotation_position, effective_start_date,
           effective_end_date, active
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
          doctor_id,
          rule.day_of_week,
          rule.office.as_ref().map(|office| office.id),
          rule.work_status,
          rule.recurrence_type,
          rule.rotation_group.as_ref().map(|group| group.id),
          rule.rotation_position,
          rule.effective_start_date,
          rule.effective_end_date,
          rule.active,
        ],
      )
      .map_err(AppError::from_write_error)?;
    Ok(connection.last_insert_rowid())
  }

  fn get_rule(connection: &Connection, id: i64) -> AppResult<DoctorWorkRule> {
    let raw = connection
      .query_row(
        "SELECT id, doctor_id, day_of_week, office_id, work_status,
                recurrence_type, rotation_group_id, rotation_position,
                effective_start_date, effective_end_date, active
         FROM doctor_work_rules WHERE id = ?1",
        [id],
        |row| {
          Ok(RawRule {
            id: row.get(0)?,
            doctor_id: row.get(1)?,
            day_of_week: row.get(2)?,
            office_id: row.get(3)?,
            work_status: row.get(4)?,
            recurrence_type: row.get(5)?,
            rotation_group_id: row.get(6)?,
            rotation_position: row.get(7)?,
            effective_start_date: row.get(8)?,
            effective_end_date: row.get(9)?,
            active: row.get(10)?,
          })
        },
      )
      .optional()
      .map_err(AppError::from_read_error)?
      .ok_or_else(|| AppError::not_found("Doctor work rule"))?;
    Self::hydrate_rule(connection, raw)
  }

  fn hydrate_rule(connection: &Connection, raw: RawRule) -> AppResult<DoctorWorkRule> {
    let office = raw
      .office_id
      .map(|id| OfficeRepository::get(connection, id))
      .transpose()?;
    let rotation_group = raw
      .rotation_group_id
      .map(|id| {
        Self::get_rotation_group(connection, id)?.ok_or_else(|| {
          AppError::new(
            "INTEGRITY_FAILURE",
            "A doctor work rule references a missing rotation group.",
          )
        })
      })
      .transpose()?;
    Ok(DoctorWorkRule {
      id: raw.id,
      doctor: DoctorReference { id: raw.doctor_id },
      day_of_week: raw.day_of_week,
      office,
      work_status: raw.work_status,
      recurrence_type: raw.recurrence_type,
      rotation_group,
      rotation_position: raw.rotation_position,
      effective_start_date: raw.effective_start_date,
      effective_end_date: raw.effective_end_date,
      active: raw.active,
    })
  }

  fn map_rotation_group(row: &rusqlite::Row<'_>) -> rusqlite::Result<RotationGroup> {
    Ok(RotationGroup {
      id: row.get(0)?,
      name: row.get(1)?,
      number_of_weeks: row.get(2)?,
      anchor_date: row.get(3)?,
      active: row.get(4)?,
    })
  }
}
