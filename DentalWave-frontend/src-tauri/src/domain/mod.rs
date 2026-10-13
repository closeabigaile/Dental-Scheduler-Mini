//! Pure scheduling rules: no Tauri, repositories, clocks, or database access.
use crate::models::{DoctorWorkRule, Office, SchedulingResource, Team};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

pub const DAYS: [&str; 7] = [
  "MONDAY",
  "TUESDAY",
  "WEDNESDAY",
  "THURSDAY",
  "FRIDAY",
  "SATURDAY",
  "SUNDAY",
];
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConflictCode {
  InvalidDate,
  OfficeInvalid,
  TeamInvalid,
  MissingRequiredResource,
  DoctorInvalid,
  AssistantInvalid,
  DoctorNotAvailable,
  DoctorWrongOffice,
  AssistantWrongOffice,
  DoctorAlreadyAssigned,
  AssistantAlreadyAssigned,
  PatternConflict,
  InvalidPattern,
}
impl ConflictCode {
  pub fn message(&self) -> &'static str {
    match self {
      Self::InvalidDate => "Use a valid date",
      Self::OfficeInvalid => "The office no longer exists",
      Self::TeamInvalid => "The reusable team is inactive, missing, or its members do not match",
      Self::MissingRequiredResource => "Assign at least one assistant to the doctor",
      Self::DoctorInvalid => "The doctor is inactive or no longer exists",
      Self::AssistantInvalid => "An assistant is inactive or no longer exists",
      Self::DoctorNotAvailable => "The doctor is not available on this date",
      Self::DoctorWrongOffice => "The doctor is not assigned to this office on this date",
      Self::AssistantWrongOffice => "An assistant is not eligible for this office",
      Self::DoctorAlreadyAssigned => "The doctor is already assigned on this date",
      Self::AssistantAlreadyAssigned => "An assistant is already assigned on this date",
      Self::PatternConflict => "Doctor work patterns overlap on this date",
      Self::InvalidPattern => "The doctor work pattern is invalid",
    }
  }
}
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
  pub code: ConflictCode,
  pub resource_id: Option<i64>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Validation {
  pub valid: bool,
  pub conflicts: Vec<Conflict>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
  pub date: String,
  pub office_id: i64,
  pub team_id: Option<i64>,
  pub doctor_id: i64,
  #[serde(default)]
  pub assistant_ids: Vec<i64>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
  pub doctor_id: i64,
  pub doctor_name: String,
  pub date: String,
  pub working: bool,
  pub office_id: Option<i64>,
  pub office_name: Option<String>,
  pub rule_id: Option<i64>,
  pub recurrence_type: Option<String>,
  pub rotation_position: Option<i64>,
  pub fallback: bool,
  pub conflict: Option<ConflictCode>,
}
pub fn date(value: &str) -> Result<NaiveDate, ConflictCode> {
  let parsed =
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| ConflictCode::InvalidDate)?;
  if parsed.format("%Y-%m-%d").to_string() != value {
    return Err(ConflictCode::InvalidDate);
  }
  Ok(parsed)
}
pub fn office_allowed(resource: &SchedulingResource, office: i64) -> bool {
  if !resource.offices.is_empty() {
    resource.offices.iter().any(|o| o.id == office)
  } else {
    resource
      .default_office
      .as_ref()
      .map_or(true, |o| o.id == office)
  }
}
pub fn resolve(
  doctor: &SchedulingResource,
  rules: &[DoctorWorkRule],
  day: NaiveDate,
) -> Resolution {
  let mut result = Resolution {
    doctor_id: doctor.id,
    doctor_name: doctor.display_name.clone(),
    date: day.to_string(),
    working: false,
    office_id: None,
    office_name: None,
    rule_id: None,
    recurrence_type: None,
    rotation_position: None,
    fallback: rules.is_empty(),
    conflict: None,
  };
  if !doctor.active {
    result.conflict = Some(ConflictCode::DoctorInvalid);
    return result;
  }
  let mut applicable = Vec::new();
  for rule in rules
    .iter()
    .filter(|r| r.active && r.day_of_week == DAYS[day.weekday().num_days_from_monday() as usize])
  {
    let matches = (|| -> Result<bool, ConflictCode> {
      let start = rule.effective_start_date.as_deref().map(date).transpose()?;
      let end = rule.effective_end_date.as_deref().map(date).transpose()?;
      if matches!((start, end), (Some(s), Some(e)) if e < s) {
        return Err(ConflictCode::InvalidPattern);
      }
      if start.is_some_and(|s| day < s) || end.is_some_and(|e| day > e) {
        return Ok(false);
      }
      if rule.recurrence_type == "EVERY_WEEK" {
        return Ok(true);
      }
      let g = rule
        .rotation_group
        .as_ref()
        .ok_or(ConflictCode::InvalidPattern)?;
      if !g.active {
        return Ok(false);
      }
      if g.number_of_weeks < 1
        || rule
          .rotation_position
          .map_or(true, |p| p < 1 || p > g.number_of_weeks)
      {
        return Err(ConflictCode::InvalidPattern);
      }
      let weeks = (day - date(&g.anchor_date)?).num_days().div_euclid(7);
      Ok(Some(weeks.rem_euclid(g.number_of_weeks) + 1) == rule.rotation_position)
    })();
    match matches {
      Ok(true) => applicable.push(rule),
      Ok(false) => {}
      Err(_) => {
        result.conflict = Some(ConflictCode::InvalidPattern);
        return result;
      }
    }
  }
  if applicable.len() > 1 {
    result.conflict = Some(ConflictCode::PatternConflict);
    return result;
  }
  if let Some(r) = applicable.first() {
    result.working = r.work_status == "WORKING";
    result.office_id = if result.working {
      r.office.as_ref().map(|o| o.id)
    } else {
      None
    };
    result.office_name = if result.working {
      r.office.as_ref().map(|o| o.name.clone())
    } else {
      None
    };
    result.rule_id = Some(r.id);
    result.recurrence_type = Some(r.recurrence_type.clone());
    result.rotation_position = r.rotation_position;
    if result.working && result.office_id.is_none() {
      result.conflict = Some(ConflictCode::InvalidPattern);
    }
  }
  result
}
fn fallback_works(doctor: &SchedulingResource, day: NaiveDate) -> bool {
  let weekday = day.weekday().num_days_from_monday() as usize;
  if weekday >= 5 {
    return false;
  }
  match doctor
    .normal_workdays
    .as_deref()
    .filter(|v| !v.trim().is_empty())
  {
    None => weekday < 4,
    Some(days) => days.to_lowercase().contains(&DAYS[weekday].to_lowercase()),
  }
}
pub struct Context<'a> {
  pub offices: &'a [Office],
  pub doctors: &'a [SchedulingResource],
  pub assistants: &'a [SchedulingResource],
  pub teams: &'a [Team],
  pub rules: &'a [DoctorWorkRule],
}
/// Existing assignments are caller-provided full-day snapshots; exclude the edited assignment.
/// This function never modifies the proposal or fills in resources silently.
pub fn validate(proposal: &Assignment, existing: &[Assignment], ctx: &Context<'_>) -> Validation {
  let mut conflicts = Vec::new();
  let mut add = |code, resource_id| conflicts.push(Conflict { code, resource_id });
  let day = match date(&proposal.date) {
    Ok(day) => Some(day),
    Err(code) => {
      add(code, None);
      None
    }
  };
  if !ctx.offices.iter().any(|o| o.id == proposal.office_id) {
    add(ConflictCode::OfficeInvalid, Some(proposal.office_id));
  }
  if let Some(id) = proposal.team_id {
    match ctx.teams.iter().find(|t| t.id == id) {
      Some(t)
        if t.active && t.doctor.id == proposal.doctor_id && {
          let mut actual = proposal.assistant_ids.clone();
          actual.sort();
          actual.dedup();
          let mut expected = t.assistants.iter().map(|a| a.id).collect::<Vec<_>>();
          expected.sort();
          actual == expected
        } => {}
      _ => add(ConflictCode::TeamInvalid, Some(id)),
    }
  }
  if proposal.assistant_ids.is_empty() {
    add(ConflictCode::MissingRequiredResource, None);
  }
  match ctx.doctors.iter().find(|d| d.id == proposal.doctor_id) {
    Some(d) if d.active => {
      if let Some(day) = day {
        let rules = ctx
          .rules
          .iter()
          .filter(|r| r.doctor.id == d.id)
          .cloned()
          .collect::<Vec<_>>();
        let r = resolve(d, &rules, day);
        if let Some(code) = r.conflict {
          add(code, Some(d.id));
        } else if r.fallback {
          if !fallback_works(d, day) {
            add(ConflictCode::DoctorNotAvailable, Some(d.id));
          }
          if !office_allowed(d, proposal.office_id) {
            add(ConflictCode::DoctorWrongOffice, Some(d.id));
          }
        } else if !r.working {
          add(ConflictCode::DoctorNotAvailable, Some(d.id));
        } else if r.office_id != Some(proposal.office_id) {
          add(ConflictCode::DoctorWrongOffice, Some(d.id));
        }
      }
    }
    _ => add(ConflictCode::DoctorInvalid, Some(proposal.doctor_id)),
  }
  let mut seen = std::collections::HashSet::new();
  for id in &proposal.assistant_ids {
    if !seen.insert(*id) {
      add(ConflictCode::AssistantAlreadyAssigned, Some(*id));
    }
    match ctx.assistants.iter().find(|a| a.id == *id) {
      Some(a) if a.active => {
        if !office_allowed(a, proposal.office_id) {
          add(ConflictCode::AssistantWrongOffice, Some(*id));
        }
      }
      _ => add(ConflictCode::AssistantInvalid, Some(*id)),
    }
  }
  for other in existing.iter().filter(|a| a.date == proposal.date) {
    if other.doctor_id == proposal.doctor_id {
      add(
        ConflictCode::DoctorAlreadyAssigned,
        Some(proposal.doctor_id),
      );
    }
    for id in &proposal.assistant_ids {
      if other.assistant_ids.contains(id) {
        add(ConflictCode::AssistantAlreadyAssigned, Some(*id));
      }
    }
  }
  conflicts.dedup();
  Validation {
    valid: conflicts.is_empty(),
    conflicts,
  }
}

