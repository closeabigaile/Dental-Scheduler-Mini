import test from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import vm from 'node:vm'

async function services(mode, failure) {
    const calls = [], context = vm.createContext({}), modules = new Map()
    const synthetic = (exports) => new vm.SyntheticModule(Object.keys(exports), function () {
        for (const [name, value] of Object.entries(exports)) this.setExport(name, value)
    }, { context })
    async function load(name) {
        if (name.endsWith('/DesktopService')) name = './DesktopService'
        if (modules.has(name)) return modules.get(name)
        let module
        if (name === 'axios') module = synthetic({ default: Object.fromEntries(['get', 'post', 'put', 'delete'].map(method => [method, (...args) => { calls.push({ method, args }); return Promise.resolve({ data: [] }) }])) })
        else if (name === '@tauri-apps/api/core') module = synthetic({ invoke: async (command, args) => { calls.push({ command, args }); if (failure) throw failure; return [{ id: 7 }] } })
        else if (name === './AuthService') module = synthetic({ getAuthHeader: () => ({ headers: {} }) })
        else if (name === './apiConfig') module = synthetic({ apiUrl: value => value })
        else {
            const service = name.split('/').at(-1)
            const directory = mode === 'tauri' && ['ManagerSchedulerService', 'DoctorWorkRuleService', 'OfficeService'].includes(service) ? 'desktop/services' : 'services'
            module = new vm.SourceTextModule(await readFile(new URL(`../src/${directory}/${service}.js`, import.meta.url), 'utf8'), { context, initializeImportMeta: meta => { meta.env = { MODE: mode } } })
        }
        modules.set(name, module)
        return module
    }
    const api = {}
    for (const name of ['ManagerSchedulerService', 'DoctorWorkRuleService', 'OfficeService', 'SchedulingDomainService', 'DesktopCalendarService']) {
        const module = await load(`./${name}`)
        await module.link(load); await module.evaluate(); api[name] = module.namespace
    }
    return { api, calls }
}

test('desktop services invoke typed commands and preserve page-facing responses', async () => {
    const { api, calls } = await services('tauri')
    const team = { name: 'Test', doctor: { id: 1 }, assistants: [{ id: 2 }] }
    const cases = [
        ['ManagerSchedulerService', 'getResources', ['DOCTOR'], 'get_doctors', {}],
        ['ManagerSchedulerService', 'getResources', ['ASSISTANT'], 'get_assistants', {}],
        ['ManagerSchedulerService', 'createResource', [{ type: 'DOCTOR' }], 'create_doctor', { input: { type: 'DOCTOR' } }],
        ['ManagerSchedulerService', 'updateResource', ['1', { type: 'ASSISTANT' }], 'update_assistant', { id: 1, input: { type: 'ASSISTANT' } }],
        ['ManagerSchedulerService', 'deleteResource', ['1', 'DOCTOR'], 'delete_doctor', { id: 1 }],
        ['ManagerSchedulerService', 'deleteResource', ['1', 'ASSISTANT'], 'delete_assistant', { id: 1 }],
        ['ManagerSchedulerService', 'getReusableTeams', [], 'get_teams', {}],
        ['ManagerSchedulerService', 'createReusableTeam', [team], 'create_team', { input: team }],
        ['ManagerSchedulerService', 'updateReusableTeam', ['7', team], 'update_team', { id: 7, input: team }],
        ['ManagerSchedulerService', 'duplicateReusableTeam', ['7'], 'duplicate_team', { id: 7 }],
        ['ManagerSchedulerService', 'deleteReusableTeam', ['7'], 'delete_team', { id: 7 }],
        ['OfficeService', 'getAllOffices', [], 'get_offices', {}],
        ['OfficeService', 'createOffice', [{ name: 'Test' }], 'create_office', { input: { name: 'Test' } }],
        ['DoctorWorkRuleService', 'getDoctorRules', ['1'], 'get_doctor_work_rules', { doctorId: 1 }],
        ['DoctorWorkRuleService', 'replaceDoctorRules', ['1', []], 'replace_doctor_work_rules', { doctorId: 1, rules: [] }],
        ['DoctorWorkRuleService', 'previewDoctorRules', ['1', '2024-01-01'], 'preview_doctor_rules', { doctorId: 1, startDate: '2024-01-01', days: 28 }],
        ['DoctorWorkRuleService', 'createRotationGroup', [{ numberOfWeeks: 2 }], 'create_doctor_rotation_group', { input: { numberOfWeeks: 2 } }],
        ['SchedulingDomainService', 'validateAssignment', [{ doctorId: 1 }], 'validate_assignment', { proposal: { doctorId: 1 }, existing: [] }],
        ['SchedulingDomainService', 'resolveDoctorLocation', ['1', '2024-01-01'], 'resolve_doctor_location', { doctorId: 1, date: '2024-01-01' }],
    ]
    for (const [service, method, args, command, payload] of cases) {
        const response = await api[service][method](...args)
        assert.equal(response.data[0].id, 7)
        assert.deepEqual(JSON.parse(JSON.stringify(calls.at(-1))), { command, args: payload })
    }
    assert.equal(calls.length, cases.length)
})
test('web team and pattern services retain REST behavior', async () => {
    const { api, calls } = await services('production')
    await api.ManagerSchedulerService.getReusableTeams()
    await api.ManagerSchedulerService.createReusableTeam({ name: 'Test' })
    await api.DoctorWorkRuleService.previewDoctorRules(1, '2024-01-01')
    await api.OfficeService.getAllOffices()
    assert.deepEqual(calls.map(c => c.method), ['get', 'post', 'get', 'get'])
    assert.equal(calls[0].args[0], '/api/manager/teams')
    assert.equal(calls[2].args[0], '/api/doctor-work-rules/doctor/1/preview')
})
test('structured desktop failures reach existing page error handlers', async () => {
    const { api } = await services('tauri', { code: 'INVALID_TEAM', message: 'Doctor is inactive.' })
    await assert.rejects(api.ManagerSchedulerService.createReusableTeam({}), error => error.code === 'INVALID_TEAM' && error.response.data.message === 'Doctor is inactive.')
})


test('desktop calendar commands include the complete month and revision snapshot', async () => {
    const { api, calls } = await services('tauri')
    const calendars = [{ id: 3, revision: 2, year: 2026, month: 9, days: [] }]
    const service = api.DesktopCalendarService
    await service.loadCalendarMonth(2026, 9)
    await service.createCalendarMonth(2026, 9, false)
    await service.saveCalendarMonth(2026, 9, calendars)
    await service.deleteCalendarMonth(2026, 9, calendars)
    await service.printCalendar()
    assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
        { command: 'list_calendars', args: { year: 2026, month: 9 } },
        { command: 'create_calendar_month', args: { year: 2026, month: 9, generate: false } },
        { command: 'save_calendar_month', args: { year: 2026, month: 9, calendars } },
        { command: 'delete_calendar_month', args: { year: 2026, month: 9, calendars } },
        { command: 'print_calendar', args: {} }
    ])
})
test('calendar conflicts retain structured resource codes for the day editor', async () => {
    const conflicts = [{ code: 'DOCTOR_WRONG_OFFICE', resourceId: 4 }]
    const { api } = await services('tauri', { code: 'ASSIGNMENT_CONFLICT', message: 'Wrong office.', conflicts })
    await assert.rejects(api.DesktopCalendarService.saveCalendarMonth(2026, 9, []), error =>
        error.code === 'ASSIGNMENT_CONFLICT' && error.conflicts === conflicts)
})
