import { isDesktop, desktopCall } from './DesktopService'
import axios from 'axios'
import { getAuthHeader } from './AuthService'
import { apiUrl } from './apiConfig'

const OFFICE_REST_API_BASE_URL = apiUrl('/api/offices')

export const getAllOffices = () =>
    isDesktop ? desktopCall('get_offices', {}) : axios.get(OFFICE_REST_API_BASE_URL, getAuthHeader())

export const getOfficeById = (officeId) =>
    isDesktop ? desktopCall('get_office', { id: Number(officeId) }) : axios.get(`${OFFICE_REST_API_BASE_URL}/${officeId}`, getAuthHeader())

export const createOffice = (office) =>
    isDesktop ? desktopCall('create_office', { input: office }) : axios.post(OFFICE_REST_API_BASE_URL, office, getAuthHeader())

export const updateOffice = (officeId, office) =>
    isDesktop ? desktopCall('update_office', { id: Number(officeId), input: office }) : axios.put(`${OFFICE_REST_API_BASE_URL}/${officeId}`, office, getAuthHeader())

export const deleteOffice = (officeId) =>
    isDesktop ? desktopCall('delete_office', { id: Number(officeId) }) : axios.delete(`${OFFICE_REST_API_BASE_URL}/${officeId}`, getAuthHeader())
