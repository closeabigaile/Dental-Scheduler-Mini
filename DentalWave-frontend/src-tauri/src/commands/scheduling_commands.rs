use crate::{
  database::AppState,
  domain::{Assignment, Resolution, Validation},
  error::AppResult,
  models::{Team, TeamInput},
  services::{scheduling_service::SchedulingService, team_service::TeamService},
};
use tauri::State;
#[tauri::command]
pub fn get_teams(state: State<'_, AppState>) -> AppResult<Vec<Team>> {
  state.with_database(false, false, |db| TeamService::new(db).list())
}
#[tauri::command]
pub fn get_team(id: i64, state: State<'_, AppState>) -> AppResult<Team> {
  state.with_database(false, false, |db| TeamService::new(db).get(id))
}
#[tauri::command]
pub fn create_team(input: TeamInput, state: State<'_, AppState>) -> AppResult<Team> {
  state.with_database(true, false, |db| TeamService::new(db).save(None, input))
}
#[tauri::command]
pub fn update_team(id: i64, input: TeamInput, state: State<'_, AppState>) -> AppResult<Team> {
  state.with_database(true, false, |db| TeamService::new(db).save(Some(id), input))
}
#[tauri::command]
pub fn duplicate_team(id: i64, state: State<'_, AppState>) -> AppResult<Team> {
  state.with_database(true, false, |db| TeamService::new(db).duplicate(id))
}
#[tauri::command]
pub fn delete_team(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  state.with_database(true, true, |db| TeamService::new(db).delete(id))
}
#[tauri::command]
pub fn preview_doctor_rules(
  doctor_id: i64,
  start_date: String,
  days: i64,
  state: State<'_, AppState>,
) -> AppResult<Vec<Resolution>> {
  state.with_database(false, false, |db| SchedulingService::new(db).preview(doctor_id, &start_date, days))
}
#[tauri::command]
pub fn resolve_doctor_location(
  doctor_id: i64,
  date: String,
  state: State<'_, AppState>,
) -> AppResult<Resolution> {
  state.with_database(false, false, |db| SchedulingService::new(db).resolve(doctor_id, &date))
}
#[tauri::command]
pub fn validate_assignment(
  proposal: Assignment,
  existing: Vec<Assignment>,
  state: State<'_, AppState>,
) -> AppResult<Validation> {
  state.with_database(false, false, |db| SchedulingService::new(db).validate(&proposal, &existing))
}
