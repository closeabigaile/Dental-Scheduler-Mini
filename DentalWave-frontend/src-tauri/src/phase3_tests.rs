use super::*;
use crate::domain::{self, Assignment, ConflictCode as C, Context};
use crate::models::{
  DoctorReference, DoctorWorkRule, Office, RotationGroup, SchedulingResource, Team, TeamInput,
};
use crate::services::{scheduling_service::SchedulingService, team_service::TeamService};

fn office(id: i64) -> Office {
  Office {
    id,
    name: format!("Test office {id}"),
    address: String::new(),
    phone_number: String::new(),
  }
}
fn resource(id: i64, kind: &str) -> SchedulingResource {
  SchedulingResource {
    id,
    resource_type: kind.into(),
    first_name: String::new(),
    last_name: String::new(),
    display_name: format!("Test {kind} {id}"),
    active: true,
    default_office: None,
    offices: vec![],
    normal_workdays: None,
    color: String::new(),
    notes: String::new(),
  }
}
fn rule(doctor: i64, day: &str, location: i64) -> DoctorWorkRule {
  DoctorWorkRule {
    id: doctor * 10 + location,
    doctor: DoctorReference { id: doctor },
    day_of_week: day.into(),
    office: Some(office(location)),
    work_status: "WORKING".into(),
    recurrence_type: "EVERY_WEEK".into(),
    rotation_group: None,
    rotation_position: None,
    effective_start_date: None,
    effective_end_date: None,
    active: true,
  }
}
fn alternating(doctor: i64, location: i64, position: i64) -> DoctorWorkRule {
  let mut r = rule(doctor, "THURSDAY", location);
  r.recurrence_type = "ROTATING".into();
  r.rotation_position = Some(position);
  r.rotation_group = Some(RotationGroup {
    id: 1,
    name: "Test shared rotation".into(),
    number_of_weeks: 2,
    anchor_date: "2024-01-01".into(),
    active: true,
  });
  r
}
struct Fixture {
  offices: Vec<Office>,
  doctors: Vec<SchedulingResource>,
  assistants: Vec<SchedulingResource>,
  teams: Vec<Team>,
  rules: Vec<DoctorWorkRule>,
}
impl Fixture {
  fn new() -> Self {
    Self {
      offices: vec![office(1), office(2), office(3)],
      doctors: vec![
        resource(1, "DOCTOR"),
        resource(2, "DOCTOR"),
        resource(3, "DOCTOR"),
        resource(4, "DOCTOR"),
      ],
      assistants: vec![resource(1, "ASSISTANT"), resource(2, "ASSISTANT")],
      teams: vec![],
      rules: vec![],
    }
  }
  fn context(&self) -> Context<'_> {
    Context {
      offices: &self.offices,
      doctors: &self.doctors,
      assistants: &self.assistants,
      teams: &self.teams,
      rules: &self.rules,
    }
  }
  fn proposal(&self) -> Assignment {
    Assignment {
      date: "2024-01-01".into(),
      office_id: 1,
      team_id: None,
      doctor_id: 1,
      assistant_ids: vec![1],
    }
  }
  fn codes(&self, p: &Assignment, existing: &[Assignment]) -> Vec<C> {
    domain::validate(p, existing, &self.context())
      .conflicts
      .into_iter()
      .map(|c| c.code)
      .collect()
  }
}

