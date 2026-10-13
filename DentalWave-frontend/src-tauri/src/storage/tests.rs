use super::*;
use crate::{
    database::migrations,
    models::{OfficeInput, ResourceKind},
    services::{
        calendar_service::CalendarService, doctor_work_rule_service::DoctorWorkRuleService,
        office_service::OfficeService, resource_service::ResourceService,
        team_service::TeamService,
    },
};
fn manager(root: &Path) -> StorageManager {
    StorageManager::new(
        DatabaseLocationService::new(root.join("local"), root.join("logs"), Mode::Portable)
            .unwrap(),
    )
}
fn setup() -> (tempfile::TempDir, StorageManager, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let mut m = manager(root.path());
    let dir = root.path().join("Dental Scheduler Mini Data");
    m.initialize_portable(dir.clone(), None).unwrap();
    (root, m, dir)
}
fn office(m: &mut StorageManager, name: &str) {
    m.with_database(true, false, |db| {
        OfficeService::new(db).create(OfficeInput {
            name: name.into(),
            address: "".into(),
            phone_number: "".into(),
        })
    })
    .unwrap();
}
fn count(m: &mut StorageManager) -> usize {
    m.with_database(false, false, |db| OfficeService::new(db).list())
        .unwrap()
        .len()
}
#[test]
fn usb_package_follows_executable_after_move_and_ignores_host_preference() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("USB D with spaces");
    fs::create_dir(&package).unwrap();
    fs::write(package.join(location::PORTABLE_MARKER), r#"{"formatVersion":1,"dataFolder":"DentalSchedulerMiniData"}"#).unwrap();
    let mut m = manager(root.path());
    let directory = location::package_directory(&package.join("DentalSchedulerMini.exe")).unwrap().unwrap();
    assert!(!directory.exists());
    m.initialize_portable(directory, None).unwrap();
    office(&mut m, "Travels with USB");
    let original_id = m.status().data_id.unwrap();
    m.close(false).unwrap();
    m.open(Mode::Local, None).unwrap();
    office(&mut m, "Host computer only");
    m.close(false).unwrap();
    let moved = root.path().join("USB E");
    fs::rename(package, &moved).unwrap();
    m.locations.portable_package_directory = location::package_directory(&moved.join("DentalSchedulerMini.exe")).unwrap();
    let reopened = m.open(Mode::Portable, None).unwrap();
    assert_eq!(reopened.data_id.as_deref(), Some(original_id.as_str()));
    assert_eq!(reopened.directory, Some(moved.join(location::FOLDER).to_string_lossy().into_owned()));
    let offices = m.with_database(false, false, |db| OfficeService::new(db).list()).unwrap();
    assert_eq!(offices[0].name, "Travels with USB");
    m.close(false).unwrap();
    fs::rename(moved.join(location::FOLDER), moved.join("preserved-data")).unwrap();
    assert!(m.open(Mode::Portable, None).is_err());
    assert!(!m.active());
    assert!(!moved.join(location::FOLDER).exists());
}

#[test]
fn usb_package_marker_is_optional_but_malformed_markers_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("DentalSchedulerMini.exe");
    assert!(location::package_directory(&executable).unwrap().is_none());
    for contents in ["invalid", r#"{"formatVersion":2,"dataFolder":"DentalSchedulerMiniData"}"#, r#"{"formatVersion":1,"dataFolder":"../elsewhere"}"#] {
        fs::write(root.path().join(location::PORTABLE_MARKER), contents).unwrap();
        assert_eq!(location::package_directory(&executable).unwrap_err().code, "INVALID_PORTABLE_PACKAGE");
    }
    assert!(!root.path().join(location::FOLDER).exists());
}

