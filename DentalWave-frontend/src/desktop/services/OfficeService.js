import { desktopCall } from '../../services/DesktopService'

export const getAllOffices = () => desktopCall('get_offices', {})
export const getOfficeById = (officeId) => desktopCall('get_office', { id: Number(officeId) })
export const createOffice = (office) => desktopCall('create_office', { input: office })
export const updateOffice = (officeId, office) => desktopCall('update_office', { id: Number(officeId), input: office })
export const deleteOffice = (officeId) => desktopCall('delete_office', { id: Number(officeId) })
