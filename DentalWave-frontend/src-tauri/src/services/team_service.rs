use crate::repositories::{
  office_repository::OfficeRepository, resource_repository::ResourceRepository,
  team_repository::TeamRepository,
};
use crate::{
  database::Database,
  error::{AppError, AppResult},
  models::{ResourceKind, Team, TeamInput},
};
pub struct TeamService<'a> {
  database: &'a Database,
}
impl<'a> TeamService<'a> {
  pub fn new(database: &'a Database) -> Self {
    Self { database }
  }
  pub fn list(&self) -> AppResult<Vec<Team>> {
    TeamRepository::list(&self.database.connect()?)
  }
  pub fn get(&self, id: i64) -> AppResult<Team> {
    TeamRepository::get(&self.database.connect()?, id)
  }
  pub fn save(&self, id: Option<i64>, mut input: TeamInput) -> AppResult<Team> {
    let mut c = self.database.connect()?;
    let tx = c
      .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
      .map_err(AppError::from_write_error)?;
    Self::validate(&tx, &mut input)?;
    let team = TeamRepository::save(&tx, id, &input)?;
    tx.commit().map_err(AppError::from_write_error)?;
    Ok(team)
  }
  fn validate(c: &rusqlite::Connection, v: &mut TeamInput) -> AppResult<()> {
    v.name = v.name.trim().to_string();
    if v.name.is_empty() {
      return Err(AppError::new("INVALID_TEAM", "Team name is required."));
    }
    for (kind, id) in std::iter::once((ResourceKind::Doctor, v.doctor.id))
      .chain(v.assistants.iter().map(|a| (ResourceKind::Assistant, a.id)))
    {
      if !ResourceRepository::exists(c, kind, id)? || !ResourceRepository::get(c, kind, id)?.active
      {
        return Err(AppError::new(
          "INVALID_TEAM",
          format!("Selected {} is missing or inactive.", kind.entity_name()),
        ));
      }
    }
    if let Some(o) = &v.default_office {
      if !OfficeRepository::exists(c, o.id)? {
        return Err(AppError::new(
          "INVALID_TEAM",
          "Default office was not found.",
        ));
      }
    }
    Ok(())
  }
  pub fn duplicate(&self, id: i64) -> AppResult<Team> {
    let mut c = self.database.connect()?;
    let tx = c
      .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
      .map_err(AppError::from_write_error)?;
    let source = TeamRepository::get(&tx, id)?;
    let mut v = TeamRepository::input(&source);
    let base = format!("{} Copy", v.name);
    v.name = base.clone();
    let mut suffix = 2;
    let teams = TeamRepository::list(&tx)?;
    while teams.iter().any(|t| t.name.eq_ignore_ascii_case(&v.name)) {
      v.name = format!("{base} {suffix}");
      suffix += 1;
    }
    // Java duplication preserves the source, even if its members were deactivated later.
    let team = TeamRepository::save(&tx, None, &v)?;
    tx.commit().map_err(AppError::from_write_error)?;
    Ok(team)
  }
  pub fn delete(&self, id: i64) -> AppResult<()> {
    TeamRepository::delete(&self.database.connect()?, id)
  }
}
