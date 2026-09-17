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
  DoctorWorkRuleService::new(&state.database).list_rules(doctor_id)
}

#[tauri::command]
pub fn replace_doctor_work_rules(
  doctor_id: i64,
  rules: Vec<DoctorWorkRuleInput>,
  state: State<'_, AppState>,
) -> AppResult<Vec<DoctorWorkRule>> {
  DoctorWorkRuleService::new(&state.database).replace_rules(doctor_id, rules)
}

#[tauri::command]
pub fn create_doctor_work_rule(
  input: DoctorWorkRuleInput,
  state: State<'_, AppState>,
) -> AppResult<DoctorWorkRule> {
  DoctorWorkRuleService::new(&state.database).create_rule(input)
}

#[tauri::command]
pub fn delete_doctor_work_rule(id: i64, state: State<'_, AppState>) -> AppResult<()> {
  DoctorWorkRuleService::new(&state.database).delete_rule(id)
}

#[tauri::command]
pub fn get_doctor_rotation_groups(
  state: State<'_, AppState>,
) -> AppResult<Vec<RotationGroup>> {
  DoctorWorkRuleService::new(&state.database).list_rotation_groups()
}

#[tauri::command]
pub fn create_doctor_rotation_group(
  input: RotationGroupInput,
  state: State<'_, AppState>,
) -> AppResult<RotationGroup> {
  DoctorWorkRuleService::new(&state.database).create_rotation_group(input)
}
