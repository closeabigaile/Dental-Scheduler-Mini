//! Explicit development migration verification, never run at normal startup.
use crate::{
  domain::{self, Assignment, Context},
  models::ResourceKind,
  repositories::{
    calendar_repository::CalendarRepository, doctor_work_rule_repository::DoctorWorkRuleRepository,
    office_repository::OfficeRepository, resource_repository::ResourceRepository,
    team_repository::TeamRepository,
  },
};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::path::Path;

pub fn verify(path: &Path, input: Value) -> Result<Value, Box<dyn std::error::Error>> {
  let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
  let version: i64 = c.query_row(
    "SELECT schema_version FROM schema_metadata WHERE id=1",
    [],
    |r| r.get(0),
  )?;
  if version != crate::database::migrations::SUPPORTED_SCHEMA_VERSION {
    return Err("SCHEMA_MISMATCH".into());
  }
  let offices = OfficeRepository::list(&c)?;
  let doctors = ResourceRepository::list(&c, ResourceKind::Doctor)?;
  let assistants = ResourceRepository::list(&c, ResourceKind::Assistant)?;
  let teams = TeamRepository::list(&c)?;
  let rules = doctors
    .iter()
    .map(|d| DoctorWorkRuleRepository::list_rules(&c, d.id))
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
  let ctx = Context {
    offices: &offices,
    doctors: &doctors,
    assistants: &assistants,
    teams: &teams,
    rules: &rules,
  };
  let mut comparisons = Vec::new();
  for case in input["cases"].as_array().ok_or("INVALID_CASES")? {
    let id = case["doctorId"].as_i64().ok_or("INVALID_ID")?;
    let day = case["date"].as_str().ok_or("INVALID_DATE")?;
    let doctor = doctors
      .iter()
      .find(|d| d.id == id)
      .ok_or("DOCTOR_MISSING")?;
    let own = rules
      .iter()
      .filter(|r| r.doctor.id == id)
      .cloned()
      .collect::<Vec<_>>();
    let result = domain::resolve(doctor, &own, domain::date(day).map_err(|_| "INVALID_DATE")?);
    let actual = json!({"working":result.working,"officeId":result.office_id,
      "ruleId":result.rule_id,"fallback":result.fallback,"conflict":result.conflict});
    comparisons
      .push(json!({"doctorId":id,"date":day,"matches":actual==case["expected"],"actual":actual}));
  }
  let calendars = CalendarRepository::list(&c, None, None)?;
  let assignments = calendars
    .iter()
    .flat_map(|cal| {
      cal.days.iter().flat_map(move |day| {
        day.assignments.iter().filter_map(move |a| {
          a.doctor_id.map(|doctor_id| {
            (
              a.id,
              Assignment {
                date: day.date.clone(),
                office_id: cal.office_id,
                team_id: a.team_id,
                doctor_id,
                assistant_ids: a.assistants.iter().map(|a| a.assistant_id).collect(),
              },
            )
          })
        })
      })
    })
    .collect::<Vec<_>>();
  let mut warnings = Vec::new();
  for (id, p) in &assignments {
    let others = assignments
      .iter()
      .filter(|(other, _)| other != id)
      .map(|(_, p)| p.clone())
      .collect::<Vec<_>>();
    let result = domain::validate(p, &others, &ctx);
    for conflict in result.conflicts {
      warnings
        .push(json!({"assignmentId":id,"code":conflict.code,"resourceId":conflict.resource_id}));
    }
  }
  Ok(
    json!({"schemaVersion":version,"comparisons":comparisons,"historicalWarnings":warnings,
    "loadedCounts":{"offices":offices.len(),"doctors":doctors.len(),"assistants":assistants.len(),"teams":teams.len(),"calendars":calendars.len()}}),
  )
}

pub fn run_cli(path: &Path) -> i32 {
  let result = (|| {
    let input = serde_json::from_reader(std::io::stdin())?;
    verify(path, input)
  })();
  match result {
    Ok(report) => {
      println!("{}", report);
      0
    }
    Err(_) => {
      eprintln!("MIGRATION_VERIFICATION_FAILED");
      2
    }
  }
}
