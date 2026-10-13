use crate::{
    database::AppState,
    error::AppResult,
    storage::{
        backup::Record,
        failure,
        location::{Candidate, Mode},
        Status,
    },
};
use std::path::PathBuf;
use tauri::State;
#[tauri::command]
pub fn storage_status(state: State<'_, AppState>) -> AppResult<Status> {
    Ok(state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .status())
}
#[tauri::command]
pub fn discover_databases(state: State<'_, AppState>) -> AppResult<Vec<Candidate>> {
    Ok(state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .locations
        .discover())
}
#[tauri::command]
pub fn open_database(
    state: State<'_, AppState>,
    mode: Mode,
    directory: Option<String>,
) -> AppResult<Status> {
    state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .open(mode, directory.map(PathBuf::from))
}
#[tauri::command]
pub fn initialize_portable_database(
    state: State<'_, AppState>,
    directory: String,
    source: Option<String>,
) -> AppResult<Status> {
    state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .initialize_portable(PathBuf::from(directory), source.map(PathBuf::from))
}
#[tauri::command]
pub fn close_database(state: State<'_, AppState>, without_backup: bool) -> AppResult<Status> {
    state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .close(without_backup)
}
#[tauri::command]
pub fn backup_database(state: State<'_, AppState>) -> AppResult<Record> {
    state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .manual_backup()
}
#[tauri::command]
pub fn list_database_backups(state: State<'_, AppState>) -> AppResult<Vec<Record>> {
    state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .backups()
}
#[tauri::command]
pub fn restore_database(state: State<'_, AppState>, filename: String) -> AppResult<Status> {
    state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .restore(&filename)
}
#[tauri::command]
pub fn diagnostic_report(state: State<'_, AppState>) -> AppResult<String> {
    Ok(state
        .storage
        .lock()
        .map_err(|_| failure("DATABASE_UNAVAILABLE"))?
        .diagnostics())
}
