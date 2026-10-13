import { desktopCall } from './DesktopService'
// Read-only Phase 3 domain entry points. No schedule instances are saved.
export const validateAssignment = (proposal, existing = []) =>
    desktopCall('validate_assignment', { proposal, existing })
export const resolveDoctorLocation = (doctorId, date) =>
    desktopCall('resolve_doctor_location', { doctorId: Number(doctorId), date })
