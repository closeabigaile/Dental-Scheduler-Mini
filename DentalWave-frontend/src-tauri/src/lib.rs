mod commands;
mod database;
mod error;
mod models;
mod repositories;
mod services;

#[cfg(test)]
mod tests;

use database::{AppState, Database};
use error::AppError;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      let app_data_dir = app.path().app_data_dir().map_err(|error| {
        eprintln!("Could not resolve Tauri application data directory: {error}");
        Box::new(AppError::initialization_failed()) as Box<dyn std::error::Error>
      })?;
      let database = Database::new(
        app_data_dir
          .join("development")
          .join("scheduler.sqlite"),
      );
      database.initialize().map_err(|error| {
        eprintln!("Desktop database initialization failed: {error}");
        Box::new(error) as Box<dyn std::error::Error>
      })?;
      app.manage(AppState { database });
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
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
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
