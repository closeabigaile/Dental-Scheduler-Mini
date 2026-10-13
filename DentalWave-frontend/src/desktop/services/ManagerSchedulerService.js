import { desktopCall } from '../../services/DesktopService'

export const getResources = (type) => desktopCall(type === 'DOCTOR' ? 'get_doctors' : 'get_assistants')
export const createResource = (value) => desktopCall(value.type === 'DOCTOR' ? 'create_doctor' : 'create_assistant', { input: value })
export const updateResource = (id, value) => desktopCall(value.type === 'DOCTOR' ? 'update_doctor' : 'update_assistant', { id: Number(id), input: value })
export const deleteResource = (id, type) => desktopCall(type === 'DOCTOR' ? 'delete_doctor' : 'delete_assistant', { id: Number(id) })
export const getReusableTeams = () => desktopCall('get_teams', {})
export const createReusableTeam = (value) => desktopCall('create_team', { input: value })
export const updateReusableTeam = (id, value) => desktopCall('update_team', { id: Number(id), input: value })
export const duplicateReusableTeam = (id) => desktopCall('duplicate_team', { id: Number(id) })
export const deleteReusableTeam = (id) => desktopCall('delete_team', { id: Number(id) })
