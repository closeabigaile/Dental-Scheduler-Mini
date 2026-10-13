mod commands;
mod domain;
mod database;
mod error;
mod models;
mod repositories;
mod services;
mod storage;
#[cfg(debug_assertions)]
pub mod migration_verification;
mod runtime;

#[cfg(all(not(debug_assertions), not(feature = "custom-protocol")))]
compile_error!("Production builds require embedded assets: build with the Tauri CLI or --features custom-protocol.");

#[cfg(test)]
mod tests;

use database::AppState;
use tauri::{Manager, Emitter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let built = tauri::Builder::default()
    .setup(runtime::setup)
    .on_window_event(|window,event| {
      if let tauri::WindowEvent::CloseRequested { api,.. }=event {
        if window.try_state::<AppState>().is_some_and(|state| state.storage.lock().map_or(true, |s| s.active())) {
          api.prevent_close();let _=window.emit("storage-close-requested",());
        }
      }
    })
    .invoke_handler(tauri::generate_handler![
      runtime::application_info,
      commands::storage_commands::storage_status,
      commands::storage_commands::discover_databases,
      commands::storage_commands::open_database,
      commands::storage_commands::initialize_portable_database,
      commands::storage_commands::close_database,
      commands::storage_commands::backup_database,
      commands::storage_commands::list_database_backups,
      commands::storage_commands::restore_database,
      commands::storage_commands::diagnostic_report,
      commands::calendar_commands::list_calendars,
      commands::calendar_commands::get_calendar,
      commands::calendar_commands::create_calendar_month,
      commands::calendar_commands::save_calendar_month,
      commands::calendar_commands::delete_calendar_month,
      commands::calendar_commands::print_calendar,

      commands::scheduling_commands::get_teams,
      commands::scheduling_commands::get_team,
      commands::scheduling_commands::create_team,
      commands::scheduling_commands::update_team,
      commands::scheduling_commands::duplicate_team,
      commands::scheduling_commands::delete_team,
      commands::scheduling_commands::preview_doctor_rules,
      commands::scheduling_commands::resolve_doctor_location,
      commands::scheduling_commands::validate_assignment,

      commands::database_commands::get_database_info,
      commands::database_commands::check_database_integrity,
      commands::office_commands::get_offices,
      commands::office_commands::get_office,
      commands::office_commands::create_office,
      commands::office_commands::update_office,
      commands::office_commands::delete_office,
      commands::resource_commands::get_doctors,
      commands::resource_commands::get_doctor,
      commands::resource_commands::create_doctor,
      commands::resource_commands::update_doctor,
      commands::resource_commands::delete_doctor,
      commands::resource_commands::get_assistants,
      commands::resource_commands::get_assistant,
      commands::resource_commands::create_assistant,
      commands::resource_commands::update_assistant,
      commands::resource_commands::delete_assistant,
      commands::doctor_work_rule_commands::get_doctor_work_rules,
      commands::doctor_work_rule_commands::replace_doctor_work_rules,
      commands::doctor_work_rule_commands::create_doctor_work_rule,
      commands::doctor_work_rule_commands::delete_doctor_work_rule,
      commands::doctor_work_rule_commands::get_doctor_rotation_groups,
      commands::doctor_work_rule_commands::create_doctor_rotation_group,
    ])
    .build(tauri::generate_context!());
  let app = match built {
    Ok(app) => app,
    Err(_) => { runtime::show_startup_failure(); return; }
  };
  app.run(|app,event| {
      if let tauri::RunEvent::ExitRequested {api,..}=event {
        if app.try_state::<AppState>().is_some_and(|state| state.storage.lock().map_or(true, |s| s.active())) {
          api.prevent_exit();let _=app.emit("storage-close-requested",());
        }
      }
    });
}
