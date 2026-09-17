use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{
  DoctorWorkRule, DoctorWorkRuleInput, ResourceKind, RotationGroup, RotationGroupInput,
};
use crate::repositories::doctor_work_rule_repository::DoctorWorkRuleRepository;
use crate::repositories::resource_repository::ResourceRepository;

pub struct DoctorWorkRuleService<'a> {
  database: &'a Database,
}

impl<'a> DoctorWorkRuleService<'a> {
  pub fn new(database: &'a Database) -> Self {
    Self { database }
  }

  pub fn list_rules(&self, doctor_id: i64) -> AppResult<Vec<DoctorWorkRule>> {
    let connection = self.database.connect()?;
    Self::validate_doctor(&connection, doctor_id)?;
    DoctorWorkRuleRepository::list_rules(&connection, doctor_id)
  }

  pub fn replace_rules(
    &self,
    doctor_id: i64,
    rules: Vec<DoctorWorkRuleInput>,
  ) -> AppResult<Vec<DoctorWorkRule>> {
    let mut connection = self.database.connect()?;
    Self::validate_doctor(&connection, doctor_id)?;
    for rule in &rules {
      Self::validate_rule(&connection, doctor_id, rule)?;
    }
    DoctorWorkRuleRepository::replace_rules(&mut connection, doctor_id, &rules)
  }

  pub fn create_rule(&self, input: DoctorWorkRuleInput) -> AppResult<DoctorWorkRule> {
    let connection = self.database.connect()?;
    let doctor_id = input.doctor.id;
    Self::validate_doctor(&connection, doctor_id)?;
    Self::validate_rule(&connection, doctor_id, &input)?;
    DoctorWorkRuleRepository::create_rule(&connection, doctor_id, &input)
  }

  pub fn delete_rule(&self, id: i64) -> AppResult<()> {
    if id <= 0 {
      return Err(AppError::invalid_input("A valid doctor work rule id is required."));
    }
    DoctorWorkRuleRepository::delete_rule(&self.database.connect()?, id)
  }

  pub fn list_rotation_groups(&self) -> AppResult<Vec<RotationGroup>> {
    DoctorWorkRuleRepository::list_rotation_groups(&self.database.connect()?)
  }

  pub fn create_rotation_group(&self, mut input: RotationGroupInput) -> AppResult<RotationGroup> {
    input.name = input.name.trim().to_string();
    input.anchor_date = input.anchor_date.trim().to_string();
    if input.name.is_empty() {
      return Err(AppError::invalid_input("Rotation group name is required."));
    }
    if input.number_of_weeks <= 0 {
      return Err(AppError::invalid_input(
        "Rotation group weeks must be greater than zero.",
      ));
    }
    if !Self::looks_like_iso_date(&input.anchor_date) {
      return Err(AppError::invalid_input(
        "Rotation group anchor date must use YYYY-MM-DD format.",
      ));
    }
    DoctorWorkRuleRepository::create_rotation_group(&self.database.connect()?, &input)
  }

  fn validate_doctor(connection: &rusqlite::Connection, doctor_id: i64) -> AppResult<()> {
    if doctor_id <= 0
      || !ResourceRepository::exists(connection, ResourceKind::Doctor, doctor_id)?
    {
      return Err(AppError::not_found("Doctor"));
    }
    Ok(())
  }

  fn validate_rule(
    connection: &rusqlite::Connection,
    doctor_id: i64,
    rule: &DoctorWorkRuleInput,
  ) -> AppResult<()> {
    if rule.doctor.id != doctor_id {
      return Err(AppError::invalid_input(
        "Every work rule must reference the selected doctor.",
      ));
    }
    const DAYS: [&str; 7] = [
      "MONDAY",
      "TUESDAY",
      "WEDNESDAY",
      "THURSDAY",
      "FRIDAY",
      "SATURDAY",
      "SUNDAY",
    ];
    if !DAYS.contains(&rule.day_of_week.as_str()) {
      return Err(AppError::invalid_input("Doctor work-rule day is invalid."));
    }
    if !["WORKING", "NOT_WORKING"].contains(&rule.work_status.as_str()) {
      return Err(AppError::invalid_input("Doctor work status is invalid."));
    }
    if rule.work_status == "WORKING" && rule.office.is_none() {
      return Err(AppError::invalid_input(
        "A working doctor rule must select an office.",
      ));
    }
    if let Some(office) = &rule.office {
      if !crate::repositories::office_repository::OfficeRepository::exists(
        connection,
        office.id,
      )? {
        return Err(AppError::invalid_input(
          "Doctor work-rule office was not found.",
        ));
      }
    }
    match rule.recurrence_type.as_str() {
      "EVERY_WEEK" if rule.rotation_group.is_none() && rule.rotation_position.is_none() => {}
      "ROTATING"
        if rule.rotation_group.is_some()
          && rule.rotation_position.is_some_and(|position| position > 0) =>
      {
        let group_id = rule.rotation_group.as_ref().expect("checked above").id;
        if DoctorWorkRuleRepository::get_rotation_group(connection, group_id)?.is_none() {
          return Err(AppError::invalid_input(
            "Doctor rotation group was not found.",
          ));
        }
      }
      _ => {
        return Err(AppError::invalid_input(
          "Doctor recurrence settings are invalid.",
        ));
      }
    }
    for date in [
      rule.effective_start_date.as_deref(),
      rule.effective_end_date.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
      if !Self::looks_like_iso_date(date) {
        return Err(AppError::invalid_input(
          "Doctor work-rule dates must use YYYY-MM-DD format.",
        ));
      }
    }
    Ok(())
  }

  fn looks_like_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
      && bytes[4] == b'-'
      && bytes[7] == b'-'
      && bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
  }
}
