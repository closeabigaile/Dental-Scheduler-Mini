use crate::database::AppState;
use crate::error::AppResult;
use crate::models::DatabaseInfo;
use tauri::State;

#[tauri::command]
pub fn get_database_info(state: State<'_, AppState>) -> AppResult<DatabaseInfo> {
  state.with_database(false, false, |db| db.info())
}

#[tauri::command]
pub fn check_database_integrity(state: State<'_, AppState>) -> AppResult<String> {
  state.with_database(false, false, |db| db.integrity_check())
}
