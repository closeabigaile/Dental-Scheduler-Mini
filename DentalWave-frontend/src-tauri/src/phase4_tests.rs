use super::*;
use crate::{models::calendar::*, repositories::calendar_repository::CalendarRepository, services::calendar_service::{CalendarService,month_dates}};
use chrono::Datelike;

fn setup() -> (TempDir,Database,i64,Vec<i64>,Vec<i64>) {
 let (dir,db)=test_database();
 let office=OfficeService::new(&db).create(office_input("Reference Office")).unwrap();
 let resources=ResourceService::new(&db);
 let doctors=(0..2).map(|i|resources.create(ResourceKind::Doctor,resource_input(ResourceKind::Doctor,&format!("Doctor {i}"),Some(office.id))).unwrap().id).collect();
 let assistants=(0..9).map(|i|resources.create(ResourceKind::Assistant,resource_input(ResourceKind::Assistant,&format!("Assistant {i}"),Some(office.id))).unwrap().id).collect();
 (dir,db,office.id,doctors,assistants)
}
fn normalized(mut calendars:Vec<Calendar>)->Vec<Calendar>{
 for c in &mut calendars {c.id=0;c.revision=0;for d in &mut c.days{d.id=0;for a in &mut d.assignments{a.id=0;}}}calendars
}
#[test]
fn date_boundaries_cover_every_day_and_do_not_assume_four_weeks(){
 for (y,m,n,first,last) in [(2027,2,28,0,6),(2028,2,29,1,1),(2026,4,30,2,3),(2026,8,31,5,0),(2026,12,31,1,3)]{
  let days=month_dates(y,m).unwrap();assert_eq!(days.len(),n);assert_eq!(days[0].weekday().num_days_from_monday(),first);assert_eq!(days[n-1].weekday().num_days_from_monday(),last);
 }
 assert!(month_dates(2026,13).is_err());assert!(month_dates(1899,1).is_err());assert_eq!(month_dates(9999,12).unwrap().len(),31);
}
#[test]
fn reference_java_local_round_robin_counts_and_stable_members_match(){
 let (_dir,db,office,doctors,assistants)=setup();let s=CalendarService::new(&db);
 let r=s.create(2026,9,true).unwrap();assert_eq!(r.status,"GENERATED_SUCCESSFULLY");
 let c=&r.calendars[0];assert_eq!(c.office_id,office);assert_eq!(c.days.len(),18);
 // Java local-resource generator: Mon–Thu fallback; two doctor teams, nine assistants -> 5 + 4.
 let first=&c.days[0];assert_eq!(first.date,"2026-09-01");assert_eq!(first.assignments.len(),2);
 for (i,a) in first.assignments.iter().enumerate(){assert_eq!(a.doctor_id,Some(doctors[i]));assert_eq!(a.assistants.iter().map(|a|a.assistant_id).collect::<Vec<_>>(),assistants.iter().skip(i).step_by(2).copied().collect::<Vec<_>>());}
 s.delete(2026,9,r.calendars.clone()).unwrap();let again=s.create(2026,9,true).unwrap();assert_eq!(normalized(r.calendars),normalized(again.calendars));
 assert_eq!(db.integrity_check().unwrap(),"ok");
}
#[test]
fn calendar_crud_edits_notes_publish_remove_and_reopen_are_exact(){
 let (dir,db,_,_,_)=setup();let s=CalendarService::new(&db);
 let mut c=s.create(2026,9,true).unwrap().calendars;
 let id=c[0].id;assert_eq!(s.get(id).unwrap(),c[0]);assert_eq!(s.list(None,None).unwrap(),c);
 c[0].days[0].assignments[0].assistants[0].partial_day_note="Leaves at noon".into();
 c[0].days[0].notes="Development note".into();c[0].published=true;
 let saved=s.save(2026,9,c).unwrap();assert!(saved[0].published);
 let reopened=Database::new(dir.path().join("development/scheduler.sqlite"));reopened.initialize().unwrap();
 assert_eq!(CalendarService::new(&reopened).list(Some(2026),Some(9)).unwrap(),saved);
 let mut edited=saved;edited[0].days[0].assignments.remove(0);edited[0].published=false;
 let edited=s.save(2026,9,edited).unwrap();assert_eq!(edited[0].days[0].assignments.len(),1);
 s.delete(2026,9,edited).unwrap();assert!(s.list(None,None).unwrap().is_empty());assert_eq!(s.get(id).unwrap_err().code,"CALENDAR_NOT_FOUND");
 let c=db.connect().unwrap();for table in ["calendar_days","daily_assignments","assignment_assistants","calendar_generation_issues"]{assert_eq!(c.query_row(&format!("SELECT count(*) FROM {table}"),[],|r|r.get::<_,i64>(0)).unwrap(),0);}
 assert_eq!(db.integrity_check().unwrap(),"ok");
}
#[test]
fn create_duplicate_never_overwrites_and_empty_month_supports_manual_staffing(){
 let (_dir,db,_,doctors,assistants)=setup();let s=CalendarService::new(&db);let mut c=s.create(2028,2,false).unwrap().calendars;
 assert!(c[0].days.is_empty());assert_eq!(s.create(2028,2,true).unwrap_err().code,"CALENDAR_ALREADY_EXISTS");assert_eq!(s.list(None,None).unwrap(),c);
 c[0].days.push(CalendarDay{id:0,date:"2028-02-29".into(),notes:String::new(),assignments:vec![CalendarAssignment{id:0,doctor_id:Some(doctors[0]),team_id:None,label:"Manual team".into(),assistants:vec![AssistantAssignment{assistant_id:assistants[0],partial_day_note:String::new()}]}]});
 let saved=s.save(2028,2,c).unwrap();assert_eq!(saved[0].days[0].date,"2028-02-29");
}
#[test]
fn invalid_manual_edits_leave_the_entire_month_unchanged(){
 let (_dir,db,_,_,_)=setup();let s=CalendarService::new(&db);let original=s.create(2026,9,true).unwrap().calendars;
 for mode in 0..7 {
  let mut c=original.clone();let day=&mut c[0].days[0];
  match mode {
   0=>day.assignments.push(day.assignments[0].clone()),
   1=>day.assignments[1].assistants[0]=day.assignments[0].assistants[0].clone(),
   2=>day.assignments[0].team_id=Some(9999),
   3=>day.date="2026-09-04".into(), // unconfigured Friday
   4=>day.date="2026-10-01".into(),
   5=>day.assignments[0].doctor_id=Some(9999),
   _=>day.assignments[0].assistants.clear(),
  }
  assert!(s.save(2026,9,c).is_err(),"mode {mode}");assert_eq!(s.list(None,None).unwrap(),original);
 }
}
#[test]
fn cross_office_wrong_office_duplicate_and_stale_saves_rejected(){
 let (_dir,db,office,doctors,assistants)=setup();
 let second=OfficeService::new(&db).create(office_input("Second Office")).unwrap();
 let s=CalendarService::new(&db);let original=s.create(2026,9,true).unwrap().calendars;
 let mut wrong=original.clone();let day=wrong[0].days.remove(0);wrong[1].days.push(day);
 assert_eq!(s.save(2026,9,wrong.clone()).unwrap_err().code,"ASSIGNMENT_CONFLICT");
 // Make both doctors and assistants explicitly eligible at both offices, then try a double booking.
 let rs=ResourceService::new(&db);
 for (kind,ids) in [(ResourceKind::Doctor,doctors),(ResourceKind::Assistant,assistants)]{
  for id in ids {let mut input=resource_input(kind,"Shared",None);input.offices=vec![IdReference{id:office},IdReference{id:second.id}];rs.update(kind,id,input).unwrap();}
 }
 let mut duplicate=original.clone();let duplicate_day=duplicate[0].days[0].clone();duplicate[1].days.push(duplicate_day);
 assert_eq!(s.save(2026,9,duplicate).unwrap_err().code,"ASSIGNMENT_CONFLICT");
 let updated=s.save(2026,9,original.clone()).unwrap();assert_eq!(s.save(2026,9,original.clone()).unwrap_err().code,"SCHEDULE_STALE");assert!(s.delete(2026,9,original).is_err());assert_eq!(s.list(None,None).unwrap(),updated);
 assert_eq!(s.save(2026,9,vec![updated[0].clone()]).unwrap_err().code,"SCHEDULE_STALE");
}
#[test]
fn save_revalidates_current_resource_state_and_foreign_keys_restrict_deletion(){
 let (_dir,db,office,doctors,assistants)=setup();let s=CalendarService::new(&db);let c=s.create(2026,9,true).unwrap().calendars;
 let rs=ResourceService::new(&db);let mut inactive=resource_input(ResourceKind::Assistant,"Inactive",Some(office));inactive.active=false;
 rs.update(ResourceKind::Assistant,assistants[0],inactive).unwrap();assert_eq!(s.save(2026,9,c.clone()).unwrap_err().code,"ASSIGNMENT_CONFLICT");
 assert!(rs.delete(ResourceKind::Doctor,doctors[0]).is_err());assert!(rs.delete(ResourceKind::Assistant,assistants[0]).is_err());assert!(OfficeService::new(&db).delete(office).is_err());
 let mut tampered=c;tampered[0].office_id=9999;assert!(s.save(2026,9,tampered).is_err());
}
#[test]
fn shortage_and_pattern_conflicts_are_reported_and_no_invalid_assignments_saved(){
 let (_dir,db,office,doctors,assistants)=setup();
 let rs=ResourceService::new(&db);for id in assistants.iter().skip(1){rs.delete(ResourceKind::Assistant,*id).unwrap();}
 let s=CalendarService::new(&db);let r=s.create(2026,9,true).unwrap();assert_eq!(r.status,"GENERATED_WITH_WARNINGS");assert!(r.calendars[0].generation_issues.iter().any(|i|i.code=="INSUFFICIENT_ASSISTANTS"));assert!(r.calendars[0].days.iter().all(|d|d.assignments.len()==1));
 // Stored rules in different recurrence slots may collide, which the shared resolver must report.
 let rules=DoctorWorkRuleService::new(&db);let g=rules.create_rotation_group(RotationGroupInput{name:"Cycle".into(),number_of_weeks:2,anchor_date:"2026-08-31".into(),active:true}).unwrap();
 let weekly=weekly_rule(doctors[0],office,"TUESDAY");let mut rotating=weekly.clone();rotating.recurrence_type="ROTATING".into();rotating.rotation_group=Some(IdReference{id:g.id});rotating.rotation_position=Some(1);rules.replace_rules(doctors[0],vec![weekly,rotating]).unwrap();
 let r=s.create(2026,10,true).unwrap();assert!(r.calendars[0].generation_issues.iter().any(|i|i.code=="DOCTOR_PATTERN_CONFLICT"));assert_eq!(db.integrity_check().unwrap(),"ok");
}
#[test]
fn generation_rotation_continues_across_months_instead_of_resetting(){
 let (_dir,db,office,doctors,_)=setup();let rules=DoctorWorkRuleService::new(&db);
 let g=rules.create_rotation_group(RotationGroupInput{name:"Anchored".into(),number_of_weeks:2,anchor_date:"2026-01-19".into(),active:true}).unwrap();
 let mut rule=weekly_rule(doctors[0],office,"MONDAY");rule.recurrence_type="ROTATING".into();rule.rotation_group=Some(IdReference{id:g.id});rule.rotation_position=Some(2);rules.replace_rules(doctors[0],vec![rule]).unwrap();
 let s=CalendarService::new(&db);let jan=s.create(2026,1,true).unwrap().calendars;let feb=s.create(2026,2,true).unwrap().calendars;
 let works=|c:&Vec<Calendar>,date:&str|c[0].days.iter().find(|d|d.date==date).is_some_and(|d|d.assignments.iter().any(|a|a.doctor_id==Some(doctors[0])));
 assert!(works(&jan,"2026-01-26"));assert!(!works(&feb,"2026-02-02"));assert!(works(&feb,"2026-02-09"));
}
#[test]
fn transaction_rolls_back_after_later_office_write_failure(){
 let (_dir,db,_,_,_)=setup();OfficeService::new(&db).create(office_input("Later Office")).unwrap();let s=CalendarService::new(&db);let original=s.create(2026,9,true).unwrap().calendars;
 let mut modified=original.clone();modified[0].days[0].notes="Must roll back".into();modified[1].days.push(CalendarDay{id:0,date:"2026-09-01".into(),notes:"FAIL".into(),assignments:vec![]});
 db.connect().unwrap().execute_batch("CREATE TRIGGER force_write_failure BEFORE INSERT ON calendar_days WHEN NEW.notes='FAIL' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
 assert!(s.save(2026,9,modified).is_err());assert_eq!(s.list(None,None).unwrap(),original);assert_eq!(db.integrity_check().unwrap(),"ok");
}
#[test]
fn populated_phase3_database_migrates_without_losing_existing_records(){
 let dir=tempfile::tempdir().unwrap();let path=dir.path().join("scheduler.sqlite");let c=rusqlite::Connection::open(&path).unwrap();
 c.execute_batch(crate::database::migrations::VERSION_1).unwrap();c.execute_batch(crate::database::migrations::VERSION_2).unwrap();
 c.execute_batch("INSERT INTO offices(id,name) VALUES(10,'Existing'); INSERT INTO doctors(id,display_name,default_office_id) VALUES(20,'Existing doctor',10); INSERT INTO assistants(id,display_name) VALUES(30,'Existing assistant'); INSERT INTO teams(id,name,doctor_id) VALUES(40,'Existing team',20); INSERT INTO team_assistants VALUES(40,30); INSERT INTO doctor_work_rules(doctor_id,day_of_week,office_id,work_status,recurrence_type) VALUES(20,'MONDAY',10,'WORKING','EVERY_WEEK');").unwrap();drop(c);
 let db=Database::new(path);db.initialize().unwrap();db.initialize().unwrap();assert_eq!(db.info().unwrap().schema_version,3);
 assert_eq!(crate::services::team_service::TeamService::new(&db).get(40).unwrap().assistants[0].id,30);assert_eq!(DoctorWorkRuleRepository::list_rules(&db.connect().unwrap(),20).unwrap().len(),1);
 assert!(CalendarService::new(&db).create(2026,9,true).is_ok());assert_eq!(db.integrity_check().unwrap(),"ok");
}
#[test]
fn annotations_and_team_membership_are_explicitly_validated(){
 let (_dir,db,office,doctors,assistants)=setup();let s=CalendarService::new(&db);let mut c=s.create(2026,9,false).unwrap().calendars;
 c[0].days.push(CalendarDay{id:0,date:"2026-09-01".into(),notes:String::new(),assignments:vec![CalendarAssignment{id:0,doctor_id:None,team_id:None,label:"Closed for training".into(),assistants:vec![]}]});
 let mut saved=s.save(2026,9,c).unwrap();saved[0].days[0].assignments[0].assistants.push(AssistantAssignment{assistant_id:assistants[0],partial_day_note:String::new()});assert!(s.save(2026,9,saved).is_err());
 let team=crate::services::team_service::TeamService::new(&db).save(None,crate::models::TeamInput{name:"Reusable".into(),doctor:IdReference{id:doctors[0]},assistants:vec![IdReference{id:assistants[0]}],default_office:Some(IdReference{id:office}),active:true,color:"#65a9b8".into(),notes:String::new()}).unwrap();
 let mut c=s.list(None,None).unwrap();c[0].days[0].assignments.push(CalendarAssignment{id:0,doctor_id:Some(doctors[0]),team_id:Some(team.id),label:team.name,assistants:vec![AssistantAssignment{assistant_id:assistants[0],partial_day_note:String::new()}]});let mut saved=s.save(2026,9,c).unwrap();saved[0].days[0].assignments[1].assistants[0].assistant_id=assistants[1];assert!(s.save(2026,9,saved).is_err());
}
#[test]
fn repository_partial_relationship_write_rolls_back(){
 let (_dir,db,_,_,_)=setup();let s=CalendarService::new(&db);let original=s.create(2026,9,true).unwrap().calendars;let mut modified=original[0].clone();modified.days[0].assignments[0].assistants[0].assistant_id=99999;
 let mut c=db.connect().unwrap();let tx=c.transaction().unwrap();assert!(CalendarRepository::replace(&tx,&modified).is_err());drop(tx);assert_eq!(s.list(None,None).unwrap(),original);
}
