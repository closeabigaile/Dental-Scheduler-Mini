use super::{office_repository::OfficeRepository, resource_repository::ResourceRepository};
use crate::error::{AppError, AppResult};
use crate::models::{IdReference, ResourceKind, Team, TeamInput};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeSet;

pub struct TeamRepository;
impl TeamRepository {
  pub fn list(c: &Connection) -> AppResult<Vec<Team>> {
    let mut s = c
      .prepare("SELECT id FROM teams ORDER BY name COLLATE NOCASE")
      .map_err(AppError::from_read_error)?;
    let ids = s
      .query_map([], |r| r.get::<_, i64>(0))
      .map_err(AppError::from_read_error)?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(AppError::from_read_error)?;
    ids.into_iter().map(|id| Self::get(c, id)).collect()
  }
  pub fn get(c: &Connection, id: i64) -> AppResult<Team> {
    let (name, doctor, office, active, color, notes) = c
      .query_row(
        "SELECT name, doctor_id, default_office_id, active, color, notes FROM teams WHERE id=?1",
        [id],
        |r| {
          Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Option<i64>>(2)?,
            r.get::<_, bool>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
          ))
        },
      )
      .optional()
      .map_err(AppError::from_read_error)?
      .ok_or_else(|| AppError::not_found("Team"))?;
    let mut s = c
      .prepare("SELECT assistant_id FROM team_assistants WHERE team_id=?1 ORDER BY assistant_id")
      .map_err(AppError::from_read_error)?;
    let ids = s
      .query_map([id], |r| r.get::<_, i64>(0))
      .map_err(AppError::from_read_error)?
      .collect::<rusqlite::Result<Vec<_>>>()
      .map_err(AppError::from_read_error)?;
    Ok(Team {
      id,
      name,
      doctor: ResourceRepository::get(c, ResourceKind::Doctor, doctor)?,
      assistants: ids
        .into_iter()
        .map(|a| ResourceRepository::get(c, ResourceKind::Assistant, a))
        .collect::<AppResult<_>>()?,
      default_office: office.map(|o| OfficeRepository::get(c, o)).transpose()?,
      active,
      color,
      notes,
    })
  }
  // Caller owns the transaction encompassing domain checks and every membership write.
  pub fn save(c: &Connection, id: Option<i64>, v: &TeamInput) -> AppResult<Team> {
    let office = v.default_office.as_ref().map(|o| o.id);
    let id = if let Some(id) = id {
      if c.execute("UPDATE teams SET name=?1,doctor_id=?2,default_office_id=?3,active=?4,color=?5,notes=?6,updated_at=CURRENT_TIMESTAMP WHERE id=?7",
                params![v.name,v.doctor.id,office,v.active,v.color,v.notes,id]).map_err(AppError::from_write_error)? == 0 { return Err(AppError::not_found("Team")); }
      id
    } else {
      c.execute("INSERT INTO teams(name,doctor_id,default_office_id,active,color,notes) VALUES(?1,?2,?3,?4,?5,?6)",
                params![v.name,v.doctor.id,office,v.active,v.color,v.notes]).map_err(AppError::from_write_error)?;
      c.last_insert_rowid()
    };
    c.execute("DELETE FROM team_assistants WHERE team_id=?1", [id])
      .map_err(AppError::from_write_error)?;
    for a in v.assistants.iter().map(|a| a.id).collect::<BTreeSet<_>>() {
      c.execute(
        "INSERT INTO team_assistants(team_id,assistant_id) VALUES(?1,?2)",
        params![id, a],
      )
      .map_err(AppError::from_write_error)?;
    }
    Self::get(c, id)
  }
  pub fn delete(c: &Connection, id: i64) -> AppResult<()> {
    if c
      .execute("DELETE FROM teams WHERE id=?1", [id])
      .map_err(AppError::from_write_error)?
      == 0
    {
      return Err(AppError::not_found("Team"));
    }
    Ok(())
  }
  pub fn input(t: &Team) -> TeamInput {
    TeamInput {
      name: t.name.clone(),
      doctor: IdReference { id: t.doctor.id },
      assistants: t
        .assistants
        .iter()
        .map(|a| IdReference { id: a.id })
        .collect(),
      default_office: t.default_office.as_ref().map(|o| IdReference { id: o.id }),
      active: t.active,
      color: t.color.clone(),
      notes: t.notes.clone(),
    }
  }
}
