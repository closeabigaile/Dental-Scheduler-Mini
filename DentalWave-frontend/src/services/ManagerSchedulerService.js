import { isDesktop, desktopCall } from './DesktopService'
import axios from 'axios'
import { getAuthHeader } from './AuthService'
import { apiUrl } from './apiConfig'
const API = apiUrl('/api/manager')
export const getResources = (type) => isDesktop ? desktopCall(type === 'DOCTOR' ? 'get_doctors' : 'get_assistants') : axios.get(`${API}/resources`, { ...getAuthHeader(), params: { type } })
export const createResource = (value) => isDesktop ? desktopCall(value.type === 'DOCTOR' ? 'create_doctor' : 'create_assistant', { input: value }) : axios.post(`${API}/resources`, value, getAuthHeader())
export const updateResource = (id, value) => isDesktop ? desktopCall(value.type === 'DOCTOR' ? 'update_doctor' : 'update_assistant', { id: Number(id), input: value }) : axios.put(`${API}/resources/${id}`, value, getAuthHeader())
export const deleteResource = (id, type) => isDesktop ? desktopCall(type === 'DOCTOR' ? 'delete_doctor' : 'delete_assistant', { id: Number(id) }) : axios.delete(`${API}/resources/${id}`, getAuthHeader())
export const getReusableTeams = () => isDesktop ? desktopCall('get_teams', {}) : axios.get(`${API}/teams`, getAuthHeader())
export const createReusableTeam = (value) => isDesktop ? desktopCall('create_team', { input: value }) : axios.post(`${API}/teams`, value, getAuthHeader())
export const updateReusableTeam = (id, value) => isDesktop ? desktopCall('update_team', { id: Number(id), input: value }) : axios.put(`${API}/teams/${id}`, value, getAuthHeader())
export const duplicateReusableTeam = (id) => isDesktop ? desktopCall('duplicate_team', { id: Number(id) }) : axios.post(`${API}/teams/${id}/duplicate`, {}, getAuthHeader())
export const deleteReusableTeam = (id) => isDesktop ? desktopCall('delete_team', { id: Number(id) }) : axios.delete(`${API}/teams/${id}`, getAuthHeader())
