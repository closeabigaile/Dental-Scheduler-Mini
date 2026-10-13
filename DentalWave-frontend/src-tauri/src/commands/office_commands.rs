use crate::database::AppState;
use crate::error::AppResult;
use crate::models::{Office, OfficeInput};
use crate::services::office_service::OfficeService;
use tauri::State;

#[tauri::command]
pub fn get_offices(state: State<'_, AppState>) -> AppResult<Vec<Office>> {
  state.with_database(false, false, |db| OfficeService::new(db).list())
}

#[tauri::command]
pub fn get_office(id: i64, state: State<'_, AppState>) -> AppResult<Office> {
  state.with_database(false, false, |db| OfficeService::new(db).get(id))
}

#[tauri::command]
pub fn create_office(input: OfficeInput, state: State<'_, AppState>) -> AppResult<Office> {
  state.with_database(true, false, |db| OfficeService::new(db).create(input))
}

#[tauri::command]
pub fn update_office(
  id: i64,
  input: OfficeInput,
  state: State<'_, AppState>,
) -> AppResult<Office> {
  state.with_database(true, false, |db| OfficeService::new(db).update(id, input))
}

#[tauri::command]
pub fn delete_office(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  state.with_database(true, true, |db| OfficeService::new(db).delete(id))
}
