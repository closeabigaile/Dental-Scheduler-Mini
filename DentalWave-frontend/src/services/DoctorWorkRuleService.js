import { isDesktop, desktopCall } from './DesktopService'
import axios from 'axios'
import { getAuthHeader } from './AuthService'
import { apiUrl } from './apiConfig'
const RULES=apiUrl('/api/doctor-work-rules'), ROTATIONS=apiUrl('/api/schedule-rotation-groups')
export const getDoctorRules=id=>isDesktop ? desktopCall('get_doctor_work_rules', { doctorId: Number(id) }) : axios.get(`${RULES}/doctor/${id}`,getAuthHeader())
const patternConfig=()=>({...getAuthHeader(),skipAuthRedirect:true})
export const createDoctorRule=value=>isDesktop ? desktopCall('create_doctor_work_rule', { input: value }) : axios.post(RULES,value,patternConfig())
export const replaceDoctorRules=(doctorId,value)=>isDesktop ? desktopCall('replace_doctor_work_rules', { doctorId: Number(doctorId), rules: value }) : axios.put(`${RULES}/doctor/${doctorId}`,value,patternConfig())
export const deleteDoctorRule=id=>isDesktop ? desktopCall('delete_doctor_work_rule', { id: Number(id) }) : axios.delete(`${RULES}/${id}`,patternConfig())
export const previewDoctorRules=(id,startDate)=>isDesktop ? desktopCall('preview_doctor_rules', { doctorId: Number(id), startDate, days: 28 }) : axios.get(`${RULES}/doctor/${id}/preview`,{...getAuthHeader(),params:{startDate,days:28}})
export const getRotationGroups=()=>isDesktop ? desktopCall('get_doctor_rotation_groups', {}) : axios.get(ROTATIONS,getAuthHeader())
export const createRotationGroup=value=>isDesktop ? desktopCall('create_doctor_rotation_group', { input: value }) : axios.post(ROTATIONS,value,patternConfig())
