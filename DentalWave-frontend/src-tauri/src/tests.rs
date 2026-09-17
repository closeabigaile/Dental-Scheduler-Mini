use crate::database::Database;
use crate::models::{
  DoctorWorkRuleInput, IdReference, OfficeInput, ResourceInput, ResourceKind,
  RotationGroupInput,
};
use crate::repositories::doctor_work_rule_repository::DoctorWorkRuleRepository;
use crate::services::doctor_work_rule_service::DoctorWorkRuleService;
use crate::services::office_service::OfficeService;
use crate::services::resource_service::ResourceService;
use tempfile::TempDir;

fn test_database() -> (TempDir, Database) {
  let directory = tempfile::tempdir().expect("temporary test directory");
  let database = Database::new(directory.path().join("development/scheduler.sqlite"));
  database.initialize().expect("initialize test database");
  (directory, database)
}

fn office_input(name: &str) -> OfficeInput {
  OfficeInput {
    name: name.to_string(),
    address: "123 Test Street".to_string(),
    phone_number: "555-0100".to_string(),
  }
}

fn resource_input(kind: ResourceKind, name: &str, office_id: Option<i64>) -> ResourceInput {
  let office = office_id.map(|id| IdReference { id });
  ResourceInput {
    resource_type: Some(kind.as_str().to_string()),
    first_name: String::new(),
    last_name: String::new(),
    display_name: name.to_string(),
    active: true,
    default_office: office.clone(),
    offices: office.into_iter().collect(),
    normal_workdays: None,
    color: "#65a9b8".to_string(),
    notes: "Development test record".to_string(),
  }
}

fn weekly_rule(doctor_id: i64, office_id: i64, day: &str) -> DoctorWorkRuleInput {
  DoctorWorkRuleInput {
    doctor: IdReference { id: doctor_id },
    day_of_week: day.to_string(),
    office: Some(IdReference { id: office_id }),
    work_status: "WORKING".to_string(),
    recurrence_type: "EVERY_WEEK".to_string(),
    rotation_group: None,
    rotation_position: None,
    effective_start_date: None,
    effective_end_date: None,
    active: true,
  }
}

#[test]
fn initializes_versioned_schema_and_passes_integrity_check() {
  let (_directory, database) = test_database();
  let info = database.info().expect("database info");
  assert_eq!(info.schema_version, 1);
  assert_eq!(info.integrity_status, "ok");
  assert!(info.path.ends_with("development/scheduler.sqlite"));
}

#[test]
fn rejects_a_schema_newer_than_the_application_supports() {
  let (_directory, database) = test_database();
  let connection = rusqlite::Connection::open(database.path()).expect("open raw test database");
  connection
    .execute("UPDATE schema_metadata SET schema_version = 2 WHERE id = 1", [])
    .expect("set future schema version");
  drop(connection);

  let error = database.initialize().expect_err("future schema must be rejected");
  assert_eq!(error.code, "INCOMPATIBLE_SCHEMA_VERSION");
}

#[test]
fn office_crud_uses_parameterized_persistent_storage() {
  let (_directory, database) = test_database();
  let service = OfficeService::new(&database);

  let created = service
    .create(office_input("Raleigh Test Office"))
    .expect("create office");
  assert_eq!(service.get(created.id).expect("read office"), created);

  let updated = service
    .update(
      created.id,
      OfficeInput {
        name: "Garner Test Office".to_string(),
        address: "456 Updated Street".to_string(),
        phone_number: "555-0199".to_string(),
      },
    )
    .expect("update office");
  assert_eq!(updated.name, "Garner Test Office");
  assert_eq!(service.list().expect("list offices").len(), 1);

  service.delete(created.id).expect("delete office");
  assert_eq!(
    service.get(created.id).expect_err("deleted office").code,
    "RECORD_NOT_FOUND"
  );
}

#[test]
fn doctor_crud_preserves_office_relationships_and_status() {
  let (_directory, database) = test_database();
  let office = OfficeService::new(&database)
    .create(office_input("Raleigh Test Office"))
    .expect("create office");
  let service = ResourceService::new(&database);

  let doctor = service
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test Doctor A", Some(office.id)),
    )
    .expect("create doctor");
  assert_eq!(doctor.resource_type, "DOCTOR");
  assert_eq!(doctor.default_office.as_ref().map(|value| value.id), Some(office.id));
  assert_eq!(doctor.offices.len(), 1);

  let mut update = resource_input(ResourceKind::Doctor, "Test Doctor Updated", Some(office.id));
  update.active = false;
  update.notes = "Updated note".to_string();
  let updated = service
    .update(ResourceKind::Doctor, doctor.id, update)
    .expect("update doctor");
  assert!(!updated.active);
  assert_eq!(updated.display_name, "Test Doctor Updated");
  assert_eq!(service.list(ResourceKind::Doctor).expect("list doctors").len(), 1);

  service
    .delete(ResourceKind::Doctor, doctor.id)
    .expect("delete doctor");
  assert_eq!(
    service
      .get(ResourceKind::Doctor, doctor.id)
      .expect_err("deleted doctor")
      .code,
    "RECORD_NOT_FOUND"
  );
}

