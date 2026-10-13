use crate::{database::Database, domain::{self, Assignment, Context}, error::{AppError,AppResult}, models::{calendar::*, *}, repositories::{calendar_repository::CalendarRepository as Repo, doctor_work_rule_repository::DoctorWorkRuleRepository, office_repository::OfficeRepository, resource_repository::ResourceRepository, team_repository::TeamRepository}};
use chrono::{Datelike, NaiveDate};
use rusqlite::{Connection, TransactionBehavior};
use std::collections::HashSet;

pub fn month_dates(year:i32, month:u32)->AppResult<Vec<NaiveDate>> {
  if !(1900..=9999).contains(&year) {return Err(AppError::new("INVALID_CALENDAR_DATE","Choose a year between 1900 and 9999."));}
  let first=NaiveDate::from_ymd_opt(year,month,1).ok_or_else(||AppError::new("INVALID_CALENDAR_DATE","Choose a valid month and year."))?;
  Ok(first.iter_days().take_while(|d|d.month()==month && d.year()==year).collect())
}
struct Resources {offices:Vec<Office>, doctors:Vec<SchedulingResource>, assistants:Vec<SchedulingResource>, teams:Vec<Team>,rules:Vec<DoctorWorkRule>}
impl Resources {
  fn load(c:&Connection)->AppResult<Self> {
    let mut doctors=ResourceRepository::list(c,ResourceKind::Doctor)?;
    let mut assistants=ResourceRepository::list(c,ResourceKind::Assistant)?;
    let mut offices=OfficeRepository::list(c)?;
    doctors.sort_by_key(|d|d.id); assistants.sort_by_key(|a|a.id); offices.sort_by_key(|o|o.id);
    let rules=doctors.iter().map(|d|DoctorWorkRuleRepository::list_rules(c,d.id)).collect::<AppResult<Vec<_>>>()?.into_iter().flatten().collect();
    Ok(Self{offices,doctors,assistants,teams:TeamRepository::list(c)?,rules})
  }
  fn context(&self)->Context<'_>{Context{offices:&self.offices,doctors:&self.doctors,assistants:&self.assistants,teams:&self.teams,rules:&self.rules}}
}
fn proposal(cal:&Calendar,day:&CalendarDay,a:&CalendarAssignment)->Option<Assignment>{
 a.doctor_id.map(|doctor_id|Assignment{date:day.date.clone(),office_id:cal.office_id,doctor_id,team_id:a.team_id,assistant_ids:a.assistants.iter().map(|a|a.assistant_id).collect()})
}
fn validate(calendars:&[Calendar],resources:&Resources)->AppResult<()> {
 let mut existing=vec![];
 for cal in calendars {
  let dates=month_dates(cal.year,cal.month)?;
  if !resources.offices.iter().any(|o|o.id==cal.office_id){return Err(AppError::new("ASSIGNMENT_CONFLICT","The selected office no longer exists."));}
  let mut seen=HashSet::new();
  for day in &cal.days {
   let date=domain::date(&day.date).map_err(|_|AppError::new("INVALID_CALENDAR_DATE","Use a valid schedule date."))?;
   if !dates.contains(&date) || !seen.insert(date){return Err(AppError::new("INVALID_CALENDAR_DATE","Each office date must appear once and belong to this calendar month."));}
   if day.notes.len()>4000 {return Err(AppError::invalid_input("Day notes are too long."));}
   for a in &day.assignments {
    if a.label.len()>200 || a.assistants.iter().any(|a|a.partial_day_note.chars().count()>40){return Err(AppError::invalid_input("Use a label of at most 200 bytes and assistant notes of at most 40 characters."));}
    if let Some(p)=proposal(cal,day,a) {
     let v=domain::validate(&p,&existing,&resources.context());
     if !v.valid {
      let details=v.conflicts.iter().map(|c| format!("{}{}",c.code.message(),c.resource_id.map(|id|format!(" (resource {id})")).unwrap_or_default())).collect::<Vec<_>>().join(", ");
      let mut error=AppError::new("ASSIGNMENT_CONFLICT",format!("{}: {}",day.date,details));
      error.conflicts=v.conflicts; return Err(error);
     }
     existing.push(p);
    } else if a.label.trim().is_empty() || a.team_id.is_some() || !a.assistants.is_empty() {
     return Err(AppError::new("ASSIGNMENT_CONFLICT","A note/closure row requires a label and cannot contain staffing."));
    }
   }
  }
 }
 Ok(())
}
fn issue(cal:&mut Calendar,date:&str,code:&str,doctor:Option<i64>,message:String){cal.generation_issues.push(GenerationIssue{date:date.into(),code:code.into(),doctor_id:doctor,message});}

