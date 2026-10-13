import { useStorage, calendarDraft } from '../storage/context'
import '../App.css'
import './DesktopCalendarPage.css'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router'
import ManagerHeader from '../components/ManagerHeader'
import PrintableSchedule from '../components/PrintableSchedule'
import { getAllOffices } from '../services/OfficeService'
import { getResources, getReusableTeams } from '../services/ManagerSchedulerService'
import { loadCalendarMonth, createCalendarMonth, saveCalendarMonth, deleteCalendarMonth, printCalendar } from '../services/DesktopCalendarService'
import { applyAssignmentEdit, dateKey, printSchedules } from '../utils/desktopCalendarUtils'
import { buildPrintWeeks, sortOfficePrintSchedules, createDoctorPrintAbbreviationMap, getDoctorPrintAbbreviation } from '../utils/printScheduleUtils'

function pendingDraft() { return calendarDraft.current }
const message = error => error?.message || 'The schedule operation failed.'
export default function DesktopCalendarPage({ previewMode = false }) {
    const storage = useStorage()
    const readOnly = storage?.readOnly || false
    const setStorageDirty = storage?.setDirty
    const [params] = useSearchParams()
    const navigate = useNavigate()
    const [period, setPeriod] = useState(pendingDraft()?.period || params.get('month') || dateKey(new Date()).slice(0, 7))
    const [year, month] = period.split('-').map(Number)
    const [calendars, setCalendars] = useState([])
    const [offices, setOffices] = useState([])
    const [doctors, setDoctors] = useState([])
    const [assistants, setAssistants] = useState([])
    const [teams, setTeams] = useState([])
    const [selectedDate, setSelectedDate] = useState('')
    const [officeFilter, setOfficeFilter] = useState('')
    const [editor, setEditor] = useState(() => pendingDraft()?.editor || null)
    const [busy, setBusy] = useState(true)
    const [loaded, setLoaded] = useState(false)
    const [saveState, setSaveState] = useState('Saved')
    const [error, setError] = useState('')
    const [notice, setNotice] = useState('')
    const [deleteIntent, setDeleteIntent] = useState(false)
    const [pendingNavigation, setPendingNavigation] = useState(null)
    const [printReady, setPrintReady] = useState(false)
    const [assistantSize, setAssistantSize] = useState('medium')
    const dirty = editor !== null
    const dirtyRef = useRef(dirty)
    useEffect(() => {
        dirtyRef.current = dirty
        calendarDraft.current = editor ? { period, editor } : null
        setStorageDirty?.(dirty)
    }, [dirty, editor, period, setStorageDirty])
    const monthLabel = new Date(year, month - 1, 1).toLocaleDateString('en-US', { month: 'long', year: 'numeric' })

    useEffect(() => {
        let active = true
        // A new period starts a fresh asynchronous load, including on preview entry.
        // eslint-disable-next-line react-hooks/set-state-in-effect
        setBusy(true); setLoaded(false); setPrintReady(false); setError('')
        Promise.all([loadCalendarMonth(year, month), getAllOffices(), getResources('DOCTOR'), getResources('ASSISTANT'), getReusableTeams()])
            .then(([c, o, d, a, t]) => {
                if (!active) return
                setCalendars(c.data); setOffices(o.data); setDoctors(d.data); setAssistants(a.data); setTeams(t.data)
                setSaveState(dirtyRef.current ? 'Unsaved Changes' : 'Saved'); setLoaded(true)
            }).catch(e => { if (active) setError(message(e)) })
            .finally(() => { if (active) setBusy(false) })
        return () => { active = false }
    }, [year, month, previewMode])

    useLayoutEffect(() => {
        if (!previewMode || !loaded) return
        // A native WebView can throttle animation frames while occluded. Measure
        // the committed preview layout without leaving printing disabled forever.
        const clipped = [...document.querySelectorAll('.manager-calendar-print .print-day-cell, .manager-calendar-print .print-office-block')]
            .some(el => el.scrollHeight > el.clientHeight + 2 || el.scrollWidth > el.clientWidth + 2)
        // eslint-disable-next-line react-hooks/set-state-in-effect
        setPrintReady(true)
        if (clipped) setNotice('Some cells are crowded. A complete staffing detail sheet is included after the monthly grid so no saved assignment is omitted.')
    }, [loaded, previewMode, assistantSize, calendars])

    function leave(action) {
        if (dirty) { setPendingNavigation(() => action); return }
        action()
    }
    function captureNavigation(event) {
        const link = event.target.closest('a[href]')
        if (dirty && link) { event.preventDefault(); setPendingNavigation(() => () => navigate(link.getAttribute('href'))) }
    }
    function changePeriod(value) {
        if (!/^\d{4}-\d{2}$/.test(value)) return
        leave(() => { setPeriod(value); setSelectedDate(''); setNotice(''); setDeleteIntent(false) })
    }
    async function create(generate) {
        if (readOnly) return
        setBusy(true); setError(''); setSaveState('Saving…')
        try {
            const response = await createCalendarMonth(year, month, generate)
            setCalendars(response.data.calendars); setSaveState('Saved')
            setNotice(generate ? 'Draft generated and saved. Review the staffing and generation report below.' : 'Empty month created and saved. Select a day to add staffing.')
        } catch (e) {
            setError(message(e)); setSaveState('Save Failed')
            if (e.code === 'CALENDAR_ALREADY_EXISTS') {
                try { setCalendars((await loadCalendarMonth(year, month)).data); setNotice('Opened the existing saved month.') } catch (loadError) { setError(message(loadError)) }
            }
        } finally { setBusy(false) }
    }
    async function save(next, closeEditor = false) {
        if (readOnly) return
        setBusy(true); setError(''); setSaveState('Saving…')
        try {
            const response = await saveCalendarMonth(year, month, next)
            setCalendars(response.data); setSaveState('Saved')
            if (closeEditor) setEditor(null)
            return true
        } catch (e) { setError(message(e)); setSaveState('Save Failed'); return false }
        finally { setBusy(false) }
    }
    function openEditor(calendar, day, assignment) {
        if (readOnly) return
        leave(() => {
            setError(''); setSaveState('Unsaved Changes')
            setEditor({
                sourceCalendarId: calendar?.id, sourceDate: day?.date, sourceAssignmentId: assignment?.id,
                date: day?.date || selectedDate, officeId: calendar?.officeId || calendars[0]?.officeId || '',
                kind: assignment && !assignment.doctorId ? 'note' : 'staff',
                doctorId: assignment?.doctorId || '', teamId: assignment?.teamId || '',
                label: assignment?.label || '', assistants: structuredClone(assignment?.assistants || [])
            })
        })
    }
    function edit(change) { setEditor(current => ({ ...current, ...change })); setSaveState('Unsaved Changes') }
    function applyTeam(id) {
        const team = teams.find(t => t.id === Number(id))
        edit(team ? { teamId: team.id, doctorId: team.doctor.id, label: team.name, assistants: team.assistants.map(a => ({ assistantId: a.id, partialDayNote: '' })) } : { teamId: '' })
    }
    async function saveEditor(event) {
        event.preventDefault()
        if (editor.kind === 'staff' && !editor.doctorId) { setError('Select a doctor.'); return }
        try { await save(applyAssignmentEdit(calendars, editor), true) } catch (e) { setError(message(e)); setSaveState('Save Failed') }
    }
    async function removeAssignment() {
        if (readOnly) return
        const next = structuredClone(calendars)
        const day = next.find(c => c.id === editor.sourceCalendarId)?.days.find(d => d.date === editor.sourceDate)
        if (!day) return
        day.assignments = day.assignments.filter(a => a.id !== editor.sourceAssignmentId)
        await save(next, true)
    }
    async function removeMonth() {
        if (readOnly) return
        setBusy(true); setError('')
        try { await deleteCalendarMonth(year, month, calendars); setCalendars([]); setDeleteIntent(false); setNotice('Month deleted.'); setSaveState('Saved') }
        catch (e) { setError(message(e)); setSaveState('Save Failed') } finally { setBusy(false) }
    }
    const rows = printSchedules(calendars, offices, doctors, assistants)
    const getOfficeSchedules = date => sortOfficePrintSchedules(rows.filter(r => r.schedule.date === dateKey(date)))
    const issues = calendars.flatMap(c => c.generationIssues.map((issue, i) => ({ ...issue, key: `${c.id}-${i}`, office: offices.find(o => o.id === c.officeId)?.name })))
    const days = new Date(year, month, 0).getDate()
    const cells = [...Array(new Date(year, month - 1, 1).getDay()).fill(null), ...Array.from({ length: days }, (_, i) => i + 1)]
    while (cells.length % 7) cells.push(null)
    const abbreviations = createDoctorPrintAbbreviationMap(doctors.map(d => d.displayName))
    const printIdentifier = name => name.startsWith('Dr. ') ? [getDoctorPrintAbbreviation(name.split(' · ')[0], abbreviations), ...name.split(' · ').slice(1)].join(' · ') : name
    const colorNames = [...new Set(rows.flatMap(r => Object.values(r.schedule.teamNames).map(printIdentifier)))]
    const sheet = <PrintableSchedule monthIndex={month - 1} weeks={buildPrintWeeks(year, month - 1)} monthLabel={monthLabel}
        printableDayNotes={rows.filter(r => r.schedule.notes).map(r => ({ id: r.schedule.id, date: new Date(`${r.schedule.date}T12:00:00`), officeName: r.officeName, note: r.schedule.notes }))}
        approvedRequests={[]} getDateKey={dateKey} getOfficeSchedules={getOfficeSchedules}
        getTeamName={(schedule, id) => schedule.teamNames[id]} getDoctorIdentifier={printIdentifier}
        getPrintableAssistants={(_name, members) => members} getPrintableFirstName={a => a.firstName}
        getTeamColorClass={name => `print-team-color-${Math.max(0, colorNames.indexOf(name)) % 10}`} assistantNameSize={assistantSize} preview />

    if (previewMode) return <main className="print-preview-page desktop-preview">
        <div className="print-preview-toolbar">
            <button onClick={() => navigate(`/manager/schedule?month=${period}`)}>Back to schedule</button>
            <label>Assistant text <select value={assistantSize} onChange={e => setAssistantSize(e.target.value)}><option>small</option><option>medium</option><option>large</option></select></label>
            <button disabled={busy || !loaded || !printReady || !calendars.length} onClick={async () => { try { await printCalendar() } catch (e) { setError(message(e)) } }}>Print saved schedule</button>
            <small>Saved schedule · {monthLabel} · Monthly grid followed by complete staffing details, including Friday–Sunday.</small>
            {error && <p role="alert">{error}</p>}{notice && <p role="status">{notice}</p>}
        </div>
        {loaded && <><div className="print-preview-sheet">{sheet}</div>
            <section className="desktop-print-details"><h1>{monthLabel} — staffing details</h1>
                {rows.filter(r => r.schedule.notes || Object.keys(r.schedule.teams).length).map(({ officeName, schedule }) => <section key={schedule.id}>
                    <h2>{schedule.date} · {officeName}</h2>{schedule.notes && <p>{schedule.notes}</p>}
                    {Object.entries(schedule.teams).map(([id, members]) => <p key={id}><strong>{schedule.teamNames[id]}</strong>{members.length ? ': ' : ''}{members.map(a => `${a.firstName}${a.partialDayNote ? ` (${a.partialDayNote})` : ''}`).join(', ')}</p>)}
                </section>)}
            </section></>}
    </main>

    return <div className="calendar-page" onClickCapture={captureNavigation}>
        <ManagerHeader />
        <main className="manager-schedule-builder desktop-calendar">
            <header className="universal-calendar-heading"><h1>Monthly Schedule</h1><span className="universal-status-badge">{calendars.length ? calendars.every(c => c.published) ? 'Published' : 'Draft' : 'Not Created'}</span></header>
            <div className="calendar-controls">
                <button disabled={busy} onClick={() => changePeriod(dateKey(new Date(year, month - 2, 1)).slice(0, 7))}>Previous month</button>
                <label>Month <input type="month" value={period} min="1900-01" max="9999-12" disabled={busy} onChange={e => changePeriod(e.target.value)} /></label>
                <button disabled={busy} onClick={() => changePeriod(dateKey(new Date(year, month, 1)).slice(0, 7))}>Next month</button>
            </div>
            <div className="desktop-calendar-actions">
                <span role="status">{saveState}</span>
                {!calendars.length && <><button disabled={busy || readOnly || !loaded} onClick={() => create(true)}>Generate editable draft</button><button disabled={busy || readOnly || !loaded} onClick={() => create(false)}>Create empty month</button></>}
                {!!calendars.length && <>
                    <button disabled={busy || readOnly || dirty} onClick={() => save(calendars.map(c => ({ ...c, published: false })))}>Save as Draft</button>
                    <button disabled={busy || readOnly || dirty} onClick={() => save(calendars.map(c => ({ ...c, published: true })))}>Publish month</button>
                    <button disabled={busy || dirty} onClick={() => navigate(`/manager/schedule/preview?month=${period}`)}>Preview saved schedule</button>
                    <button disabled={busy || readOnly || dirty} onClick={() => setDeleteIntent(true)}>Delete month…</button>
                </>}
            </div>
            {error && <p className="error-message" role="alert">{error}</p>}{notice && <p role="status">{notice}</p>}
            {deleteIntent && <section className="desktop-confirm" role="alertdialog" aria-label="Delete month"><p>Delete every office schedule in {monthLabel}? This cannot be undone.</p><button disabled={busy || readOnly} onClick={removeMonth}>Delete {monthLabel}</button><button disabled={busy || readOnly} onClick={() => setDeleteIntent(false)}>Cancel</button></section>}
            {pendingNavigation && <section className="desktop-confirm" role="alertdialog" aria-label="Unsaved changes"><p>Unsaved changes. Save the open assignment or discard it before leaving.</p><button onClick={() => { setEditor(null); setSaveState('Saved'); const action = pendingNavigation; setPendingNavigation(null); action() }}>Discard and continue</button><button onClick={() => setPendingNavigation(null)}>Keep editing</button></section>}
            {!!issues.length && <details><summary>Generation report ({issues.length} notices) — review before publishing</summary><p>These notices describe the original generation. Saved manual corrections are shown in the calendar.</p><ul>{issues.map(i => <li key={i.key}>{i.date} · {i.office} · {i.code}: {i.message}</li>)}</ul></details>}
            <label>Location <select value={officeFilter} onChange={e => setOfficeFilter(e.target.value)}><option value="">All locations</option>{offices.map(o => <option key={o.id} value={o.id}>{o.name}</option>)}</select></label>
            <div className="calendar-grid universal-calendar-grid">
                {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map(d => <div className="calendar-day-header" key={d}>{d}</div>)}
                {cells.map((d, index) => {
                    const date = d ? `${period}-${String(d).padStart(2, '0')}` : ''
                    const matching = calendars.filter(c => !officeFilter || c.officeId === Number(officeFilter))
                    return <button key={index} disabled={!d || busy || !loaded} className={`calendar-day ${!d ? 'calendar-day-empty' : ''} ${selectedDate === date ? 'calendar-day-selected' : ''}`} onClick={() => leave(() => setSelectedDate(date))}>
                        <span>{d}</span>{matching.map(c => {
                            const day = c.days.find(day => day.date === date)
                            if (!day?.assignments.length) return null
                            return <div className="universal-calendar-office" key={c.id}><strong>{offices.find(o => o.id === c.officeId)?.name}</strong>{day.assignments.map(a => <span key={a.id}>{a.doctorId ? `Dr. ${doctors.find(d => d.id === a.doctorId)?.displayName || a.doctorId}` : a.label}{a.doctorId ? ` · ${a.assistants.length} assistants` : ''}</span>)}</div>
                        })}
                    </button>
                })}
            </div>
            {selectedDate && <section className="desktop-day-panel">
                <h2>{selectedDate}</h2><button disabled={busy || readOnly || !calendars.length} onClick={() => openEditor()}>Add assignment or note</button>
                {calendars.filter(c => !officeFilter || c.officeId === Number(officeFilter)).map(c => {
                    const day = c.days.find(d => d.date === selectedDate)
                    return <section className="universal-location-card" key={c.id}><h3>{offices.find(o => o.id === c.officeId)?.name}</h3>
                        {day?.assignments.map(a => <div className="universal-team-block" key={a.id}>
                            <strong>{a.doctorId ? `Dr. ${doctors.find(d => d.id === a.doctorId)?.displayName || a.doctorId}` : a.label}</strong>{a.doctorId && a.label && <span> · {a.label}</span>}
                            <p>{a.assistants.map(m => `${assistants.find(r => r.id === m.assistantId)?.displayName || m.assistantId}${m.partialDayNote ? ` (${m.partialDayNote})` : ''}`).join(', ')}</p>
                            <button disabled={busy || readOnly} onClick={() => openEditor(c, day, a)}>Edit assignment</button>
                        </div>)}
                        {!day?.assignments.length && <p>No assignments saved.</p>}
                    </section>
                })}
            </section>}
            {editor && <form className="desktop-assignment-editor" onSubmit={saveEditor}>
                <h2>{editor.sourceAssignmentId ? 'Edit assignment' : 'Add assignment'}</h2>
                <fieldset disabled={busy || readOnly}>
                    <label>Date <input type="date" required min={`${period}-01`} max={`${period}-${days}`} value={editor.date} onChange={e => edit({ date: e.target.value })} /></label>
                    <label>Office <select required value={editor.officeId} onChange={e => edit({ officeId: Number(e.target.value) })}>{calendars.map(c => <option key={c.id} value={c.officeId}>{offices.find(o => o.id === c.officeId)?.name}</option>)}</select></label>
                    <label>Row type <select value={editor.kind} onChange={e => edit({ kind: e.target.value })}><option value="staff">Doctor and assistants</option><option value="note">Closure or note</option></select></label>
                    {editor.kind === 'staff' && <>
                        <label>Reusable team <select value={editor.teamId} onChange={e => applyTeam(e.target.value)}><option value="">Custom staffing</option>{teams.map(t => <option key={t.id} value={t.id} disabled={!t.active}>{t.name}{!t.active ? ' (inactive)' : ''}</option>)}</select></label>
                        <label>Doctor <select required value={editor.doctorId} onChange={e => edit({ doctorId: Number(e.target.value), teamId: '' })}><option value="">Select doctor</option>{doctors.map(d => <option key={d.id} value={d.id} disabled={!d.active}>{d.displayName}{!d.active ? ' (inactive)' : ''}</option>)}</select></label>
                    </>}
                    <label>{editor.kind === 'note' ? 'Note or closure text' : 'Team label (optional)'} <input maxLength={200} required={editor.kind === 'note'} value={editor.label} onChange={e => edit({ label: e.target.value })} /></label>
                    {editor.kind === 'staff' && <div className="desktop-assistant-options"><h3>Assistants</h3><p>Changing staffing switches to a custom team. Partial-day notes do not permit double booking.</p>
                        {assistants.map(a => {
                            const member = editor.assistants.find(m => m.assistantId === a.id)
                            return <div key={a.id}><label><input type="checkbox" checked={!!member} disabled={!a.active && !member} onChange={e => edit({ teamId: '', assistants: e.target.checked ? [...editor.assistants, { assistantId: a.id, partialDayNote: '' }] : editor.assistants.filter(m => m.assistantId !== a.id) })} />{a.displayName}{!a.active ? ' (inactive)' : ''}</label>
                                {member && <input aria-label={`${a.displayName} partial-day note`} placeholder="Partial-day note" maxLength={40} value={member.partialDayNote} onChange={e => edit({ assistants: editor.assistants.map(m => m.assistantId === a.id ? { ...m, partialDayNote: e.target.value } : m) })} />}
                            </div>
                        })}
                    </div>}
                    <button type="submit">{busy ? 'Saving…' : 'Save assignment'}</button>
                    <button type="button" onClick={() => { setEditor(null); setSaveState('Saved') }}>Discard changes</button>
                    {editor.sourceAssignmentId && <button type="button" onClick={removeAssignment}>Remove this assignment</button>}
                </fieldset>
            </form>}
        </main>
    </div>
}