#[test]
fn assistant_crud_preserves_office_relationships_and_status() {
  let (_directory, database) = test_database();
  let office = OfficeService::new(&database)
    .create(office_input("Garner Test Office"))
    .expect("create office");
  let service = ResourceService::new(&database);

  let assistant = service
    .create(
      ResourceKind::Assistant,
      resource_input(
        ResourceKind::Assistant,
        "Test Assistant A",
        Some(office.id),
      ),
    )
    .expect("create assistant");
  assert_eq!(assistant.resource_type, "ASSISTANT");

  let mut update = resource_input(
    ResourceKind::Assistant,
    "Test Assistant Updated",
    Some(office.id),
  );
  update.active = false;
  let updated = service
    .update(ResourceKind::Assistant, assistant.id, update)
    .expect("update assistant");
  assert!(!updated.active);
  assert_eq!(service.list(ResourceKind::Assistant).expect("list assistants").len(), 1);

  service
    .delete(ResourceKind::Assistant, assistant.id)
    .expect("delete assistant");
  assert_eq!(
    service
      .get(ResourceKind::Assistant, assistant.id)
      .expect_err("deleted assistant")
      .code,
    "RECORD_NOT_FOUND"
  );
}

#[test]
fn validates_required_entity_input() {
  let (_directory, database) = test_database();
  let office_error = OfficeService::new(&database)
    .create(office_input("   "))
    .expect_err("blank office name");
  assert_eq!(office_error.code, "INVALID_INPUT");

  let resource_error = ResourceService::new(&database)
    .create(
      ResourceKind::Assistant,
      resource_input(ResourceKind::Assistant, " ", None),
    )
    .expect_err("blank assistant name");
  assert_eq!(resource_error.code, "INVALID_INPUT");
}

#[test]
fn foreign_keys_prevent_deleting_a_referenced_office() {
  let (_directory, database) = test_database();
  let office_service = OfficeService::new(&database);
  let office = office_service
    .create(office_input("Referenced Test Office"))
    .expect("create office");
  ResourceService::new(&database)
    .create(
      ResourceKind::Assistant,
      resource_input(
        ResourceKind::Assistant,
        "Test Assistant A",
        Some(office.id),
      ),
    )
    .expect("create assistant");

  let error = office_service
    .delete(office.id)
    .expect_err("referenced office cannot be deleted");
  assert_eq!(error.code, "RECORD_CONFLICT");
  assert_eq!(office_service.get(office.id).expect("office remains").id, office.id);
}

#[test]
fn doctor_work_rule_replacement_rolls_back_on_failure() {
  let (_directory, database) = test_database();
  let office = OfficeService::new(&database)
    .create(office_input("Pattern Test Office"))
    .expect("create office");
  let doctor = ResourceService::new(&database)
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test Doctor A", Some(office.id)),
    )
    .expect("create doctor");
  let rule_service = DoctorWorkRuleService::new(&database);
  rule_service
    .replace_rules(doctor.id, vec![weekly_rule(doctor.id, office.id, "MONDAY")])
    .expect("store initial rule");

  let mut connection = database.connect().expect("open database");
  let result = DoctorWorkRuleRepository::replace_rules(
    &mut connection,
    doctor.id,
    &[
      weekly_rule(doctor.id, office.id, "TUESDAY"),
      weekly_rule(doctor.id, 999_999, "WEDNESDAY"),
    ],
  );
  assert!(result.is_err());
  let stored = DoctorWorkRuleRepository::list_rules(&connection, doctor.id)
    .expect("read rules after rollback");
  assert_eq!(stored.len(), 1);
  assert_eq!(stored[0].day_of_week, "MONDAY");
}

#[test]
fn rotation_groups_and_doctor_patterns_are_persisted_as_data_only() {
  let (_directory, database) = test_database();
  let office = OfficeService::new(&database)
    .create(office_input("Rotation Test Office"))
    .expect("create office");
  let doctor = ResourceService::new(&database)
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test Doctor A", Some(office.id)),
    )
    .expect("create doctor");
  let service = DoctorWorkRuleService::new(&database);
  let rotation = service
    .create_rotation_group(RotationGroupInput {
      name: "Standard alternating weeks".to_string(),
      number_of_weeks: 2,
      anchor_date: "2024-01-01".to_string(),
      active: true,
    })
    .expect("create rotation group");
  let rules = service
    .replace_rules(
      doctor.id,
      vec![DoctorWorkRuleInput {
        doctor: IdReference { id: doctor.id },
        day_of_week: "THURSDAY".to_string(),
        office: Some(IdReference { id: office.id }),
        work_status: "WORKING".to_string(),
        recurrence_type: "ROTATING".to_string(),
        rotation_group: Some(IdReference { id: rotation.id }),
        rotation_position: Some(1),
        effective_start_date: None,
        effective_end_date: None,
        active: true,
      }],
    )
    .expect("store rotating rule");
  assert_eq!(rules[0].rotation_group.as_ref().map(|value| value.id), Some(rotation.id));
}

#[test]
fn data_persists_after_database_is_closed_and_reopened() {
  let (directory, database) = test_database();
  let created = OfficeService::new(&database)
    .create(office_input("Persistent Test Office"))
    .expect("create persistent office");
  let path = database.path().to_path_buf();
  drop(database);

  let reopened = Database::new(path);
  reopened.initialize().expect("reopen database");
  assert_eq!(
    OfficeService::new(&reopened)
      .get(created.id)
      .expect("persistent office")
      .name,
    "Persistent Test Office"
  );
  drop(directory);
}
