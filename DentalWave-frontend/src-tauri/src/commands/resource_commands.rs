use crate::database::AppState;
use crate::error::AppResult;
use crate::models::{ResourceInput, ResourceKind, SchedulingResource};
use crate::services::resource_service::ResourceService;
use tauri::State;

#[tauri::command]
pub fn get_doctors(state: State<'_, AppState>) -> AppResult<Vec<SchedulingResource>> {
  ResourceService::new(&state.database).list(ResourceKind::Doctor)
}

#[tauri::command]
pub fn get_doctor(id: i64, state: State<'_, AppState>) -> AppResult<SchedulingResource> {
  ResourceService::new(&state.database).get(ResourceKind::Doctor, id)
}

#[tauri::command]
pub fn create_doctor(
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  ResourceService::new(&state.database).create(ResourceKind::Doctor, input)
}

#[tauri::command]
pub fn update_doctor(
  id: i64,
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  ResourceService::new(&state.database).update(ResourceKind::Doctor, id, input)
}

#[tauri::command]
pub fn delete_doctor(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  ResourceService::new(&state.database).delete(ResourceKind::Doctor, id)
}

#[tauri::command]
pub fn get_assistants(state: State<'_, AppState>) -> AppResult<Vec<SchedulingResource>> {
  ResourceService::new(&state.database).list(ResourceKind::Assistant)
}

#[tauri::command]
pub fn get_assistant(id: i64, state: State<'_, AppState>) -> AppResult<SchedulingResource> {
  ResourceService::new(&state.database).get(ResourceKind::Assistant, id)
}

#[tauri::command]
pub fn create_assistant(
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  ResourceService::new(&state.database).create(ResourceKind::Assistant, input)
}

#[tauri::command]
pub fn update_assistant(
  id: i64,
  input: ResourceInput,
  state: State<'_, AppState>,
) -> AppResult<SchedulingResource> {
  ResourceService::new(&state.database).update(ResourceKind::Assistant, id, input)
}

#[tauri::command]
pub fn delete_assistant(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  ResourceService::new(&state.database).delete(ResourceKind::Assistant, id)
}
