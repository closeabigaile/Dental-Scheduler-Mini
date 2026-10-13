// A view adapter only: validation and allocation belong to Rust.
export function dateKey(date) {
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}
export function printSchedules(calendars, offices, doctors, assistants) {
    return calendars.flatMap(calendar => calendar.days.map(day => ({
        officeName: offices.find(o => o.id === calendar.officeId)?.name || 'Office',
        schedule: {
            id: day.id, date: day.date, notes: day.notes,
            teams: Object.fromEntries(day.assignments.map(a => [a.id, a.assistants.map(member => {
                const resource = assistants.find(r => r.id === member.assistantId)
                return { id: member.assistantId, firstName: resource?.displayName || 'Assistant', lastName: '', schedulingResource: true, partialDayNote: member.partialDayNote }
            })])),
            teamNames: Object.fromEntries(day.assignments.map(a => {
                const doctor = doctors.find(d => d.id === a.doctorId)
                const doctorName = doctor ? `Dr. ${doctor.displayName.replace(/^Dr\.?\s+/i, '')}` : ''
                // Migrated rows retain the old doctor label verbatim. Render it
                // once when it already identifies the linked doctor.
                const label = a.label?.trim() === doctorName ? '' : a.label
                return [a.id, [doctorName, label].filter(Boolean).join(' · ')]
            }))
        }
    })))
}

export function applyAssignmentEdit(calendars, editor) {
    const next = structuredClone(calendars)
    let previousIndex = -1
    if (editor.sourceAssignmentId != null) {
        const source = next.find(c => c.id === editor.sourceCalendarId)
        const sourceDay = source?.days.find(d => d.date === editor.sourceDate)
        if (!sourceDay?.assignments.some(a => a.id === editor.sourceAssignmentId)) throw new Error('The original assignment changed. Reload the calendar.')
        previousIndex = sourceDay.assignments.findIndex(a => a.id === editor.sourceAssignmentId)
        sourceDay.assignments = sourceDay.assignments.filter(a => a.id !== editor.sourceAssignmentId)
    }
    const target = next.find(c => c.officeId === Number(editor.officeId))
    if (!target) throw new Error('Select an office in this saved month.')
    let day = target.days.find(d => d.date === editor.date)
    if (!day) { day = { id: 0, date: editor.date, notes: '', assignments: [] }; target.days.push(day) }
    const assignment = {
        id: 0, doctorId: editor.kind === 'note' ? null : Number(editor.doctorId) || null,
        teamId: editor.kind === 'note' ? null : Number(editor.teamId) || null,
        label: editor.label.trim(),
        assistants: editor.kind === 'note' ? [] : editor.assistants
    }
    if (target.id === editor.sourceCalendarId && day.date === editor.sourceDate && previousIndex >= 0) day.assignments.splice(previousIndex, 0, assignment)
    else day.assignments.push(assignment)
    return next
}
