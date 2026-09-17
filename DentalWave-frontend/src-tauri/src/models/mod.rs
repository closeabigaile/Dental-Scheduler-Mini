use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Office {
  pub id: i64,
  pub name: String,
  pub address: String,
  pub phone_number: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeInput {
  pub name: String,
  #[serde(default)]
  pub address: String,
  #[serde(default)]
  pub phone_number: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
  Doctor,
  Assistant,
}

impl ResourceKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Doctor => "DOCTOR",
      Self::Assistant => "ASSISTANT",
    }
  }

  pub fn entity_name(self) -> &'static str {
    match self {
      Self::Doctor => "Doctor",
      Self::Assistant => "Assistant",
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdReference {
  pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInput {
  #[serde(rename = "type")]
  pub resource_type: Option<String>,
  #[serde(default)]
  pub first_name: String,
  #[serde(default)]
  pub last_name: String,
  pub display_name: String,
  #[serde(default = "default_true")]
  pub active: bool,
  pub default_office: Option<IdReference>,
  #[serde(default)]
  pub offices: Vec<IdReference>,
  pub normal_workdays: Option<String>,
  #[serde(default = "default_color")]
  pub color: String,
  #[serde(default)]
  pub notes: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SchedulingResource {
  pub id: i64,
  #[serde(rename = "type")]
  pub resource_type: String,
  pub first_name: String,
  pub last_name: String,
  pub display_name: String,
  pub active: bool,
  pub default_office: Option<Office>,
  pub offices: Vec<Office>,
  pub normal_workdays: Option<String>,
  pub color: String,
  pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RotationGroup {
  pub id: i64,
  pub name: String,
  pub number_of_weeks: i64,
  pub anchor_date: String,
  pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationGroupInput {
  pub name: String,
  pub number_of_weeks: i64,
  pub anchor_date: String,
  #[serde(default = "default_true")]
  pub active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReference {
  pub id: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DoctorWorkRule {
  pub id: i64,
  pub doctor: DoctorReference,
  pub day_of_week: String,
  pub office: Option<Office>,
  pub work_status: String,
  pub recurrence_type: String,
  pub rotation_group: Option<RotationGroup>,
  pub rotation_position: Option<i64>,
  pub effective_start_date: Option<String>,
  pub effective_end_date: Option<String>,
  pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorWorkRuleInput {
  pub doctor: IdReference,
  pub day_of_week: String,
  pub office: Option<IdReference>,
  pub work_status: String,
  pub recurrence_type: String,
  pub rotation_group: Option<IdReference>,
  pub rotation_position: Option<i64>,
  pub effective_start_date: Option<String>,
  pub effective_end_date: Option<String>,
  #[serde(default = "default_true")]
  pub active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
  pub path: String,
  pub schema_version: i64,
  pub integrity_status: String,
}

fn default_true() -> bool {
  true
}

fn default_color() -> String {
  "#65a9b8".to_string()
}
