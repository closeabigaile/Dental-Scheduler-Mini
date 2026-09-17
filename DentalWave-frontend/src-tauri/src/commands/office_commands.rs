use crate::database::AppState;
use crate::error::AppResult;
use crate::models::{Office, OfficeInput};
use crate::services::office_service::OfficeService;
use tauri::State;

#[tauri::command]
pub fn get_offices(state: State<'_, AppState>) -> AppResult<Vec<Office>> {
  OfficeService::new(&state.database).list()
}

#[tauri::command]
pub fn get_office(id: i64, state: State<'_, AppState>) -> AppResult<Office> {
  OfficeService::new(&state.database).get(id)
}

#[tauri::command]
pub fn create_office(input: OfficeInput, state: State<'_, AppState>) -> AppResult<Office> {
  OfficeService::new(&state.database).create(input)
}

#[tauri::command]
pub fn update_office(
  id: i64,
  input: OfficeInput,
  state: State<'_, AppState>,
) -> AppResult<Office> {
  OfficeService::new(&state.database).update(id, input)
}

#[tauri::command]
pub fn delete_office(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  OfficeService::new(&state.database).delete(id)
}
