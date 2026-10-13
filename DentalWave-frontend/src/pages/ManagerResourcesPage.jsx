import { useStorage, clearUnsaved } from '../storage/context'
import { useEffect, useState } from 'react'
import ManagerHeader from '../components/ManagerHeader'
import { getAllOffices } from '../services/OfficeService'
import {
    createResource,
    deleteResource,
    getResources,
    updateResource
} from '../services/ManagerSchedulerService'
import {
    createRotationGroup,
    getDoctorRules,
    getRotationGroups,
    replaceDoctorRules
} from '../services/DoctorWorkRuleService'

const DAYS = ['MONDAY', 'TUESDAY', 'WEDNESDAY', 'THURSDAY']
const STANDARD_ROTATION_NAME = 'Standard alternating weeks'
const STANDARD_ROTATION_ANCHOR = '2024-01-01'

const empty = (type) => ({
    type,
    firstName: '',
    lastName: '',
    displayName: '',
    active: true,
    defaultOffice: null,
    offices: [],
    color: '#65a9b8',
    notes: ''
})

const emptyPattern = () => DAYS.map((day) => ({
    day,
    works: false,
    scheduleType: 'SAME_OFFICE',
    officeId: '',
    alternateOfficeId: ''
}))

const friendlyDay = (day) => day[0] + day.slice(1).toLowerCase()

function usesAdvancedPattern(rules) {
    return (rules || []).some((rule) =>
        !rule.active || rule.effectiveStartDate || rule.effectiveEndDate || !DAYS.includes(rule.dayOfWeek)
        || (rule.recurrenceType === 'ROTATING' && (
            rule.rotationGroup?.numberOfWeeks !== 2 || rule.rotationPosition > 2
            || rule.rotationGroup?.anchorDate !== STANDARD_ROTATION_ANCHOR
            || !rule.rotationGroup?.active || rule.workStatus !== 'WORKING'
        ))
    ) || DAYS.some(day => {
        const rows = (rules || []).filter(rule => rule.dayOfWeek === day)
        const rotating = rows.filter(rule => rule.recurrenceType === 'ROTATING')
        return rows.length > 2 || (rotating.length > 0 && (
            rotating.length !== 2 || rows.length !== 2
            || new Set(rotating.map(rule => rule.rotationGroup?.id)).size !== 1
            || new Set(rotating.map(rule => rule.rotationPosition)).size !== 2
        ))
    })
}

function patternFromRules(rules) {
    return DAYS.map((day) => {
        const dayRules = (rules || []).filter(
            (rule) => rule.active && rule.dayOfWeek === day
        )
        const rotatingRules = dayRules
            .filter((rule) =>
                rule.workStatus === 'WORKING' && rule.recurrenceType === 'ROTATING'
            )
            .sort((left, right) => left.rotationPosition - right.rotationPosition)

        if (rotatingRules.length > 0) {
            const weekA = rotatingRules.find((rule) => rule.rotationPosition === 1)
            const weekB = rotatingRules.find((rule) => rule.rotationPosition === 2)
            return {
                day,
                works: true,
                scheduleType: 'ALTERNATING',
                officeId: weekA?.office?.id ? String(weekA.office.id) : '',
                alternateOfficeId: weekB?.office?.id ? String(weekB.office.id) : ''
            }
        }

        const everyWeekRule = dayRules.find(
            (rule) => rule.recurrenceType === 'EVERY_WEEK'
        )
        return {
            day,
            works: everyWeekRule?.workStatus === 'WORKING',
            scheduleType: 'SAME_OFFICE',
            officeId: everyWeekRule?.office?.id ? String(everyWeekRule.office.id) : '',
            alternateOfficeId: ''
        }
    })
}

