use crate::repositories::{
  doctor_work_rule_repository::DoctorWorkRuleRepository, office_repository::OfficeRepository,
  resource_repository::ResourceRepository, team_repository::TeamRepository,
};
use crate::{
  database::Database,
  domain::{self, Assignment, Context, Resolution, Validation},
  error::{AppError, AppResult},
  models::ResourceKind,
};
use chrono::{Datelike, Duration};
pub struct SchedulingService<'a> {
  database: &'a Database,
}
impl<'a> SchedulingService<'a> {
  pub fn new(database: &'a Database) -> Self {
    Self { database }
  }
  pub fn preview(&self, id: i64, start: &str, days: i64) -> AppResult<Vec<Resolution>> {
    if !(1..=84).contains(&days) {
      return Err(AppError::invalid_input(
        "Preview days must be between 1 and 84.",
      ));
    }
    let start = domain::date(start)
      .map_err(|_| AppError::new("INVALID_DATE", "Use a valid YYYY-MM-DD date."))?;
    let mut c = self.database.connect()?;
    let tx = c.transaction().map_err(AppError::from_read_error)?;
    let doctor = ResourceRepository::get(&tx, ResourceKind::Doctor, id)?;
    let rules = DoctorWorkRuleRepository::list_rules(&tx, id)?;
    (0..days)
      .filter_map(|offset| {
        let day = start.checked_add_signed(Duration::days(offset))?;
        (day.weekday().num_days_from_monday() < 4).then(|| domain::resolve(&doctor, &rules, day))
      })
      .map(Ok)
      .collect()
  }
  pub fn resolve(&self, id: i64, day: &str) -> AppResult<Resolution> {
    let day = domain::date(day)
      .map_err(|_| AppError::new("INVALID_DATE", "Use a valid YYYY-MM-DD date."))?;
    let mut c = self.database.connect()?;
    let tx = c.transaction().map_err(AppError::from_read_error)?;
    Ok(domain::resolve(
      &ResourceRepository::get(&tx, ResourceKind::Doctor, id)?,
      &DoctorWorkRuleRepository::list_rules(&tx, id)?,
      day,
    ))
  }
  pub fn validate(&self, proposal: &Assignment, existing: &[Assignment]) -> AppResult<Validation> {
    let mut c = self.database.connect()?;
    let tx = c.transaction().map_err(AppError::from_read_error)?;
    let offices = OfficeRepository::list(&tx)?;
    let doctors = ResourceRepository::list(&tx, ResourceKind::Doctor)?;
    let assistants = ResourceRepository::list(&tx, ResourceKind::Assistant)?;
    let teams = TeamRepository::list(&tx)?;
    let rules = doctors
      .iter()
      .map(|d| DoctorWorkRuleRepository::list_rules(&tx, d.id))
      .collect::<AppResult<Vec<_>>>()?
      .into_iter()
      .flatten()
      .collect::<Vec<_>>();
    Ok(domain::validate(
      proposal,
      existing,
      &Context {
        offices: &offices,
        doctors: &doctors,
        assistants: &assistants,
        teams: &teams,
        rules: &rules,
      },
    ))
  }
}