#[test]
fn missing_never_creates_portable_or_falls_back() {
    let root = tempfile::tempdir().unwrap();
    let mut m = manager(root.path());
    assert_eq!(
        m.open(Mode::Portable, Some(root.path().join("missing")))
            .unwrap_err()
            .code,
        "INVALID_DATA_METADATA"
    );
    assert!(!root.path().join("missing").exists());
    assert!(!m.locations.local_path().exists());
    assert_eq!(m.locations.choose(&[]).unwrap_err().code, "USB_NOT_FOUND");
}
#[test]
fn local_mode_still_initializes_and_saves() {
    let root = tempfile::tempdir().unwrap();
    let mut m = manager(root.path());
    m.open(Mode::Local, None).unwrap();
    office(&mut m, "Local");
    m.close(false).unwrap();
    m.open(Mode::Local, None).unwrap();
    assert_eq!(count(&mut m), 1);
    assert!(root.path().join("logs/storage.log").exists());
}
#[test]
fn discovery_remount_spaces_multiple_and_remembered_id() {
    let (root, mut m, dir) = setup();
    let id = m.status().data_id.unwrap();
    m.close(false).unwrap();
    let mut current = dir;
    for letter in ["D", "E", "F"] {
        let volume = root.path().join(letter);
        fs::create_dir(&volume).unwrap();
        let next = volume.join("Dental Scheduler Mini Data");
        fs::rename(current, &next).unwrap();
        let found = DatabaseLocationService::discover_on(&[volume]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].data_id, id);
        assert_eq!(m.locations.choose(&found).unwrap(), next);
        m.open(Mode::Portable, Some(next.clone())).unwrap();
        m.close(false).unwrap();
        current = next;
    }
    let other = root.path().join("other");
    fs::create_dir(&other).unwrap();
    m.initialize_portable(other.join(location::FOLDER), None)
        .unwrap();
    m.close(false).unwrap();
    m.locations.settings.preferred_id = None;
    let found = DatabaseLocationService::discover_on(&[root.path().join("F"), other]);
    assert_eq!(
        m.locations.choose(&found).unwrap_err().code,
        "MULTIPLE_DATABASES_FOUND"
    );
    m.locations.settings.preferred_id = Some(id);
    assert_eq!(m.locations.choose(&found).unwrap(), current);
}
#[test]
fn missing_database_has_recovery_session_without_creation() {
    let (_r, mut m, dir) = setup();
    m.close(false).unwrap();
    fs::remove_file(dir.join(DATABASE)).unwrap();
    let s = m.open(Mode::Portable, Some(dir.clone())).unwrap();
    assert_eq!(s.error_code.as_deref(), Some("PORTABLE_DATABASE_NOT_FOUND"));
    assert!(!dir.join(DATABASE).exists());
    let backup = m.backups().unwrap()[0].filename.clone();
    m.restore(&backup).unwrap();
    assert_eq!(count(&mut m), 0);
}
#[test]
fn readonly_viewing_blocks_backend_writes() {
    let (_r, mut m, dir) = setup();
    office(&mut m, "Office");
    m.close(false).unwrap();
    let path = dir.join(DATABASE);
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&path, perms).unwrap();
    assert_eq!(
        m.open(Mode::Portable, Some(dir)).unwrap().access,
        "READ_ONLY"
    );
    assert_eq!(count(&mut m), 1);
    assert_eq!(
        m.with_database(true, false, |_| Ok(())).unwrap_err().code,
        "DATABASE_READ_ONLY"
    );
    m.close(false).unwrap();
}
#[test]
fn second_session_lock_released_on_close() {
    let (root, mut m, dir) = setup();
    let mut second = manager(&root.path().join("another"));
    assert_eq!(
        second
            .open(Mode::Portable, Some(dir.clone()))
            .unwrap_err()
            .code,
        "DATABASE_ALREADY_IN_USE"
    );
    m.close(false).unwrap();
    second.open(Mode::Portable, Some(dir)).unwrap();
}
#[test]
fn removal_before_save_is_latched_and_does_not_replace_data() {
    let (root, mut m, dir) = setup();
    fs::rename(&dir, root.path().join("removed")).unwrap();
    assert_eq!(
        m.with_database(true, false, |_| -> AppResult<()> {
            panic!("must not save")
        })
        .unwrap_err()
        .code,
        "PORTABLE_STORAGE_REMOVED"
    );
    fs::rename(root.path().join("removed"), &dir).unwrap();
    assert!(m.with_database(true, false, |_| Ok(())).is_err());
    assert!(!m.locations.local_path().exists());
    m.close(true).unwrap();
    m.open(Mode::Portable, Some(dir)).unwrap();
}
#[test]
fn removal_during_save_never_acknowledges_success() {
    let (root, mut m, dir) = setup();
    let result = m.with_database(true, false, |db| {
        let mut c = db.connect()?;
        let tx = c.transaction().map_err(AppError::from_write_error)?;
        tx.execute("INSERT INTO offices(name) VALUES('During')", [])
            .map_err(AppError::from_write_error)?;
        fs::rename(&dir, root.path().join("removed")).unwrap();
        tx.commit().map_err(AppError::from_write_error)
    });
    assert!(result.is_err());
    assert_eq!(m.status().access, "UNAVAILABLE");
    m.close(true).unwrap();
    fs::rename(root.path().join("removed"), &dir).unwrap();
    m.open(Mode::Portable, Some(dir.clone())).unwrap();
    assert_eq!(validate::inspect(&dir.join(DATABASE)).unwrap(), 3);
    assert!(count(&mut m) <= 1);
}
#[test]
fn replaced_file_identity_is_rejected() {
    let (_r, mut m, dir) = setup();
    fs::rename(dir.join(DATABASE), dir.join("old.sqlite")).unwrap();
    Database::session(dir.join(DATABASE), false)
        .initialize()
        .unwrap();
    assert_eq!(
        m.status().error_code.as_deref(),
        Some("PORTABLE_STORAGE_REMOVED")
    );
}
#[test]
fn newer_random_and_fk_damage_rejected_before_writes() {
    let (_r, mut m, dir) = setup();
    m.close(false).unwrap();
    let path = dir.join(DATABASE);
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE schema_metadata SET schema_version=999", [])
        .unwrap();
    drop(c);
    let before = fs::read(&path).unwrap();
    assert_eq!(
        m.open(Mode::Portable, Some(dir.clone())).unwrap_err().code,
        "DATABASE_VERSION_TOO_NEW"
    );
    assert_eq!(before, fs::read(&path).unwrap());
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE schema_metadata SET schema_version=3", [])
        .unwrap();
    c.execute("PRAGMA foreign_keys=OFF", []).unwrap();
    c.execute(
        "INSERT INTO doctors(display_name,default_office_id) VALUES('Bad',42)",
        [],
    )
    .unwrap();
    drop(c);
    assert_eq!(
        validate::inspect(&path).unwrap_err().code,
        "DATABASE_DAMAGED"
    );
    assert_eq!(
        m.open(Mode::Portable, Some(dir)).unwrap().access,
        "UNAVAILABLE"
    );
}
#[test]
fn older_upgrade_is_backed_up_first() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(location::FOLDER);
    fs::create_dir(&dir).unwrap();
    let path = dir.join(DATABASE);
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(migrations::VERSION_1).unwrap();
    drop(c);
    location::atomic_json(&dir.join(location::METADATA), &Metadata::fresh()).unwrap();
    let mut m = manager(root.path());
    m.open(Mode::Portable, Some(dir)).unwrap();
    let records = m.backups().unwrap();
    assert!(records
        .iter()
        .any(|r| r.reason == "before-upgrade" && r.schema_version == 1));
    assert_eq!(m.status().schema_version, Some(3));
}
#[test]
fn automatic_manual_destructive_and_close_backups_are_verified() {
    let (_r, mut m, dir) = setup();
    assert_eq!(m.backups().unwrap().len(), 1);
    office(&mut m, "Saved");
    let backup = m.manual_backup().unwrap();
    assert_eq!(
        validate::inspect(&dir.join("Backups").join(backup.filename)).unwrap(),
        3
    );
    m.with_database(true, true, |db| OfficeService::new(db).delete(1))
        .unwrap();
    assert!(m
        .backups()
        .unwrap()
        .iter()
        .any(|b| b.reason == "before-destructive-change"));
    m.close(false).unwrap();
    m.open(Mode::Portable, Some(dir)).unwrap();
    assert!(m.backups().unwrap().iter().any(|b| b.reason == "close"));
}
#[test]
fn restore_preserves_current_and_reloads_contents() {
    let (_r, mut m, dir) = setup();
    office(&mut m, "First");
    let b = m.manual_backup().unwrap();
    office(&mut m, "Second");
    let old = m.status().session_key;
    m.restore(&b.filename).unwrap();
    assert_ne!(m.status().session_key, old);
    assert_eq!(count(&mut m), 1);
    assert!(m
        .backups()
        .unwrap()
        .iter()
        .any(|b| b.reason == "before-restore"));
    assert!(fs::read_dir(&dir).unwrap().any(|p| p
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("preserved-")));
    m.close(false).unwrap();
    m.open(Mode::Portable, Some(dir)).unwrap();
    assert_eq!(count(&mut m), 1);
}
#[test]
fn retention_never_deletes_unknown_or_invalid_backups() {
    let (_r, mut m, dir) = setup();
    let backups = dir.join("Backups");
    fs::write(backups.join("unknown.sqlite"), b"keep").unwrap();
    let invalid = m.manual_backup().unwrap();
    fs::write(backups.join(&invalid.filename), b"damaged").unwrap();
    for _ in 0..22 {
        m.manual_backup().unwrap();
    }
    assert_eq!(m.backups().unwrap().len(), backup::RETENTION);
    assert!(backups.join("unknown.sqlite").exists());
    assert!(backups.join(invalid.filename).exists());
    backup::retain(&backups, &m.status().data_id.unwrap(), 0).unwrap();
    assert_eq!(m.backups().unwrap().len(), 1);
}
#[test]
fn invalid_and_newer_restore_do_not_touch_original() {
    let (_r, mut m, dir) = setup();
    office(&mut m, "Keep");
    let before = fs::read(dir.join(DATABASE)).unwrap();
    assert_eq!(
        m.restore("../scheduler.sqlite").unwrap_err().code,
        "INVALID_BACKUP"
    );
    assert_eq!(before, fs::read(dir.join(DATABASE)).unwrap());
    let b = m.backups().unwrap()[0].clone();
    let c = rusqlite::Connection::open(dir.join("Backups").join(&b.filename)).unwrap();
    c.execute("UPDATE schema_metadata SET schema_version=999", [])
        .unwrap();
    drop(c);
    assert!(m.restore(&b.filename).is_err());
    assert_eq!(before, fs::read(dir.join(DATABASE)).unwrap());
}
#[test]
fn interrupted_restore_requires_deliberate_recovery() {
    let (_r, mut m, dir) = setup();
    office(&mut m, "Before");
    let b = m.manual_backup().unwrap();
    m.close(false).unwrap();
    fs::rename(
        dir.join(DATABASE),
        dir.join("preserved-interruption.sqlite"),
    )
    .unwrap();
    fs::write(dir.join(".restore-pending.json"), b"{}").unwrap();
    assert_eq!(
        m.open(Mode::Portable, Some(dir.clone()))
            .unwrap()
            .error_code
            .as_deref(),
        Some("RESTORE_INTERRUPTED")
    );
    assert!(m.with_database(true, false, |_| Ok(())).is_err());
    m.restore(&b.filename).unwrap();
    assert_eq!(count(&mut m), 1);
    assert!(dir.join("preserved-interruption.sqlite").exists());
    assert!(!dir.join(".restore-pending.json").exists());
}
#[test]
fn damaged_original_is_preserved_during_restore() {
    let (_r, mut m, dir) = setup();
    let b = m.manual_backup().unwrap();
    m.close(false).unwrap();
    fs::write(dir.join(DATABASE), b"damaged original").unwrap();
    m.open(Mode::Portable, Some(dir.clone())).unwrap();
    m.restore(&b.filename).unwrap();
    let preserved = fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().starts_with("preserved-"))
        .unwrap();
    assert_eq!(fs::read(preserved.path()).unwrap(), b"damaged original");
    assert_eq!(count(&mut m), 0);
}
#[test]
fn diagnostic_report_contains_no_schedule_or_paths() {
    let (_r, mut m, dir) = setup();
    office(&mut m, "PRIVATE NAME");
    let report = m.diagnostics();
    assert!(!report.contains("PRIVATE NAME"));
    assert!(!report.contains(dir.to_str().unwrap()));
    assert!(report.contains("schemaVersion"));
    assert!(!dir.join("storage.log").exists());
}
#[test]
fn explicit_initialization_never_overwrites_folder() {
    let (root, mut m, dir) = setup();
    m.close(false).unwrap();
    let before = fs::read(dir.join(DATABASE)).unwrap();
    assert_eq!(
        m.initialize_portable(dir.clone(), None).unwrap_err().code,
        "INITIALIZATION_TARGET_EXISTS"
    );
    assert_eq!(before, fs::read(dir.join(DATABASE)).unwrap());
    assert!(!root.path().join("scheduler.sqlite").exists());
}
#[test]
fn journaling_is_delete_extra_and_foreign_keys_enabled() {
    let (_r, mut m, _dir) = setup();
    m.with_database(false, false, |db| {
        let c = db.connect()?;
        assert_eq!(
            c.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "delete"
        );
        assert_eq!(
            c.query_row("PRAGMA synchronous", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            c.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        Ok(())
    })
    .unwrap();
}
#[test]
#[ignore = "Requires MIGRATION_TEST_SQLITE synthetic Phase 5 output"]
fn migrated_copy_full_portable_cycle() {
    let root = tempfile::tempdir().unwrap();
    let mut m = manager(root.path());
    let dir = root.path().join(location::FOLDER);
    m.initialize_portable(
        dir.clone(),
        Some(std::env::var_os("MIGRATION_TEST_SQLITE").unwrap().into()),
    )
    .unwrap();
    m.with_database(false, false, |db| {
        assert_eq!(OfficeService::new(db).list()?.len(), 2);
        assert!(!ResourceService::new(db)
            .list(ResourceKind::Doctor)?
            .is_empty());
        assert!(!ResourceService::new(db)
            .list(ResourceKind::Assistant)?
            .is_empty());
        assert!(!TeamService::new(db).list()?.is_empty());
        assert!(!DoctorWorkRuleService::new(db)
            .list_rules(ResourceService::new(db).list(ResourceKind::Doctor)?[0].id)?
            .is_empty());
        Ok(())
    })
    .unwrap();
    let original = m.manual_backup().unwrap();
    m.with_database(true, false, |db| {
        let svc = CalendarService::new(db);
        let mut month = svc.list(Some(2026), Some(9))?;
        month[0].days[0].notes = "Portable edited".into();
        svc.save(2026, 9, month)?;
        Ok(())
    })
    .unwrap();
    m.close(false).unwrap();
    m.open(Mode::Portable, Some(dir)).unwrap();
    m.with_database(false, false, |db| {
        assert_eq!(
            CalendarService::new(db).list(Some(2026), Some(9))?[0].days[0].notes,
            "Portable edited"
        );
        Ok(())
    })
    .unwrap();
    m.restore(&original.filename).unwrap();
    m.with_database(false, false, |db| {
        assert_ne!(
            CalendarService::new(db).list(Some(2026), Some(9))?[0].days[0].notes,
            "Portable edited"
        );
        Ok(())
    })
    .unwrap();
}
#[test]
fn crash_child() {
    let Some(dir) = std::env::var_os("DENTAL_STORAGE_CRASH_CHILD") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let mut m = manager(&dir.join("child-local"));
    m.open(Mode::Portable, Some(dir.clone())).unwrap();
    let c = m.session.as_ref().unwrap().db.connect().unwrap();
    c.execute_batch("PRAGMA cache_size=10; BEGIN IMMEDIATE;")
        .unwrap();
    for index in 0..500 {
        c.execute(
            "INSERT INTO offices(name,address) VALUES(?1,?2)",
            rusqlite::params![format!("Uncommitted {index}"), "X".repeat(8000)],
        )
        .unwrap();
    }
    fs::write(dir.join("child-ready"), b"ready").unwrap();
    std::thread::sleep(std::time::Duration::from_secs(60));
    panic!("parent should terminate child");
}
#[test]
fn process_termination_releases_lock_and_recovers_hot_journal() {
    let (root, mut m, dir) = setup();
    office(&mut m, "Committed before crash");
    m.close(false).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "storage::tests::crash_child", "--nocapture"])
        .env("DENTAL_STORAGE_CRASH_CHILD", &dir)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !dir.join("child-ready").exists() {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("child did not acquire session");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let mut second = manager(&root.path().join("second-process"));
    assert_eq!(
        second
            .open(Mode::Portable, Some(dir.clone()))
            .unwrap_err()
            .code,
        "DATABASE_ALREADY_IN_USE"
    );
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(dir.join("scheduler.sqlite-journal").exists());
    let status = m.open(Mode::Portable, Some(dir.clone())).unwrap();
    assert_eq!(status.access, "READABLE_AND_WRITABLE", "{:?}", status);
    assert_eq!(count(&mut m), 1);
    assert_eq!(validate::inspect(&dir.join(DATABASE)).unwrap(), 3);
    assert!(m
        .backups()
        .unwrap()
        .iter()
        .any(|b| b.reason == "crash-recovery"));
}
#[test]
fn backup_failure_blocks_destructive_change_and_preserves_prior_backups() {
    let (_r, mut m, dir) = setup();
    office(&mut m, "Keep");
    let backups = dir.join("Backups");
    fs::rename(&backups, dir.join("saved-backups")).unwrap();
    fs::write(&backups, b"not a directory").unwrap();
    let error = m
        .with_database(true, true, |db| OfficeService::new(db).delete(1))
        .unwrap_err();
    assert_eq!(error.code, "BACKUP_FAILED");
    assert_eq!(m.status().access, "UNAVAILABLE");
    assert_eq!(
        validate::read_only(&dir.join(DATABASE))
            .unwrap()
            .query_row("SELECT count(*) FROM offices", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    fs::remove_file(&backups).unwrap();
    fs::rename(dir.join("saved-backups"), &backups).unwrap();
    m.manual_backup().unwrap();
    assert_eq!(count(&mut m), 1);
}
#[test]
fn sqlite_write_failure_latches_the_session() {
    let (_r, mut m, _dir) = setup();
    let result: AppResult<()> =
        m.with_database(true, false, |_| Err(failure("DATABASE_WRITE_FAILED")));
    assert_eq!(result.unwrap_err().code, "DATABASE_WRITE_FAILED");
    assert_eq!(
        m.with_database(true, false, |_| Ok(())).unwrap_err().code,
        "DATABASE_WRITE_FAILED"
    );
}
#[test]
fn cloned_ids_require_explicit_selection() {
    let (_r, mut m, dir) = setup();
    let id = m.status().data_id.unwrap();
    m.locations.settings.preferred_id = Some(id.clone());
    let a = location::Candidate {
        directory: dir.display().to_string(),
        data_id: id,
        schema_version: Some(3),
        error_code: None,
    };
    let mut b = a.clone();
    b.directory = "another mounted volume".into();
    assert_eq!(
        m.locations.choose(&[a, b]).unwrap_err().code,
        "MULTIPLE_DATABASES_FOUND"
    );
}
#[test]
fn backup_snapshot_includes_committed_wal() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("wal.sqlite");
    let db = Database::new(path.clone());
    db.initialize().unwrap();
    let c = db.connect().unwrap();
    c.execute_batch(
        "PRAGMA wal_autocheckpoint=0; INSERT INTO offices(name) VALUES('Committed WAL');",
    )
    .unwrap();
    assert!(path.with_file_name("wal.sqlite-wal").exists());
    let target = root.path().join("backup.sqlite");
    backup::snapshot(&path, &target).unwrap();
    assert_eq!(
        validate::read_only(&target)
            .unwrap()
            .query_row("SELECT name FROM offices", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Committed WAL"
    );
}
#[test]
fn missing_remembered_id_never_selects_another_database() {
    let (_r, mut m, dir) = setup();
    m.locations.settings.preferred_id = Some(uuid::Uuid::new_v4().to_string());
    let candidate = location::Candidate {
        directory: dir.display().to_string(),
        data_id: m.status().data_id.unwrap(),
        schema_version: Some(3),
        error_code: None,
    };
    assert_eq!(
        m.locations.choose(&[candidate]).unwrap_err().code,
        "PORTABLE_DATABASE_NOT_FOUND"
    );
}
#[cfg(unix)]
#[test]
fn readonly_directory_supports_viewing_and_releases_handles() {
    use std::os::unix::fs::PermissionsExt;
    let (_root, mut m, dir) = setup();
    m.close(false).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let result = m.open(Mode::Portable, Some(dir.clone()));
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(result.unwrap().access, "READ_ONLY");
    assert_eq!(count(&mut m), 0);
    m.close(false).unwrap();
}
#[cfg(unix)]
#[test]
fn local_lock_fallback_never_grants_editing() {
    use std::os::unix::fs::PermissionsExt;
    let (_root, mut m, dir) = setup();
    m.close(false).unwrap();
    fs::set_permissions(dir.join(LOCK), fs::Permissions::from_mode(0o000)).unwrap();
    let result = m.open(Mode::Portable, Some(dir.clone()));
    fs::set_permissions(dir.join(LOCK), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(result.unwrap().access, "READ_ONLY");
    let candidate = m.backups().unwrap()[0].filename.clone();
    assert_eq!(
        m.restore(&candidate).unwrap_err().code,
        "DATABASE_READ_ONLY"
    );
    assert_eq!(
        m.with_database(true, false, |_| Ok(())).unwrap_err().code,
        "DATABASE_READ_ONLY"
    );
}

#[test]
fn manual_paths_reject_relative_traversal_and_symlink_parents_before_writes() {
    let root = tempfile::tempdir().unwrap();
    let mut m = manager(root.path());
    assert_eq!(m.initialize_portable(PathBuf::from("DentalSchedulerMiniData"), None).unwrap_err().code, "INVALID_STORAGE_PATH");
    assert_eq!(m.open(Mode::Portable, Some(PathBuf::from("relative"))).unwrap_err().code, "INVALID_STORAGE_PATH");
    assert_eq!(location::validate_path(&root.path().join("../other")).unwrap_err().code, "INVALID_STORAGE_PATH");
    #[cfg(unix)]
    {
        let link = root.path().join("redirect");
        std::os::unix::fs::symlink(root.path(), &link).unwrap();
        assert_eq!(m.initialize_portable(link.join("new-data"), None).unwrap_err().code, "INVALID_STORAGE_PATH");
        assert!(!root.path().join("new-data").exists());
    }
}

#[test]
fn diagnostic_report_identifies_version_and_fault_without_sensitive_data() {
    let (root, mut m, dir) = setup();
    office(&mut m, "SENSITIVE EMPLOYEE EXAMPLE");
    m.close(false).unwrap();
    m.open(Mode::Portable, Some(root.path().join("missing"))).unwrap_err();
    let report = m.diagnostics();
    let value: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(value["appVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["networkRequired"], false);
    assert_eq!(value["lastErrorCode"], "INVALID_DATA_METADATA");
    assert!(!report.contains("SENSITIVE EMPLOYEE EXAMPLE"));
    assert!(!report.contains(dir.to_str().unwrap()));
    let log = fs::read_to_string(root.path().join("logs/storage.log")).unwrap();
    assert!(log.contains(&format!("version={}", env!("CARGO_PKG_VERSION"))));
    assert!(!log.contains("SENSITIVE EMPLOYEE EXAMPLE"));
    assert!(!log.contains(dir.to_str().unwrap()));
}