pub struct CalendarService<'a>{database:&'a Database}
impl<'a> CalendarService<'a> {
 pub fn new(database:&'a Database)->Self{Self{database}}
 pub fn list(&self,year:Option<i32>,month:Option<u32>)->AppResult<Vec<Calendar>>{
  if let (Some(y),Some(m))=(year,month){month_dates(y,m)?;}
  let mut c=self.database.connect()?; let tx=c.transaction().map_err(AppError::from_read_error)?;
  Repo::list(&tx,year,month)
 }
 pub fn get(&self,id:i64)->AppResult<Calendar>{let mut c=self.database.connect()?;let tx=c.transaction().map_err(AppError::from_read_error)?;Repo::get(&tx,id)}
 pub fn create(&self,year:i32,month:u32,generate:bool)->AppResult<CalendarResult>{
  let dates=month_dates(year,month)?;
  let mut c=self.database.connect()?;let tx=c.transaction_with_behavior(TransactionBehavior::Immediate).map_err(AppError::from_write_error)?;
  if !Repo::list(&tx,Some(year),Some(month))?.is_empty(){return Err(AppError::new("CALENDAR_ALREADY_EXISTS","This month already exists. Open the saved schedule."));}
  let resources=Resources::load(&tx)?;
  if resources.offices.is_empty(){return Err(AppError::new("INVALID_CONFIGURATION","Add an office first."));}
  let mut calendars=resources.offices.iter().map(|o|Repo::create(&tx,year,month,o.id)).collect::<AppResult<Vec<_>>>()?;
  let mut existing=vec![];
  if generate {
   for date in dates {
    let date_string=date.to_string();
    for cal in &mut calendars {
     let mut slots=vec![];
     for d in resources.doctors.iter().filter(|d|d.active) {
      let rules=resources.rules.iter().filter(|r|r.doctor.id==d.id).cloned().collect::<Vec<_>>();
      let resolved=domain::resolve(d,&rules,date);
      if let Some(conflict)=resolved.conflict {
       issue(cal,&date_string,"DOCTOR_PATTERN_CONFLICT",Some(d.id),format!("{}: {:?}. Review doctor patterns.",d.display_name,conflict)); continue;
      }
      let p=Assignment{date:date_string.clone(),office_id:cal.office_id,team_id:None,doctor_id:d.id,assistant_ids:vec![]};
      // Ask the shared validator about eligibility; missing assistants are filled below.
      let v=domain::validate(&p,&[],&resources.context());
      if v.conflicts.iter().any(|c|c.code!=domain::ConflictCode::MissingRequiredResource){continue;}
      if existing.iter().any(|a:&Assignment|a.date==date_string && a.doctor_id==d.id) {
       issue(cal,&date_string,"UNABLE_TO_ASSIGN_DOCTOR",Some(d.id),format!("{} is eligible here but already placed at another office. Configure an explicit office pattern.",d.display_name));continue;
      }
      slots.push(CalendarAssignment{id:0,doctor_id:Some(d.id),team_id:None,label:String::new(),assistants:vec![]});
     }
     if slots.is_empty(){continue;}
     let available=resources.assistants.iter().filter(|a|a.active && domain::office_allowed(a,cal.office_id) && !existing.iter().any(|p:&Assignment|p.date==date_string && p.assistant_ids.contains(&a.id))).collect::<Vec<_>>();
     let count=slots.len();
     for (i,a) in available.iter().take(count*5).enumerate(){slots[i%count].assistants.push(AssistantAssignment{assistant_id:a.id,partial_day_note:String::new()});}
     let mut day=CalendarDay{id:0,date:date_string.clone(),notes:String::new(),assignments:vec![]};
     for a in slots {
      let doctor=a.doctor_id.unwrap();
      if a.assistants.len()<4 {
       issue(cal,&date_string,"INSUFFICIENT_ASSISTANTS",Some(doctor),format!("Doctor {doctor}: {} assistant(s) available; target is four. {}",a.assistants.len(),if a.assistants.is_empty(){"No assignment was saved."}else{"Review staffing."}));
      }
      if a.assistants.is_empty(){continue;}
      let p=proposal(cal,&day,&a).unwrap();
      let v=domain::validate(&p,&existing,&resources.context());
      if !v.valid {return Err(AppError::new("SCHEDULE_GENERATION_FAILED","Generated staffing did not pass validation. Nothing was saved."));}
      existing.push(p);day.assignments.push(a);
     }
     cal.days.push(day);
    }
   }
   for cal in &mut calendars {
    if cal.days.iter().all(|d|d.assignments.is_empty()) {
     issue(cal,&format!("{year:04}-{month:02}-01"),"INVALID_CONFIGURATION",None,"No staffed assignments were generated for this office. Review doctors, patterns and assistants, or add assignments manually.".into());
    }
   }
  }
  validate(&calendars,&resources)?;
  let warnings=calendars.iter().any(|c|!c.generation_issues.is_empty());
  for cal in &calendars {Repo::replace(&tx,cal)?;Repo::record_issues(&tx,cal)?;}
  let loaded=Repo::list(&tx,Some(year),Some(month))?;tx.commit().map_err(AppError::from_write_error)?;
  Ok(CalendarResult{status:if warnings{"GENERATED_WITH_WARNINGS"}else if generate{"GENERATED_SUCCESSFULLY"}else{"CREATED"}.into(),calendars:loaded})
 }
 pub fn save(&self,year:i32,month:u32,calendars:Vec<Calendar>)->AppResult<Vec<Calendar>>{
  month_dates(year,month)?;
  let mut c=self.database.connect()?;let tx=c.transaction_with_behavior(TransactionBehavior::Immediate).map_err(AppError::from_write_error)?;
  let saved=Repo::list(&tx,Some(year),Some(month))?;
  Self::check_snapshot(&saved,&calendars)?;
  validate(&calendars,&Resources::load(&tx)?)?;
  for cal in &calendars {Repo::replace(&tx,cal)?;}
  let result=Repo::list(&tx,Some(year),Some(month))?;
  tx.commit().map_err(AppError::from_write_error)?;Ok(result)
 }
 fn check_snapshot(saved:&[Calendar],incoming:&[Calendar])->AppResult<()> {
  if saved.is_empty(){return Err(AppError::new("CALENDAR_NOT_FOUND","Create or open this month first."));}
  let ids=incoming.iter().map(|c|c.id).collect::<HashSet<_>>();
  if ids.len()!=saved.len() || incoming.len()!=saved.len(){return Err(AppError::new("SCHEDULE_STALE","Save must include every office in the loaded month. Reload the month."));}
  for cal in incoming {
   let old=saved.iter().find(|s|s.id==cal.id).ok_or_else(||AppError::new("SCHEDULE_STALE","A calendar does not belong to this month."))?;
   if (old.year,old.month,old.office_id,old.revision)!=(cal.year,cal.month,cal.office_id,cal.revision){return Err(AppError::new("SCHEDULE_STALE","The saved month changed. Reload before saving your edits."));}
  }
  Ok(())
 }
 pub fn delete(&self,year:i32,month:u32,calendars:Vec<Calendar>)->AppResult<()> {
  month_dates(year,month)?;
  let mut c=self.database.connect()?;let tx=c.transaction_with_behavior(TransactionBehavior::Immediate).map_err(AppError::from_write_error)?;
  Self::check_snapshot(&Repo::list(&tx,Some(year),Some(month))?,&calendars)?;
  for cal in calendars {Repo::delete(&tx,cal.id,cal.revision)?;}
  tx.commit().map_err(AppError::from_write_error)
 }
}
