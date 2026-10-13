import test from 'node:test'
import assert from 'node:assert/strict'
import { applyAssignmentEdit, printSchedules } from './desktopCalendarUtils.js'
const saved = [{ id: 1, officeId: 10, days: [{ id: 1, date: '2026-09-01', notes: 'Test note', assignments: [{ id: 1, doctorId: 20, label: 'Team A', teamId: null, assistants: [{ assistantId: 30, partialDayNote: 'Until noon' }] }] }] }, { id: 2, officeId: 11, days: [] }]
test('editor move preserves the saved snapshot until a successful save and moves all staffing', () => {
    const before = structuredClone(saved)
    const edited = applyAssignmentEdit(saved, { sourceCalendarId: 1, sourceDate: '2026-09-01', sourceAssignmentId: 1, date: '2026-09-02', officeId: 11, kind: 'staff', doctorId: 21, teamId: '', label: 'Edited', assistants: [{ assistantId: 31, partialDayNote: '' }] })
    assert.deepEqual(saved, before)
    assert.equal(edited[0].days[0].assignments.length, 0)
    assert.equal(edited[1].days[0].date, '2026-09-02')
    assert.equal(edited[1].days[0].assignments[0].doctorId, 21)
    assert.equal(edited[1].days[0].assignments[0].assistants[0].assistantId, 31)
    assert.throws(() => applyAssignmentEdit(saved, { sourceCalendarId: 1, sourceDate: '2026-09-01', sourceAssignmentId: 999 }))
})
test('print adapter derives exact saved dates, doctor, team, office, assistant and partial-day note', () => {
    const rows = printSchedules(saved, [{ id: 10, name: 'Test Office' }], [{ id: 20, displayName: 'Test Doctor' }], [{ id: 30, displayName: 'Test Assistant' }])
    assert.equal(rows.length, 1)
    assert.equal(rows[0].officeName, 'Test Office')
    assert.equal(rows[0].schedule.date, '2026-09-01')
    assert.equal(rows[0].schedule.teamNames[1], 'Dr. Test Doctor · Team A')
    assert.equal(rows[0].schedule.teams[1][0].firstName, 'Test Assistant')
    assert.equal(rows[0].schedule.teams[1][0].partialDayNote, 'Until noon')
    assert.equal(rows[0].schedule.notes, 'Test note')
})
test('closure editing clears resource relationships without mutating saved data', () => {
    const edited = applyAssignmentEdit(saved, { date: '2026-09-03', officeId: 10, kind: 'note', label: 'Closed', doctorId: 20, teamId: 3, assistants: [{ assistantId: 30 }] })
    assert.deepEqual(edited[0].days[1].assignments[0], { id: 0, doctorId: null, teamId: null, label: 'Closed', assistants: [] })
})

test('migrated doctor labels print once and preserve original saved text', () => {
    const migrated = structuredClone(saved)
    migrated[0].days[0].assignments[0].label = 'Dr. Alpha'
    const rows = printSchedules(migrated, [{ id: 10, name: 'Fixture North' }], [{ id: 20, displayName: 'Alpha' }], [{ id: 30, displayName: 'One' }])
    assert.equal(rows[0].schedule.teamNames[1], 'Dr. Alpha')
    assert.equal(rows[0].schedule.teams[1][0].partialDayNote, 'Until noon')
    assert.equal(migrated[0].days[0].assignments[0].label, 'Dr. Alpha')
})
