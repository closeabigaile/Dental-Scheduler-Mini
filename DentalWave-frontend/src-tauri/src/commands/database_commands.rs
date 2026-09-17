use crate::database::AppState;
use crate::error::AppResult;
use crate::models::DatabaseInfo;
use tauri::State;

#[tauri::command]
pub fn get_database_info(state: State<'_, AppState>) -> AppResult<DatabaseInfo> {
  state.database.info()
}

#[tauri::command]
pub fn check_database_integrity(state: State<'_, AppState>) -> AppResult<String> {
  state.database.integrity_check()
}
