import { Component, StrictMode, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { invoke } from '@tauri-apps/api/core'
import '../index.css'
import DesktopApp from './App'

class ApplicationBoundary extends Component {
    state = { failed: false }
    static getDerivedStateFromError() { return { failed: true } }
    render() {
        if (this.state.failed) return <main className="storage-panel"><h1>Unexpected application error</h1><p>Saved schedules remain on disk. Keep the application open if you need to preserve unsaved information, then close and reopen it.</p><p>Error code: UNEXPECTED_APPLICATION_ERROR</p></main>
        return this.props.children
    }
}
// This entry point deliberately mounts its components rather than exporting them.
// eslint-disable-next-line react-refresh/only-export-components
function Startup() {
    const [info, setInfo] = useState(null)
    useEffect(() => { invoke('application_info').then(setInfo).catch(() => setInfo({ startupError: { code: 'UNEXPECTED_APPLICATION_ERROR', message: 'The application could not finish starting. Close and reopen it. If this continues, contact your support team.' } })) }, [])
    if (!info) return <main className="storage-panel">Starting Dental Scheduler Mini…</main>
    if (info.startupError) return <main className="storage-panel"><h1>Application startup needs attention</h1><p role="alert">{info.startupError.message}</p><p>Error code: {info.startupError.code}</p><p>Check application-data folder access with your support team. No alternate database was opened.</p><textarea aria-label="Startup diagnostic report" readOnly rows={10} value={JSON.stringify(info, null, 2)} /></main>
    return <DesktopApp />
}
createRoot(document.getElementById('root')).render(<StrictMode><ApplicationBoundary><Startup /></ApplicationBoundary></StrictMode>)
