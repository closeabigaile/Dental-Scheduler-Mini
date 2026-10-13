use crate::{error::{AppError, AppResult}, models::calendar::*};
use rusqlite::{params, Connection, OptionalExtension};

pub struct CalendarRepository;
impl CalendarRepository {
  pub fn list(c: &Connection, year: Option<i32>, month: Option<u32>) -> AppResult<Vec<Calendar>> {
    let mut q = c.prepare("SELECT id FROM calendars WHERE (?1 IS NULL OR year=?1) AND (?2 IS NULL OR month=?2) ORDER BY year,month,office_id").map_err(AppError::from_read_error)?;
    let ids = q.query_map(params![year,month], |r| r.get::<_,i64>(0)).map_err(AppError::from_read_error)?.collect::<Result<Vec<_>,_>>().map_err(AppError::from_read_error)?;
    ids.into_iter().map(|id| Self::get(c,id)).collect()
  }
  pub fn get(c: &Connection, id:i64) -> AppResult<Calendar> {
    let mut cal = c.query_row("SELECT id,year,month,office_id,published,revision FROM calendars WHERE id=?1", [id], |r| Ok(Calendar{id:r.get(0)?,year:r.get(1)?,month:r.get(2)?,office_id:r.get(3)?,published:r.get(4)?,revision:r.get(5)?,days:vec![],generation_issues:vec![]})).optional().map_err(AppError::from_read_error)?.ok_or_else(|| AppError::new("CALENDAR_NOT_FOUND","This calendar no longer exists."))?;
    let mut q = c.prepare("SELECT id,date,notes FROM calendar_days WHERE calendar_id=?1 ORDER BY date").map_err(AppError::from_read_error)?;
    cal.days = q.query_map([id], |r| Ok(CalendarDay{id:r.get(0)?,date:r.get(1)?,notes:r.get(2)?,assignments:vec![]})).map_err(AppError::from_read_error)?.collect::<Result<Vec<_>,_>>().map_err(AppError::from_read_error)?;
    for day in &mut cal.days {
      let mut q = c.prepare("SELECT id,doctor_id,team_id,label FROM daily_assignments WHERE day_id=?1 ORDER BY id").map_err(AppError::from_read_error)?;
      day.assignments=q.query_map([day.id],|r|Ok(CalendarAssignment{id:r.get(0)?,doctor_id:r.get(1)?,team_id:r.get(2)?,label:r.get(3)?,assistants:vec![]})).map_err(AppError::from_read_error)?.collect::<Result<Vec<_>,_>>().map_err(AppError::from_read_error)?;
      for assignment in &mut day.assignments {
        let mut q=c.prepare("SELECT assistant_id,partial_day_note FROM assignment_assistants WHERE assignment_id=?1 ORDER BY assistant_id").map_err(AppError::from_read_error)?;
        assignment.assistants=q.query_map([assignment.id],|r|Ok(AssistantAssignment{assistant_id:r.get(0)?,partial_day_note:r.get(1)?})).map_err(AppError::from_read_error)?.collect::<Result<Vec<_>,_>>().map_err(AppError::from_read_error)?;
      }
    }
    let mut q=c.prepare("SELECT date,code,doctor_id,message FROM calendar_generation_issues WHERE calendar_id=?1 ORDER BY id").map_err(AppError::from_read_error)?;
    cal.generation_issues=q.query_map([id],|r|Ok(GenerationIssue{date:r.get(0)?,code:r.get(1)?,doctor_id:r.get(2)?,message:r.get(3)?})).map_err(AppError::from_read_error)?.collect::<Result<Vec<_>,_>>().map_err(AppError::from_read_error)?;
    Ok(cal)
  }
  // All mutation methods are used inside a service-owned IMMEDIATE transaction.
  pub fn create(c:&Connection, year:i32, month:u32, office:i64) -> AppResult<Calendar> {
    c.execute("INSERT INTO calendars(year,month,office_id) VALUES(?1,?2,?3)",params![year,month,office]).map_err(|e| {
      let err=AppError::from_write_error(e);
      if err.code=="RECORD_CONFLICT" { AppError::new("CALENDAR_ALREADY_EXISTS","A calendar already exists for this office and month. Open the saved schedule.") } else {err}
    })?;
    Self::get(c,c.last_insert_rowid())
  }
  pub fn replace(c:&Connection, cal:&Calendar) -> AppResult<()> {
    let changed=c.execute("UPDATE calendars SET published=?1,revision=revision+1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND revision=?3",params![cal.published,cal.id,cal.revision]).map_err(AppError::from_write_error)?;
    if changed!=1 {return Err(AppError::new("SCHEDULE_STALE","This month changed since it was loaded. Keep your edits and reload before trying again."));}
    c.execute("DELETE FROM calendar_days WHERE calendar_id=?1",[cal.id]).map_err(AppError::from_write_error)?;
    for day in &cal.days {
      c.execute("INSERT INTO calendar_days(calendar_id,date,notes) VALUES(?1,?2,?3)",params![cal.id,day.date,day.notes]).map_err(AppError::from_write_error)?;
      let day_id=c.last_insert_rowid();
      for a in &day.assignments {
        c.execute("INSERT INTO daily_assignments(day_id,doctor_id,team_id,label) VALUES(?1,?2,?3,?4)",params![day_id,a.doctor_id,a.team_id,a.label]).map_err(AppError::from_write_error)?;
        let assignment_id=c.last_insert_rowid();
        for assistant in &a.assistants {
          c.execute("INSERT INTO assignment_assistants(assignment_id,assistant_id,partial_day_note) VALUES(?1,?2,?3)",params![assignment_id,assistant.assistant_id,assistant.partial_day_note]).map_err(AppError::from_write_error)?;
        }
      }
    }
    Ok(())
  }
  pub fn record_issues(c:&Connection,cal:&Calendar)->AppResult<()> {
    for issue in &cal.generation_issues {
      c.execute("INSERT INTO calendar_generation_issues(calendar_id,date,code,doctor_id,message) VALUES(?1,?2,?3,?4,?5)",params![cal.id,issue.date,issue.code,issue.doctor_id,issue.message]).map_err(AppError::from_write_error)?;
    }
    Ok(())
  }
  pub fn delete(c:&Connection,id:i64,revision:i64)->AppResult<()> {
    if c.execute("DELETE FROM calendars WHERE id=?1 AND revision=?2",params![id,revision]).map_err(AppError::from_write_error)?!=1 {
      return Err(AppError::new("SCHEDULE_STALE","The calendar changed. Reload before deleting."));
    }
    Ok(())
  }
}
