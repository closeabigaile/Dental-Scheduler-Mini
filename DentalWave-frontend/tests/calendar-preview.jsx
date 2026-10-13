// Browser-only development fixture. Not imported by the app or included in its build.
// This exercises React UI with a mock transport; it is not a native persistence test.
import React from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import DesktopCalendarPage from '../src/pages/DesktopCalendarPage'
const offices = [{ id: 1, name: 'Raleigh' }, { id: 2, name: 'Garner' }, { id: 3, name: 'Smithfield' }]
const doctors = ['Adams', 'Baker', 'Clark', 'Davis', 'Evans', 'Foster'].map((name, i) => ({ id: i + 1, displayName: name, active: true }))
const assistants = Array.from({ length: 24 }, (_, i) => ({ id: i + 1, displayName: ['Alice', 'Bea', 'Cara', 'Dena', 'Elsie', 'Faye'][i % 6] + ` ${i + 1}`, active: true }))
let id = 1
let calendars = offices.map(office => ({ id: office.id, year: 2026, month: 9, officeId: office.id, published: false, revision: 1, generationIssues: [], days: Array.from({ length: 30 }, (_, i) => i + 1).filter(day => [1, 2, 3, 4].includes(new Date(2026, 8, day).getDay())).map(day => ({ id: id++, date: `2026-09-${String(day).padStart(2, '0')}`, notes: '', assignments: [0, 1].map(slot => ({ id: id++, doctorId: (office.id - 1) * 2 + slot + 1, teamId: null, label: '', assistants: Array.from({ length: 4 }, (_, i) => ({ assistantId: (office.id - 1) * 8 + slot * 4 + i + 1, partialDayNote: '' })) })) })) }))
const fixtureKey = 'phase4-browser-fixture'
try { calendars = JSON.parse(localStorage.getItem(fixtureKey)) || calendars } catch { /* Start a new controlled fixture. */ }
window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' } },
    transformCallback: () => 1, unregisterCallback: () => {},
    invoke: async (command, args = {}) => {
        if (command === 'get_offices') return offices
        if (command === 'get_doctors') return doctors
        if (command === 'get_assistants') return assistants
        if (command === 'get_teams') return [{ id: 1, name: 'Adams team', active: true, doctor: doctors[0], assistants: assistants.slice(0, 4) }]
        if (command === 'list_calendars') return structuredClone(calendars.filter(c => c.year === args.year && c.month === args.month))
        if (command === 'save_calendar_month') {
            const assignment = args.calendars.flatMap(c => c.days.flatMap(d => d.assignments)).find(a => a.doctorId && !a.assistants.length)
            if (assignment) throw { code: 'ASSIGNMENT_CONFLICT', message: 'Assign at least one assistant to the doctor.' }
            calendars = structuredClone(args.calendars).map(c => ({ ...c, revision: c.revision + 1 }))
            localStorage.setItem(fixtureKey, JSON.stringify(calendars))
            return structuredClone(calendars)
        }
        if (command === 'print_calendar') { window.print(); return }
        if (command.startsWith('plugin:event|')) return 1
        throw { code: 'FIXTURE_ONLY', message: 'This action is not implemented by the browser fixture.' }
    }
}
createRoot(document.getElementById('root')).render(<React.StrictMode><BrowserRouter><DesktopCalendarPage previewMode={new URLSearchParams(location.search).get('preview') === '1'} /></BrowserRouter></React.StrictMode>)
