use crate::database::AppState;
use crate::error::AppResult;
use crate::models::{ResourceInput, ResourceKind, SchedulingResource};
use crate::services::resource_service::ResourceService;
use tauri::State;

#[tauri::command]
pub fn get_doctors(state: State<'_, AppState>) -> AppResult<Vec<SchedulingResource>> {
  state.with_database(false, false, |db| ResourceService::new(db).list(ResourceKind::Doctor))
}

#[tauri::command]
pub fn get_doctor(id: i64, state: State<'_, AppState>) -> AppResult<SchedulingResource> {
  state.with_database(false, false, |db| ResourceService::new(db).get(ResourceKind::Doctor, id))
}

#[tauri::command]
pub fn create_doctor(
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  state.with_database(true, false, |db| ResourceService::new(db).create(ResourceKind::Doctor, input))
}

#[tauri::command]
pub fn update_doctor(
  id: i64,
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  state.with_database(true, false, |db| ResourceService::new(db).update(ResourceKind::Doctor, id, input))
}

#[tauri::command]
pub fn delete_doctor(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  state.with_database(true, true, |db| ResourceService::new(db).delete(ResourceKind::Doctor, id))
}

#[tauri::command]
pub fn get_assistants(state: State<'_, AppState>) -> AppResult<Vec<SchedulingResource>> {
  state.with_database(false, false, |db| ResourceService::new(db).list(ResourceKind::Assistant))
}

#[tauri::command]
pub fn get_assistant(id: i64, state: State<'_, AppState>) -> AppResult<SchedulingResource> {
  state.with_database(false, false, |db| ResourceService::new(db).get(ResourceKind::Assistant, id))
}

#[tauri::command]
pub fn create_assistant(
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  state.with_database(true, false, |db| ResourceService::new(db).create(ResourceKind::Assistant, input))
}

#[tauri::command]
pub fn update_assistant(
  id: i64,
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  state.with_database(true, false, |db| ResourceService::new(db).update(ResourceKind::Assistant, id, input))
}

#[tauri::command]
pub fn delete_assistant(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  state.with_database(true, true, |db| ResourceService::new(db).delete(ResourceKind::Assistant, id))
}
