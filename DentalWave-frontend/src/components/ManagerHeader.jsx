import { isDesktop } from '../services/DesktopService'
import { Link, useNavigate } from 'react-router'

function ManagerHeader() {
    const navigate = useNavigate()
    async function handleLogout() {
        if (import.meta.env.MODE !== 'tauri') {
            const { logout } = await import('../services/AuthService')
            logout()
            navigate('/login')
        }
    }

    return (
        <header className="calendar-header">
            <Link to="/manager/dashboard" className="business-name">
                Wake Orthodontics and Pediatric Dentistry
            </Link>

            <nav>
                {!isDesktop && <Link to="/manager/dashboard">Dashboard</Link>}
                <Link to="/manager/schedule">Monthly Schedule</Link>
                {isDesktop && <Link to="/manager/offices">Offices</Link>}
                {isDesktop && <Link to="/manager/teams">Teams</Link>}
                {isDesktop && <Link to="/manager/doctor-patterns">Doctor Patterns</Link>}
                <Link to="/manager/doctors">Doctors</Link>
                <Link to="/manager/assistants">Assistants</Link>
                {isDesktop && <Link to="/manager/data">Data & backups</Link>}
            </nav>

            <div className="header-actions">

                {isDesktop ? <span>Desktop scheduler</span> : <button className="logout-button" onClick={handleLogout}>Logout</button>}
            </div>
        </header>
    )
}

export default ManagerHeader