export default function ManagerResourcesPage({ type }) {const storage = useStorage(); const readOnly = storage?.readOnly || false;
    const isDoctor = type === 'DOCTOR'
    const title = isDoctor ? 'Doctors' : 'Assistants'
    const [items, setItems] = useState([])
    const [offices, setOffices] = useState([])
    const [form, setForm] = useState(empty(type))
    const [weeklyPattern, setWeeklyPattern] = useState(emptyPattern)
    const [error, setError] = useState('')
    const [saving, setSaving] = useState(false)
    const [loadingPattern, setLoadingPattern] = useState(false)
    const [advancedPattern, setAdvancedPattern] = useState(false)

    const load = () => getResources(type)
        .then((response) => setItems(response.data || []))
        .catch(() => setError(`Unable to load ${title.toLowerCase()}.`))

    useEffect(() => {
        getResources(type)
            .then((response) => setItems(response.data || []))
            .catch(() => setError(`Unable to load ${title.toLowerCase()}.`))
        getAllOffices()
            .then((response) => setOffices(response.data || []))
            .catch(() => setError('Unable to load office locations.'))
    }, [type, title])

    function resetForm() {
        setForm(empty(type))
        setWeeklyPattern(emptyPattern())
        setError('')
        setLoadingPattern(false)
        setAdvancedPattern(false)
    }

    function updatePattern(day, changes) {
        setWeeklyPattern((current) => current.map((entry) =>
            entry.day === day ? { ...entry, ...changes } : entry
        ))
    }

    async function selectItem(item) {
        setForm({
            ...item,
            displayName: item.displayName
                || `${item.firstName || ''} ${item.lastName || ''}`.trim(),
            offices: item.offices || []
        })
        setError('')

        if (!isDoctor) return
        setLoadingPattern(true)
        try {
            const response = await getDoctorRules(item.id)
            const rules = response.data || []
            setAdvancedPattern(usesAdvancedPattern(rules))
            setWeeklyPattern(patternFromRules(rules))
        } catch {
            setError('Unable to load this doctor’s work schedule.')
            setWeeklyPattern(emptyPattern())
        } finally {
            setLoadingPattern(false)
        }
    }

    function validateDoctorPattern() {
        for (const entry of weeklyPattern) {
            if (!entry.works) continue

            if (!entry.officeId) {
                throw new Error(`Choose an office for ${friendlyDay(entry.day)}.`)
            }
            if (entry.scheduleType === 'ALTERNATING') {
                if (!entry.alternateOfficeId) {
                    throw new Error(`Choose the Week B office for ${friendlyDay(entry.day)}.`)
                }
                if (entry.officeId === entry.alternateOfficeId) {
                    throw new Error(
                        `${friendlyDay(entry.day)} must use two different offices when alternating.`
                    )
                }
            }
        }
    }

    async function getStandardRotation() {
        const response = await getRotationGroups()
        const existing = (response.data || []).find(
            (group) => group.name === STANDARD_ROTATION_NAME
        )
        if (existing) {
            if (existing.numberOfWeeks !== 2 || existing.anchorDate !== STANDARD_ROTATION_ANCHOR) {
                throw new Error(
                    'The shared alternating-week pattern has changed. Restore it to two weeks before saving.'
                )
            }
            return existing
        }

        const created = await createRotationGroup({
            name: STANDARD_ROTATION_NAME,
            numberOfWeeks: 2,
            anchorDate: STANDARD_ROTATION_ANCHOR,
            active: true
        })
        return created.data
    }

    function buildDoctorRules(doctorId, rotationGroup) {
        return weeklyPattern.flatMap((entry) => {
            const common = {
                doctor: { id: doctorId },
                dayOfWeek: entry.day,
                active: true
            }

            if (!entry.works) {
                return [{
                    ...common,
                    workStatus: 'NOT_WORKING',
                    office: null,
                    recurrenceType: 'EVERY_WEEK',
                    rotationGroup: null,
                    rotationPosition: null
                }]
            }

            if (entry.scheduleType === 'ALTERNATING') {
                return [
                    {
                        ...common,
                        workStatus: 'WORKING',
                        office: { id: Number(entry.officeId) },
                        recurrenceType: 'ROTATING',
                        rotationGroup,
                        rotationPosition: 1
                    },
                    {
                        ...common,
                        workStatus: 'WORKING',
                        office: { id: Number(entry.alternateOfficeId) },
                        recurrenceType: 'ROTATING',
                        rotationGroup,
                        rotationPosition: 2
                    }
                ]
            }

            return [{
                ...common,
                workStatus: 'WORKING',
                office: { id: Number(entry.officeId) },
                recurrenceType: 'EVERY_WEEK',
                rotationGroup: null,
                rotationPosition: null
            }]
        })
    }

    async function submit(event) {
        event.preventDefault()
        setError('')
        setSaving(true)

        try {
            let rotationGroup = null
            if (isDoctor && !advancedPattern) {
                validateDoctorPattern()
                if (weeklyPattern.some(
                    (entry) => entry.works && entry.scheduleType === 'ALTERNATING'
                )) {
                    rotationGroup = await getStandardRotation()
                }
            }

            const payload = {
                ...form,
                defaultOffice: form.defaultOffice?.id ? form.defaultOffice : null,
                offices: form.offices || [],
                firstName: form.displayName,
                lastName: ''
            }
            const response = form.id
                ? await updateResource(form.id, payload)
                : await createResource(payload)

            if (isDoctor && !advancedPattern) {
                await replaceDoctorRules(
                    response.data.id,
                    buildDoctorRules(response.data.id, rotationGroup)
                )
            }

            clearUnsaved();resetForm()
            await load()
        } catch (requestError) {
            const fallbackMessage = form.id
                ? `${isDoctor ? 'Doctor' : 'Assistant'} changes could not be saved. Please check the entered information and try again.`
                : `${isDoctor ? 'Doctor' : 'Assistant'} could not be added. Please check the entered information and try again.`
            const localValidationMessage = !requestError.response
                && requestError.message !== 'Network Error'
                ? requestError.message
                : ''
            setError(
                requestError.response?.data?.message
                || requestError.response?.data?.detail
                || localValidationMessage
                || fallbackMessage
            )
        } finally {
            setSaving(false)
        }
    }

    async function remove() {
        if (!form.id || !window.confirm(
            `Remove ${form.displayName || 'this doctor'}? This cannot be undone.`
        )) return

        setError('')
        setSaving(true)
        try {
            await deleteResource(form.id, type)
            clearUnsaved();resetForm()
            await load()
        } catch (requestError) {
            setError(
                requestError.response?.data?.message
                || 'This doctor could not be removed.'
            )
        } finally {
            setSaving(false)
        }
    }

    return (
        <div className="calendar-page">
            <ManagerHeader />
            <main className="lite-shell">
                <div className="page-heading">
                    <div>
                        <p className="eyebrow">Scheduling resources</p>
                        <h1>{title}</h1>
                    </div>
                    <button className="lite-button" onClick={resetForm}>
                        Add {isDoctor ? 'doctor' : 'assistant'}
                    </button>
                </div>

                {error && <p className="form-error">{error}</p>}

                <div className="resource-layout">
                    <section className="resource-list">
                        {items.length ? items.map((item) => (
                            <button
                                key={item.id}
                                className="resource-row"
                                onClick={() => selectItem(item)}
                            >
                                <span
                                    className="color-dot"
                                    style={{ background: item.color }}
                                />
                                <span>
                                    <strong>
                                        {item.displayName
                                            || `${item.firstName} ${item.lastName}`}
                                    </strong>
                                    <small>
                                        {item.defaultOffice?.name || 'Work pattern set by day'}
                                        {' · '}
                                        {item.active ? 'Active' : 'Inactive'}
                                    </small>
                                </span>
                                <span>Edit</span>
                            </button>
                        )) : (
                            <div className="empty-state">No {title.toLowerCase()} yet.</div>
                        )}
                    </section>

                    <fieldset className="storage-readonly" disabled={readOnly}><form className="lite-form doctor-details-form" onSubmit={submit}>
                        <h2>{form.id ? 'Edit' : 'Add'} {isDoctor ? 'doctor' : 'assistant'}</h2>

                        <label>
                            Name
                            <input
                                required
                                value={form.displayName || ''}
                                onChange={(event) =>
                                    setForm({ ...form, displayName: event.target.value })
                                }
                            />
                        </label>

                        {!isDoctor && (
                            <label>
                                Default office
                                <select
                                    value={form.defaultOffice?.id || ''}
                                    onChange={(event) => setForm({
                                        ...form,
                                        defaultOffice: offices.find(
                                            (office) => String(office.id) === event.target.value
                                        ) || null
                                    })}
                                >
                                    <option value="">All locations</option>
                                    {offices.map((office) => (
                                        <option key={office.id} value={office.id}>
                                            {office.name}
                                        </option>
                                    ))}
                                </select>
                            </label>
                        )}

                        {isDoctor && (
                            <fieldset className="doctor-week-pattern doctor-schedule-builder">
                                <legend>Work schedule</legend>
                                <p>
                                    Choose the office for each workday. Select alternating
                                    offices when the doctor switches locations every other week.
                                </p>

                                {loadingPattern ? (
                                    <p className="doctor-pattern-loading">Loading work schedule…</p>
                                ) : advancedPattern ? (
                                    <div className="doctor-alternating-example">
                                        <strong>This doctor uses a pattern longer than two weeks.</strong>
                                        <span>
                                            Use Advanced patterns to change the schedule. Saving here
                                            will only update the doctor’s name, notes, and status.
                                        </span>
                                    </div>
                                ) : weeklyPattern.map((entry) => (
                                    <div
                                        className={`doctor-day-card ${entry.works ? 'is-working' : ''}`}
                                        key={entry.day}
                                    >
                                        <label className="doctor-day-toggle">
                                            <input
                                                type="checkbox"
                                                checked={entry.works}
                                                onChange={(event) => updatePattern(entry.day, {
                                                    works: event.target.checked,
                                                    officeId: event.target.checked
                                                        ? entry.officeId
                                                        : '',
                                                    alternateOfficeId: event.target.checked
                                                        ? entry.alternateOfficeId
                                                        : ''
                                                })}
                                            />
                                            <span>
                                                <strong>{friendlyDay(entry.day)}</strong>
                                                <small>
                                                    {entry.works ? 'Working' : 'Does not work'}
                                                </small>
                                            </span>
                                        </label>

                                        {entry.works && (
                                            <div className="doctor-day-details">
                                                <label>
                                                    Office pattern
                                                    <select
                                                        value={entry.scheduleType}
                                                        onChange={(event) => updatePattern(
                                                            entry.day,
                                                            {
                                                                scheduleType: event.target.value,
                                                                alternateOfficeId:
                                                                    event.target.value === 'ALTERNATING'
                                                                        ? entry.alternateOfficeId
                                                                        : ''
                                                            }
                                                        )}
                                                    >
                                                        <option value="SAME_OFFICE">
                                                            Same office every week
                                                        </option>
                                                        <option value="ALTERNATING">
                                                            Alternate offices every other week
                                                        </option>
                                                    </select>
                                                </label>

                                                {entry.scheduleType === 'ALTERNATING' ? (
                                                    <div className="doctor-alternating-offices">
                                                        <label>
                                                            Week A
                                                            <select
                                                                required
                                                                value={entry.officeId}
                                                                onChange={(event) => updatePattern(
                                                                    entry.day,
                                                                    { officeId: event.target.value }
                                                                )}
                                                            >
                                                                <option value="">Select office</option>
                                                                {offices.map((office) => (
                                                                    <option
                                                                        key={office.id}
                                                                        value={office.id}
                                                                    >
                                                                        {office.name}
                                                                    </option>
                                                                ))}
                                                            </select>
                                                        </label>
                                                        <span aria-hidden="true">↔</span>
                                                        <label>
                                                            Week B
                                                            <select
                                                                required
                                                                value={entry.alternateOfficeId}
                                                                onChange={(event) => updatePattern(
                                                                    entry.day,
                                                                    {
                                                                        alternateOfficeId:
                                                                            event.target.value
                                                                    }
                                                                )}
                                                            >
                                                                <option value="">Select office</option>
                                                                {offices.map((office) => (
                                                                    <option
                                                                        key={office.id}
                                                                        value={office.id}
                                                                    >
                                                                        {office.name}
                                                                    </option>
                                                                ))}
                                                            </select>
                                                        </label>
                                                    </div>
                                                ) : (
                                                    <label>
                                                        Office
                                                        <select
                                                            required
                                                            value={entry.officeId}
                                                            onChange={(event) => updatePattern(
                                                                entry.day,
                                                                { officeId: event.target.value }
                                                            )}
                                                        >
                                                            <option value="">Select office</option>
                                                            {offices.map((office) => (
                                                                <option
                                                                    key={office.id}
                                                                    value={office.id}
                                                                >
                                                                    {office.name}
                                                                </option>
                                                            ))}
                                                        </select>
                                                    </label>
                                                )}
                                            </div>
                                        )}
                                    </div>
                                ))}

                            </fieldset>
                        )}

                        <label>
                            Notes
                            <textarea
                                value={form.notes || ''}
                                onChange={(event) =>
                                    setForm({ ...form, notes: event.target.value })
                                }
                            />
                        </label>

                        <label className="check-row">
                            <input
                                type="checkbox"
                                checked={form.active}
                                onChange={(event) =>
                                    setForm({ ...form, active: event.target.checked })
                                }
                            />
                            Active
                        </label>

                        <button
                            className="lite-button"
                            disabled={saving || loadingPattern}
                        >
                            {saving
                                ? 'Saving…'
                                : `Save ${isDoctor
                                    ? advancedPattern
                                        ? 'doctor information'
                                        : 'doctor and schedule'
                                    : 'assistant'}`}
                        </button>

                        {isDoctor && form.id && (
                            <button
                                className="danger-button"
                                type="button"
                                disabled={saving}
                                onClick={remove}
                            >
                                Remove doctor
                            </button>
                        )}
                    </form></fieldset>
                </div>
            </main>
        </div>
    )
}
