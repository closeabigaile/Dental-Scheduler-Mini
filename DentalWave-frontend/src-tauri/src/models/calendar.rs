use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantAssignment {
  pub assistant_id: i64,
  #[serde(default)] pub partial_day_note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarAssignment {
  #[serde(default)] pub id: i64,
  pub doctor_id: Option<i64>,
  pub team_id: Option<i64>,
  #[serde(default)] pub label: String,
  #[serde(default)] pub assistants: Vec<AssistantAssignment>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDay {
  #[serde(default)] pub id: i64,
  pub date: String,
  #[serde(default)] pub notes: String,
  #[serde(default)] pub assignments: Vec<CalendarAssignment>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerationIssue {
  pub date: String,
  pub code: String,
  pub doctor_id: Option<i64>,
  pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
  pub id: i64,
  pub year: i32,
  pub month: u32,
  pub office_id: i64,
  pub published: bool,
  pub revision: i64,
  pub days: Vec<CalendarDay>,
  #[serde(default)] pub generation_issues: Vec<GenerationIssue>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarResult {
  pub status: String,
  pub calendars: Vec<Calendar>,
}
