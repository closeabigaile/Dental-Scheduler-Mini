use crate::database::AppState;
use crate::error::AppResult;
use crate::models::{
  DoctorWorkRule, DoctorWorkRuleInput, RotationGroup, RotationGroupInput,
};
use crate::services::doctor_work_rule_service::DoctorWorkRuleService;
use tauri::State;

#[tauri::command]
pub fn get_doctor_work_rules(
  doctor_id: i64,
  state: State<'_, AppState>,
) -> AppResult<Vec<DoctorWorkRule>> {
  state.with_database(false, false, |db| DoctorWorkRuleService::new(db).list_rules(doctor_id))
}

#[tauri::command]
pub fn replace_doctor_work_rules(
  doctor_id: i64,
  rules: Vec<DoctorWorkRuleInput>,
  state: State<'_, AppState>,
) -> AppResult<Vec<DoctorWorkRule>> {
  state.with_database(true, true, |db| DoctorWorkRuleService::new(db).replace_rules(doctor_id, rules))
}

#[tauri::command]
pub fn create_doctor_work_rule(
  input: DoctorWorkRuleInput,
  state: State<'_, AppState>,
) -> AppResult<DoctorWorkRule> {
  state.with_database(true, false, |db| DoctorWorkRuleService::new(db).create_rule(input))
}

#[tauri::command]
pub fn delete_doctor_work_rule(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  state.with_database(true, true, |db| DoctorWorkRuleService::new(db).delete_rule(id))
}

#[tauri::command]
pub fn get_doctor_rotation_groups(
  state: State<'_, AppState>,
) -> AppResult<Vec<RotationGroup>> {
  state.with_database(false, false, |db| DoctorWorkRuleService::new(db).list_rotation_groups())
}

#[tauri::command]
pub fn create_doctor_rotation_group(
  input: RotationGroupInput,
  state: State<'_, AppState>,
) -> AppResult<RotationGroup> {
  state.with_database(true, false, |db| DoctorWorkRuleService::new(db).create_rotation_group(input))
}
