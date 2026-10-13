import { desktopCall } from '../../services/DesktopService'

export const getDoctorRules = id => desktopCall('get_doctor_work_rules', { doctorId: Number(id) })
export const createDoctorRule = value => desktopCall('create_doctor_work_rule', { input: value })
export const replaceDoctorRules = (doctorId,value) => desktopCall('replace_doctor_work_rules', { doctorId: Number(doctorId), rules: value })
export const deleteDoctorRule = id => desktopCall('delete_doctor_work_rule', { id: Number(id) })
export const previewDoctorRules = (id,startDate) => desktopCall('preview_doctor_rules', { doctorId: Number(id), startDate, days: 28 })
export const getRotationGroups = () => desktopCall('get_doctor_rotation_groups', {})
export const createRotationGroup = value => desktopCall('create_doctor_rotation_group', { input: value })
