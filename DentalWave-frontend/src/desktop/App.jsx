import '../App.css'
import { useState } from 'react'
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router'
import StorageProvider from '../storage/StorageProvider'
import DesktopCalendarPage from '../pages/DesktopCalendarPage'
import DesktopOfficesPage from '../pages/DesktopOfficesPage'
import ManagerResourcesPage from '../pages/ManagerResourcesPage'
import ManagerTeamsPage from '../pages/ManagerTeamsPage'
import DoctorSchedulePatternsPage from '../pages/DoctorSchedulePatternsPage'

function Workspace() {
    const location = useLocation()
    const [last, setLast] = useState(location.pathname === '/manager/data' ? { ...location, pathname: '/manager/schedule' } : location)
    if (location.pathname !== '/manager/data' && last !== location) setLast(location)
    // Keep the editor mounted while Data is open, preserving unsaved drafts.
    return <Routes location={last}>
        <Route path="/manager/schedule" element={<DesktopCalendarPage />} />
        <Route path="/manager/schedule/preview" element={<DesktopCalendarPage previewMode />} />
        <Route path="/manager/print" element={<DesktopCalendarPage previewMode />} />
        <Route path="/manager/calendar" element={<DesktopCalendarPage />} />
        <Route path="/manager/calendar/new" element={<DesktopCalendarPage />} />
        <Route path="/manager/offices" element={<DesktopOfficesPage />} />
        <Route path="/manager/doctors" element={<ManagerResourcesPage type="DOCTOR" />} />
        <Route path="/manager/assistants" element={<ManagerResourcesPage type="ASSISTANT" />} />
        <Route path="/manager/teams" element={<ManagerTeamsPage />} />
        <Route path="/manager/doctor-patterns" element={<DoctorSchedulePatternsPage />} />
        <Route path="*" element={<Navigate to="/manager/schedule" replace />} />
    </Routes>
}
export default function DesktopApp() {
    return <BrowserRouter><StorageProvider><Workspace /></StorageProvider></BrowserRouter>
}