// Reference: DoctorRuleResolutionService.resolve + CalendarServiceImpl.doctorBelongsAtOffice.
#[test]
fn reference_two_day_doctor_rejects_unconfigured_weekdays_and_wrong_office() {
  let mut f = Fixture::new();
  f.rules = vec![rule(1, "MONDAY", 1), rule(1, "TUESDAY", 2)];
  let mut p = f.proposal();
  assert!(f.codes(&p, &[]).is_empty());
  p.office_id = 2;
  assert_eq!(f.codes(&p, &[]), vec![C::DoctorWrongOffice]);
  p.date = "2024-01-02".into();
  assert!(f.codes(&p, &[]).is_empty());
  for day in ["2024-01-03", "2024-01-04", "2024-01-05"] {
    p.date = day.into();
    assert_eq!(f.codes(&p, &[]), vec![C::DoctorNotAvailable]);
  }
}
#[test]
fn reference_four_day_doctor_uses_each_days_office() {
  let mut f = Fixture::new();
  f.rules = vec![
    rule(2, "MONDAY", 1),
    rule(2, "TUESDAY", 3),
    rule(2, "WEDNESDAY", 2),
    rule(2, "THURSDAY", 1),
  ];
  let mut p = f.proposal();
  p.doctor_id = 2;
  for (day, office) in [
    ("2024-01-01", 1),
    ("2024-01-02", 3),
    ("2024-01-03", 2),
    ("2024-01-04", 1),
  ] {
    p.date = day.into();
    p.office_id = office;
    assert!(f.codes(&p, &[]).is_empty());
  }
}
// Java floorDiv(daysBetween(anchor,date),7), floorMod(weeks,numberOfWeeks)+1.
#[test]
fn reference_alternation_handles_anchor_before_anchor_year_boundary_and_opposite_doctor() {
  let a = resource(3, "DOCTOR");
  let b = resource(4, "DOCTOR");
  let ar = vec![alternating(3, 1, 1), alternating(3, 2, 2)];
  let br = vec![alternating(4, 2, 1), alternating(4, 1, 2)];
  for (date, location) in [
    ("2023-12-28", 2),
    ("2024-01-04", 1),
    ("2024-01-11", 2),
    ("2024-01-18", 1),
    ("2025-01-02", 1),
  ] {
    let day = domain::date(date).unwrap();
    assert_eq!(domain::resolve(&a, &ar, day).office_id, Some(location));
    assert_eq!(domain::resolve(&b, &br, day).office_id, Some(3 - location));
  }
  let mut shifted = ar.clone();
  for r in &mut shifted {
    r.rotation_group.as_mut().unwrap().anchor_date = "2024-01-03".into();
  }
  assert_eq!(
    domain::resolve(&a, &shifted, domain::date("2024-01-04").unwrap()).rotation_position,
    Some(1)
  );
}
#[test]
fn reference_effective_dates_inclusive_inactive_and_explicit_not_working() {
  let d = resource(1, "DOCTOR");
  let mut r = rule(1, "MONDAY", 1);
  r.effective_start_date = Some("2024-01-08".into());
  r.effective_end_date = Some("2024-01-15".into());
  for (date, works) in [
    ("2024-01-01", false),
    ("2024-01-08", true),
    ("2024-01-15", true),
    ("2024-01-22", false),
  ] {
    assert_eq!(
      domain::resolve(&d, &[r.clone()], domain::date(date).unwrap()).working,
      works
    );
  }
  r.active = false;
  let resolved = domain::resolve(&d, &[r.clone()], domain::date("2024-01-08").unwrap());
  assert!(!resolved.working);
  assert!(!resolved.fallback);
  r.active = true;
  r.work_status = "NOT_WORKING".into();
  assert!(!domain::resolve(&d, &[r], domain::date("2024-01-08").unwrap()).working);
  let mut rotated = alternating(1, 1, 1);
  rotated.rotation_group.as_mut().unwrap().active = false;
  assert!(!domain::resolve(&d, &[rotated], domain::date("2024-01-04").unwrap()).working);
}
#[test]
fn reference_fallback_uses_memberships_default_office_and_normal_workdays() {
  let mut f = Fixture::new();
  let mut p = f.proposal();
  assert!(f.codes(&p, &[]).is_empty());
  p.date = "2024-01-05".into();
  assert_eq!(f.codes(&p, &[]), vec![C::DoctorNotAvailable]);
  f.doctors[0].normal_workdays = Some("FRIDAY SATURDAY".into());
  assert!(f.codes(&p, &[]).is_empty());
  p.date = "2024-01-06".into();
  assert_eq!(f.codes(&p, &[]), vec![C::DoctorNotAvailable]);
  p.date = "2024-01-05".into();
  f.doctors[0].default_office = Some(office(2));
  assert_eq!(f.codes(&p, &[]), vec![C::DoctorWrongOffice]);
  f.doctors[0].offices = vec![office(1)];
  assert!(f.codes(&p, &[]).is_empty());
  // Explicit work rules take precedence over fallback office membership and Friday defaults.
  f.rules = vec![rule(1, "FRIDAY", 3)];
  p.office_id = 3;
  assert!(f.codes(&p, &[]).is_empty());
}
#[test]
fn reference_pattern_collision_is_reported_without_silent_override() {
  let mut f = Fixture::new();
  f.rules = vec![rule(1, "THURSDAY", 1), alternating(1, 2, 1)];
  let mut p = f.proposal();
  p.date = "2024-01-04".into();
  assert_eq!(f.codes(&p, &[]), vec![C::PatternConflict]);
  assert_eq!(p.office_id, 1);
  assert_eq!(p.assistant_ids, vec![1]);
}
#[test]
fn reference_assistant_eligibility_duplicates_and_cross_office_full_day_conflicts() {
  let mut f = Fixture::new();
  let p = f.proposal();
  let mut other = p.clone();
  other.office_id = 2;
  assert_eq!(
    f.codes(&p, &[other.clone()]),
    vec![C::DoctorAlreadyAssigned, C::AssistantAlreadyAssigned]
  );
  other.date = "2024-01-02".into();
  assert!(f.codes(&p, &[other]).is_empty());
  f.assistants[0].default_office = Some(office(2));
  assert_eq!(f.codes(&p, &[]), vec![C::AssistantWrongOffice]);
  f.assistants[0].offices = vec![office(1)];
  assert!(f.codes(&p, &[]).is_empty());
  let mut duplicate = p.clone();
  duplicate.assistant_ids.push(1);
  assert_eq!(f.codes(&duplicate, &[]), vec![C::AssistantAlreadyAssigned]);
  f.assistants[0].active = false;
  assert_eq!(f.codes(&p, &[]), vec![C::AssistantInvalid]);
}
#[test]
fn assignment_references_team_memberships_and_missing_staff_are_structured() {
  let mut f = Fixture::new();
  let mut p = f.proposal();
  f.teams.push(Team {
    id: 1,
    name: "Test team".into(),
    doctor: f.doctors[0].clone(),
    assistants: vec![f.assistants[0].clone()],
    default_office: Some(office(2)),
    active: true,
    color: String::new(),
    notes: String::new(),
  });
  p.team_id = Some(1);
  assert!(f.codes(&p, &[]).is_empty()); // team default is a preference, not a restriction
  p.assistant_ids = vec![2];
  assert_eq!(f.codes(&p, &[]), vec![C::TeamInvalid]);
  p.assistant_ids = vec![1];
  f.teams[0].active = false;
  assert_eq!(f.codes(&p, &[]), vec![C::TeamInvalid]);
  p.team_id = Some(999);
  assert_eq!(f.codes(&p, &[]), vec![C::TeamInvalid]);
  p.team_id = None;
  p.assistant_ids.clear();
  assert_eq!(f.codes(&p, &[]), vec![C::MissingRequiredResource]);
  p.assistant_ids = vec![999];
  assert_eq!(f.codes(&p, &[]), vec![C::AssistantInvalid]);
  p.assistant_ids = vec![1];
  p.doctor_id = 999;
  assert_eq!(f.codes(&p, &[]), vec![C::DoctorInvalid]);
  p.doctor_id = 1;
  f.doctors[0].active = false;
  assert_eq!(f.codes(&p, &[]), vec![C::DoctorInvalid]);
  f.doctors[0].active = true;
  p.office_id = 999;
  assert_eq!(f.codes(&p, &[]), vec![C::OfficeInvalid]);
  p.office_id = 1;
  p.date = "2024-02-30".into();
  assert_eq!(f.codes(&p, &[]), vec![C::InvalidDate]);
}
#[test]
fn calendar_date_validation_handles_leap_days_and_rejects_noncanonical_dates() {
  assert!(domain::date("2024-02-29").is_ok());
  for date in [
    "2023-02-29",
    "2024-13-01",
    "2024-1-01",
    "2024-01-1",
    "not a date",
  ] {
    assert!(domain::date(date).is_err());
  }
}
fn team_input(doctor: i64, assistant: i64, office: i64) -> TeamInput {
  TeamInput {
    name: " Test Team ".into(),
    doctor: IdReference { id: doctor },
    assistants: vec![IdReference { id: assistant }, IdReference { id: assistant }],
    default_office: Some(IdReference { id: office }),
    active: true,
    color: "#123456".into(),
    notes: "test notes".into(),
  }
}
#[test]
fn teams_crud_duplicate_membership_changes_and_reopen_persist() {
  let (_dir, db) = test_database();
  let o = OfficeService::new(&db)
    .create(office_input("Test office"))
    .unwrap();
  let rs = ResourceService::new(&db);
  let d = rs
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test doctor", None),
    )
    .unwrap();
  let a = rs
    .create(
      ResourceKind::Assistant,
      resource_input(ResourceKind::Assistant, "Test assistant", None),
    )
    .unwrap();
  let service = TeamService::new(&db);
  let t = service.save(None, team_input(d.id, a.id, o.id)).unwrap();
  assert_eq!(t.name, "Test Team");
  assert_eq!(t.assistants.len(), 1);
  assert_eq!(t.doctor.id, d.id);
  assert!(service.save(None, team_input(d.id, a.id, o.id)).is_err());
  let copy = service.duplicate(t.id).unwrap();
  assert_eq!(copy.name, "Test Team Copy");
  assert_eq!(copy.assistants, t.assistants);
  assert_eq!(service.duplicate(t.id).unwrap().name, "Test Team Copy 2");
  let reopened = Database::new(db.path().to_path_buf());
  reopened.initialize().unwrap();
  assert_eq!(TeamService::new(&reopened).get(t.id).unwrap(), t);
  let mut input = team_input(d.id, a.id, o.id);
  input.name = "Renamed".into();
  input.assistants.clear();
  input.active = false;
  let changed = service.save(Some(t.id), input).unwrap();
  assert!(!changed.active);
  assert!(changed.assistants.is_empty());
  assert!(rs.delete(ResourceKind::Doctor, d.id).is_err());
  assert!(rs.delete(ResourceKind::Assistant, a.id).is_err());
  assert!(OfficeService::new(&db).delete(o.id).is_err());
  service.delete(copy.id).unwrap();
  assert!(service.get(copy.id).is_err());
  let c = db.connect().unwrap();
  let count: i64 = c
    .query_row(
      "SELECT count(*) FROM team_assistants WHERE team_id=?1",
      [copy.id],
      |r| r.get(0),
    )
    .unwrap();
  assert_eq!(count, 0);
}
#[test]
fn team_transaction_rolls_back_on_invalid_membership_and_rejects_inactive_resources() {
  let (_dir, db) = test_database();
  let o = OfficeService::new(&db)
    .create(office_input("Test office"))
    .unwrap();
  let rs = ResourceService::new(&db);
  let d = rs
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test doctor", None),
    )
    .unwrap();
  let a = rs
    .create(
      ResourceKind::Assistant,
      resource_input(ResourceKind::Assistant, "Test assistant", None),
    )
    .unwrap();
  let service = TeamService::new(&db);
  let original = service.save(None, team_input(d.id, a.id, o.id)).unwrap();
  let mut bad = team_input(d.id, a.id, o.id);
  bad.name = "Should roll back".into();
  bad.assistants.push(IdReference { id: 999 });
  assert!(service.save(Some(original.id), bad.clone()).is_err());
  assert_eq!(service.get(original.id).unwrap(), original);
  // Exercise the SQL rollback as well as service prevalidation.
  let mut c = db.connect().unwrap();
  {
    let tx = c.transaction().unwrap();
    assert!(crate::repositories::team_repository::TeamRepository::save(
      &tx,
      Some(original.id),
      &bad
    )
    .is_err());
  }
  assert_eq!(service.get(original.id).unwrap(), original);
  c.execute("UPDATE assistants SET active=0 WHERE id=?1", [a.id])
    .unwrap();
  assert_eq!(
    service
      .save(None, team_input(d.id, a.id, o.id))
      .unwrap_err()
      .code,
    "INVALID_TEAM"
  );
  c.execute("UPDATE assistants SET active=1 WHERE id=?1", [a.id])
    .unwrap();
  c.execute("UPDATE doctors SET active=0 WHERE id=?1", [d.id])
    .unwrap();
  assert_eq!(
    service
      .save(None, team_input(d.id, a.id, o.id))
      .unwrap_err()
      .code,
    "INVALID_TEAM"
  );
  assert_eq!(service.list().unwrap().len(), 1);
}
#[test]
fn v1_migration_preserves_resources_relationships_and_patterns_and_is_idempotent() {
  let dir = tempfile::tempdir().unwrap();
  let db = Database::new(dir.path().join("v1.sqlite"));
  let c = rusqlite::Connection::open(db.path()).unwrap();
  c.execute_batch(crate::database::migrations::VERSION_1)
    .unwrap();
  c.execute_batch("INSERT INTO offices(id,name) VALUES(11,'Test legacy office'); INSERT INTO doctors(id,display_name,default_office_id) VALUES(21,'Test legacy doctor',11); INSERT INTO assistants(id,display_name) VALUES(31,'Test legacy assistant'); INSERT INTO doctor_offices VALUES(21,11); INSERT INTO assistant_offices VALUES(31,11); INSERT INTO doctor_rotation_groups(id,name,number_of_weeks,anchor_date) VALUES(41,'Test legacy rotation',2,'2024-01-01'); INSERT INTO doctor_work_rules(id,doctor_id,day_of_week,office_id,work_status,recurrence_type,rotation_group_id,rotation_position) VALUES(51,21,'THURSDAY',11,'WORKING','ROTATING',41,1);").unwrap();
  drop(c);
  db.initialize().unwrap();
  db.initialize().unwrap();
  assert_eq!(db.info().unwrap().schema_version, 3);
  let rs = ResourceService::new(&db);
  assert_eq!(rs.get(ResourceKind::Doctor, 21).unwrap().offices[0].id, 11);
  assert_eq!(
    rs.get(ResourceKind::Assistant, 31).unwrap().offices[0].id,
    11
  );
  let rules = DoctorWorkRuleService::new(&db).list_rules(21).unwrap();
  assert_eq!(rules[0].id, 51);
  assert_eq!(rules[0].rotation_group.as_ref().unwrap().id, 41);
  let service = TeamService::new(&db);
  assert!(service.list().unwrap().is_empty());
  service.save(None, team_input(21, 31, 11)).unwrap();
  assert_eq!(
    SchedulingService::new(&db)
      .resolve(21, "2024-01-04")
      .unwrap()
      .office_id,
    Some(11)
  );
  assert_eq!(db.integrity_check().unwrap(), "ok");
}
#[test]
fn work_pattern_storage_validates_dates_positions_overlaps_and_preserves_previous_rules() {
  let (_dir, db) = test_database();
  let o = OfficeService::new(&db)
    .create(office_input("Test office"))
    .unwrap();
  let d = ResourceService::new(&db)
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test doctor", None),
    )
    .unwrap();
  let service = DoctorWorkRuleService::new(&db);
  let input = weekly_rule(d.id, o.id, "MONDAY");
  let original = service.replace_rules(d.id, vec![input.clone()]).unwrap();
  assert_eq!(
    service.create_rule(input.clone()).unwrap_err().code,
    "PATTERN_CONFLICT"
  );
  assert!(service
    .replace_rules(d.id, vec![input.clone(), input.clone()])
    .is_err());
  assert_eq!(service.list_rules(d.id).unwrap(), original);
  for (start, end) in [("2024-02-30", "2024-03-01"), ("2024-03-01", "2024-02-01")] {
    let mut r = input.clone();
    r.effective_start_date = Some(start.into());
    r.effective_end_date = Some(end.into());
    assert!(service.replace_rules(d.id, vec![r]).is_err());
  }
  let mut a = input.clone();
  a.effective_end_date = Some("2024-01-07".into());
  let mut b = input.clone();
  b.effective_start_date = Some("2024-01-08".into());
  assert!(service.replace_rules(d.id, vec![a, b]).is_ok());
  let group = service
    .create_rotation_group(RotationGroupInput {
      name: "Test rotation".into(),
      number_of_weeks: 2,
      anchor_date: "2024-01-01".into(),
      active: true,
    })
    .unwrap();
  let mut r = input;
  r.recurrence_type = "ROTATING".into();
  r.rotation_group = Some(IdReference { id: group.id });
  r.rotation_position = Some(3);
  assert!(service.replace_rules(d.id, vec![r.clone()]).is_err());
  r.rotation_position = Some(1);
  service.replace_rules(d.id, vec![r]).unwrap();
  let reopened = Database::new(db.path().to_path_buf());
  reopened.initialize().unwrap();
  let scheduler = SchedulingService::new(&reopened);
  assert_eq!(
    scheduler.resolve(d.id, "2024-01-01").unwrap().office_id,
    Some(o.id)
  );
  assert!(!scheduler.resolve(d.id, "2024-01-08").unwrap().working);
  assert_eq!(scheduler.preview(d.id, "2024-01-01", 28).unwrap().len(), 16);
  let c = db.connect().unwrap();
  assert!(c
    .execute("DELETE FROM offices WHERE id=?1", [o.id])
    .is_err());
  assert!(c
    .execute("DELETE FROM doctor_rotation_groups WHERE id=?1", [group.id])
    .is_err());
  ResourceService::new(&db)
    .delete(ResourceKind::Doctor, d.id)
    .unwrap();
  let n: i64 = c
    .query_row("SELECT count(*) FROM doctor_work_rules", [], |r| r.get(0))
    .unwrap();
  assert_eq!(n, 0);
}
#[test]
fn resource_storage_preserves_explicit_offices_without_adding_default_office() {
  let (_dir, db) = test_database();
  let os = OfficeService::new(&db);
  let one = os.create(office_input("Test One")).unwrap();
  let two = os.create(office_input("Test Two")).unwrap();
  let mut input = resource_input(ResourceKind::Assistant, "Test Assistant", Some(two.id));
  input.offices = vec![IdReference { id: one.id }];
  let a = ResourceService::new(&db)
    .create(ResourceKind::Assistant, input)
    .unwrap();
  assert_eq!(a.offices, vec![one.clone()]);
  assert_eq!(a.default_office, Some(two.clone()));
  assert!(domain::office_allowed(&a, one.id));
  assert!(!domain::office_allowed(&a, two.id));
}
#[test]
fn scheduling_service_validates_persisted_team_and_detects_later_deactivation() {
  let (_dir, db) = test_database();
  let o = OfficeService::new(&db)
    .create(office_input("Test office"))
    .unwrap();
  let rs = ResourceService::new(&db);
  let d = rs
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test doctor", None),
    )
    .unwrap();
  let a = rs
    .create(
      ResourceKind::Assistant,
      resource_input(ResourceKind::Assistant, "Test assistant", None),
    )
    .unwrap();
  let t = TeamService::new(&db)
    .save(None, team_input(d.id, a.id, o.id))
    .unwrap();
  let p = Assignment {
    date: "2024-01-01".into(),
    office_id: o.id,
    team_id: Some(t.id),
    doctor_id: d.id,
    assistant_ids: vec![a.id],
  };
  let service = SchedulingService::new(&db);
  assert!(service.validate(&p, &[]).unwrap().valid);
  db.connect()
    .unwrap()
    .execute("UPDATE doctors SET active=0 WHERE id=?1", [d.id])
    .unwrap();
  assert!(!service.validate(&p, &[]).unwrap().valid);
  assert_eq!(
    service.resolve(d.id, &p.date).unwrap().conflict,
    Some(C::DoctorInvalid)
  );
}