pub fn pattern_input(r: &DoctorWorkRule) -> crate::models::DoctorWorkRuleInput {
  use crate::models::{DoctorWorkRuleInput, IdReference};
  DoctorWorkRuleInput {
    doctor: IdReference { id: r.doctor.id },
    day_of_week: r.day_of_week.clone(),
    office: r.office.as_ref().map(|o| IdReference { id: o.id }),
    work_status: r.work_status.clone(),
    recurrence_type: r.recurrence_type.clone(),
    rotation_group: r.rotation_group.as_ref().map(|g| IdReference { id: g.id }),
    rotation_position: r.rotation_position,
    effective_start_date: r.effective_start_date.clone(),
    effective_end_date: r.effective_end_date.clone(),
    active: r.active,
  }
}
/// Java rejects overlapping active rules in the same weekday/cycle slot.
/// Different cycles (or weekly + rotating) can coexist; resolution reports collisions.
pub fn validate_pattern_set(
  rules: &[crate::models::DoctorWorkRuleInput],
) -> crate::error::AppResult<()> {
  for (i, a) in rules.iter().enumerate().filter(|(_, r)| r.active) {
    for b in rules.iter().skip(i + 1).filter(|r| r.active) {
      let same = a.doctor.id == b.doctor.id
        && a.day_of_week == b.day_of_week
        && a.recurrence_type == b.recurrence_type
        && (a.recurrence_type == "EVERY_WEEK"
          || (a.rotation_group.as_ref().map(|g| g.id) == b.rotation_group.as_ref().map(|g| g.id)
            && a.rotation_position == b.rotation_position));
      let before = |end: &Option<String>, start: &Option<String>| matches!((end,start),(Some(e),Some(s)) if e<s);
      if same
        && !before(&a.effective_end_date, &b.effective_start_date)
        && !before(&b.effective_end_date, &a.effective_start_date)
      {
        return Err(crate::error::AppError::new(
          "PATTERN_CONFLICT",
          "Two active rules overlap in the same weekday and cycle position.",
        ));
      }
    }
  }
  Ok(())
}
