import { desktopCall } from './DesktopService'

// Desktop calendar transport. The old CalendarService/ScheduleService remain web-only.
export const loadCalendarMonth = (year, month) => desktopCall('list_calendars', { year, month })
export const listDesktopCalendars = () => desktopCall('list_calendars')
export const getDesktopCalendar = (id) => desktopCall('get_calendar', { id })
export const createCalendarMonth = (year, month, generate = true) => desktopCall('create_calendar_month', { year, month, generate })
export const saveCalendarMonth = (year, month, calendars) => desktopCall('save_calendar_month', { year, month, calendars })
export const deleteCalendarMonth = (year, month, calendars) => desktopCall('delete_calendar_month', { year, month, calendars })
export const printCalendar = () => desktopCall('print_calendar')
