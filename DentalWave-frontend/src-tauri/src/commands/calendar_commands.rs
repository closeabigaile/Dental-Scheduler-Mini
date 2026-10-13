use crate::{database::AppState,error::{AppError,AppResult},models::calendar::{Calendar,CalendarResult},services::calendar_service::CalendarService};
use tauri::State;
#[tauri::command]
pub fn list_calendars(state:State<'_,AppState>,year:Option<i32>,month:Option<u32>)->AppResult<Vec<Calendar>>{
  state.with_database(false, false, |db| CalendarService::new(db).list(year,month))
}
#[tauri::command]
pub fn get_calendar(state:State<'_,AppState>,id:i64)->AppResult<Calendar>{
  state.with_database(false, false, |db| CalendarService::new(db).get(id))
}
#[tauri::command]
pub fn create_calendar_month(state:State<'_,AppState>,year:i32,month:u32,generate:bool)->AppResult<CalendarResult>{
  state.with_database(true, false, |db| CalendarService::new(db).create(year,month,generate))
}
#[tauri::command]
pub fn save_calendar_month(state:State<'_,AppState>,year:i32,month:u32,calendars:Vec<Calendar>)->AppResult<Vec<Calendar>>{
  state.with_database(true, false, |db| CalendarService::new(db).save(year,month,calendars))
}
#[tauri::command]
pub fn delete_calendar_month(state:State<'_,AppState>,year:i32,month:u32,calendars:Vec<Calendar>)->AppResult<()>{
  state.with_database(true, true, |db| CalendarService::new(db).delete(year,month,calendars))
}
#[tauri::command]
pub fn print_calendar(window:tauri::WebviewWindow)->AppResult<()> {
 #[cfg(target_os = "macos")]
 let result = window.print();
 #[cfg(not(target_os = "macos"))]
 let result = window.eval("window.print()");
 result.map_err(|_|AppError::new("PRINT_PREVIEW_FAILED","The print dialog could not be opened. Your saved schedule has not changed."))
}
