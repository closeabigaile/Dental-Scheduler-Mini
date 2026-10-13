import './App.css'
import StorageProvider from './storage/StorageProvider'
import { isDesktop } from './services/DesktopService'
import axios from 'axios'
import { useEffect, useState } from 'react'
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router'
import DesktopCalendarPage from './pages/DesktopCalendarPage'
import DesktopOfficesPage from './pages/DesktopOfficesPage'
import LoginPage from './pages/LoginPage'
import ManagerCalendarPage from './pages/ManagerCalendarPage'
import ManagerCalendarOverviewPage from './pages/ManagerCalendarOverviewPage'
import ManagerEditCalendarPage from './pages/ManagerEditCalendarPage'
import ManagerNewCalendarPage from './pages/ManagerNewCalendarPage'
import ManagerDashboardPage from './pages/ManagerDashboardPage'
import ManagerResourcesPage from './pages/ManagerResourcesPage'
import ManagerTeamsPage from './pages/ManagerTeamsPage'
import DoctorSchedulePatternsPage from './pages/DoctorSchedulePatternsPage'
import { getAuthHeader, getLoggedInUserRole, getToken, logout, saveLoggedInUser, saveLoggedInUserId, saveLoggedInUserName } from './services/AuthService'
import { apiUrl } from './services/apiConfig'

function ProtectedRoute({ children }) {
  const location = useLocation()
  const token = getToken()
  const [role, setRole] = useState(getLoggedInUserRole())
  const [loading, setLoading] = useState(Boolean(token) && !role)
  useEffect(() => {
    if (!token || role) return
    axios.get(apiUrl('/api/users/me'), getAuthHeader()).then(response => {
      const user=response.data||{}; const hydratedRole=Array.isArray(user.roles)?user.roles[0]:null
      if(hydratedRole){saveLoggedInUser(user.username,hydratedRole);saveLoggedInUserId(user.id);saveLoggedInUserName(user.firstName,user.lastName);setRole(hydratedRole)}
    }).catch(()=>{logout();setRole(null)}).finally(()=>setLoading(false))
  }, [token, role])
  if(!token) return <Navigate to="/login" replace state={{from:location}}/>
  if(loading) return <main className="lite-shell">Loading secure scheduling workspace…</main>
  if(role!=='ROLE_MANAGER'&&role!=='ROLE_ADMIN') return <Navigate to="/login" replace state={{unsupportedRole:true}}/>
  return children
}

const desktopRoutes = new Set(['/manager/data', '/manager/schedule', '/manager/schedule/preview', '/manager/calendar/new', '/manager/calendar', '/manager/print', '/manager/dashboard', '/manager/offices', '/manager/doctors', '/manager/assistants', '/manager/teams', '/manager/doctor-patterns'])
const Secure=({children})=>{
  const location = useLocation()
  if (isDesktop) return desktopRoutes.has(location.pathname) ? children : <Navigate to="/manager/doctors" replace/>
  return <ProtectedRoute>{children}</ProtectedRoute>
}
function WorkspaceRoutes(){
  const location = useLocation()
  const [last, setLast] = useState(location.pathname === '/manager/data' ? { ...location, pathname: '/manager/schedule' } : location)
  if (location.pathname !== '/manager/data' && last !== location) setLast(location)
  return <Routes location={isDesktop ? last : location}>
  <Route path="/" element={<Navigate to="/manager/dashboard" replace/>}/><Route path="/login" element={isDesktop ? <Navigate to="/manager/doctors" replace/> : <LoginPage/>}/>
  <Route path="/manager/dashboard" element={<Secure><>{isDesktop ? <Navigate to="/manager/schedule" replace/> : <ManagerDashboardPage/>}</></Secure>}/>
  <Route path="/manager/schedule" element={<Secure><>{isDesktop ? <DesktopCalendarPage/> : <ManagerCalendarPage/>}</></Secure>}/>
  <Route path="/manager/schedule/preview" element={<Secure><>{isDesktop ? <DesktopCalendarPage previewMode/> : <ManagerCalendarPage previewMode/>}</></Secure>}/>
  <Route path="/manager/calendar" element={<Secure><>{isDesktop ? <DesktopCalendarPage/> : <ManagerCalendarOverviewPage/>}</></Secure>}/>
  <Route path="/manager/calendar/build" element={<Navigate to="/manager/schedule" replace/>}/>
  <Route path="/manager/calendar/new" element={<Secure><>{isDesktop ? <DesktopCalendarPage/> : <ManagerNewCalendarPage/>}</></Secure>}/>
  <Route path="/manager/calendar/:id/edit" element={<Secure><ManagerEditCalendarPage/></Secure>}/>
  {isDesktop && <Route path="/manager/offices" element={<Secure><DesktopOfficesPage/></Secure>}/>}
  <Route path="/manager/teams" element={<Secure><ManagerTeamsPage/></Secure>}/>
  <Route path="/manager/doctors" element={<Secure><ManagerResourcesPage type="DOCTOR"/></Secure>}/>
  <Route path="/manager/doctor-patterns" element={<Secure><DoctorSchedulePatternsPage/></Secure>}/>
  <Route path="/manager/assistants" element={<Secure><ManagerResourcesPage type="ASSISTANT"/></Secure>}/>
  <Route path="/manager/print" element={<Secure><>{isDesktop ? <DesktopCalendarPage previewMode/> : <ManagerCalendarOverviewPage printMode/>}</></Secure>}/>
  <Route path="*" element={<Navigate to="/manager/dashboard" replace/>}/>
</Routes>}
export default function App(){return <BrowserRouter>{isDesktop ? <StorageProvider><WorkspaceRoutes/></StorageProvider> : <WorkspaceRoutes/>}</BrowserRouter>}
