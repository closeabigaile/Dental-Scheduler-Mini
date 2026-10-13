use super::*;
use crate::{database::Database, services::calendar_service::CalendarService};

#[test]
fn migrated_copy_selection_never_creates_missing_files() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("missing.sqlite");
  assert!(Database::migrated_copy(path.clone()).is_err());
  assert!(!path.exists());
}

#[test]
fn migrated_copy_requires_provenance_and_compatible_schema() {
  let (_dir, db) = test_database();
  assert!(Database::migrated_copy(db.path().to_path_buf()).is_err());
  let c = db.connect().unwrap();
  c.execute_batch("CREATE TABLE migration_history(source_type TEXT,tool_version INTEGER); INSERT INTO migration_history VALUES('H2',1);").unwrap();
  assert!(Database::migrated_copy(db.path().to_path_buf()).is_ok());
  c.execute("UPDATE schema_metadata SET schema_version=999", [])
    .unwrap();
  assert!(Database::migrated_copy(db.path().to_path_buf()).is_err());
}

#[test]
fn existing_connection_does_not_recreate_deleted_database() {
  let (_dir, db) = test_database();
  std::fs::remove_file(db.path()).unwrap();
  assert!(db.connect().is_err());
  assert!(!db.path().exists());
}

#[test]
#[ignore = "Requires MIGRATION_TEST_SQLITE pointing to the synthetic Hibernate migration output"]
fn migrated_hibernate_fixture_edit_save_reload_generate() {
  let source = std::path::PathBuf::from(
    std::env::var_os("MIGRATION_TEST_SQLITE").expect("synthetic fixture path"),
  );
  let directory = tempfile::tempdir().unwrap();
  let target = directory.path().join("editable-copy.sqlite");
  // Test fixture is closed before copying; production migration never copies live SQLite.
  std::fs::copy(source, &target).unwrap();
  let db = Database::migrated_copy(target.clone()).unwrap();
  let service = CalendarService::new(&db);
  let mut month = service.list(Some(2026), Some(9)).unwrap();
  assert_eq!(month.len(), 2);
  assert_eq!(
    month[0].days[0].assignments[0].assistants[0].partial_day_note,
    "AM only"
  );
  month[0].days[0].notes = "Edited synthetic migrated day".into();
  let saved = service.save(2026, 9, month).unwrap();
  drop(db);
  let reopened = Database::migrated_copy(target).unwrap();
  assert_eq!(
    saved,
    CalendarService::new(&reopened)
      .list(Some(2026), Some(9))
      .unwrap()
  );
  let created = CalendarService::new(&reopened)
    .create(2026, 11, true)
    .unwrap();
  assert_eq!(created.calendars.len(), 2);
  assert!(created
    .calendars
    .iter()
    .flat_map(|c| &c.days)
    .any(|d| !d.assignments.is_empty()));
  assert_eq!(reopened.integrity_check().unwrap(), "ok");
}