#[test]
fn reference_pattern_normalization_and_rotation_creation_bounds() {
  let (_dir, db) = test_database();
  let office = OfficeService::new(&db)
    .create(office_input("Test office"))
    .unwrap();
  let doctor = ResourceService::new(&db)
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test doctor", None),
    )
    .unwrap();
  let service = DoctorWorkRuleService::new(&db);
  for weeks in [0, 1, 13] {
    assert!(service
      .create_rotation_group(RotationGroupInput {
        name: format!("Invalid {weeks}"),
        number_of_weeks: weeks,
        anchor_date: "2024-01-01".into(),
        active: true
      })
      .is_err());
  }
  assert!(service
    .create_rotation_group(RotationGroupInput {
      name: "Invalid date".into(),
      number_of_weeks: 2,
      anchor_date: "2024-02-30".into(),
      active: true
    })
    .is_err());
  let mut r = weekly_rule(doctor.id, office.id, "MONDAY");
  r.work_status = "NOT_WORKING".into();
  r.rotation_group = Some(IdReference { id: 999 });
  r.rotation_position = Some(5);
  let saved = service.create_rule(r.clone()).unwrap();
  assert!(saved.office.is_none());
  assert!(saved.rotation_group.is_none());
  assert!(saved.rotation_position.is_none());
  r.active = false;
  assert!(service.create_rule(r).is_ok());
  let mut invalid = weekly_rule(doctor.id, 999, "TUESDAY");
  assert!(service.create_rule(invalid.clone()).is_err());
  invalid.office = Some(IdReference { id: office.id });
  invalid.doctor.id = 999;
  assert!(service.create_rule(invalid).is_err());
}

#[test]
fn team_save_rejects_missing_references_and_can_replace_its_doctor() {
  let (_dir, db) = test_database();
  let o = OfficeService::new(&db)
    .create(office_input("Test office"))
    .unwrap();
  let rs = ResourceService::new(&db);
  let d = rs
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test A", None),
    )
    .unwrap();
  let other = rs
    .create(
      ResourceKind::Doctor,
      resource_input(ResourceKind::Doctor, "Test B", None),
    )
    .unwrap();
  let a = rs
    .create(
      ResourceKind::Assistant,
      resource_input(ResourceKind::Assistant, "Test assistant", None),
    )
    .unwrap();
  let service = TeamService::new(&db);
  for input in [
    team_input(999, a.id, o.id),
    team_input(d.id, 999, o.id),
    team_input(d.id, a.id, 999),
  ] {
    assert_eq!(service.save(None, input).unwrap_err().code, "INVALID_TEAM");
  }
  let t = service.save(None, team_input(d.id, a.id, o.id)).unwrap();
  let changed = service
    .save(Some(t.id), team_input(other.id, a.id, o.id))
    .unwrap();
  assert_eq!(changed.doctor.id, other.id);
  rs.delete(ResourceKind::Doctor, d.id).unwrap();
}
