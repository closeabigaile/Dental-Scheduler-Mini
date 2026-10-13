import { useCallback, useEffect, useRef, useState } from 'react'
import { StorageContext as Context, useStorage, calendarDraft } from './context'
import { useLocation, useNavigate } from 'react-router'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import './storage.css'
export default function StorageProvider({ children }) {
    const [status, setStatus] = useState(null)
    const [error, setError] = useState('')
    const [notice, setNotice] = useState('')
    const [busy, setBusy] = useState(false)
    const [confirmation, setConfirmation] = useState(null)
    const pendingConfirmation = useRef(false)
    const confirmAction = useCallback(message => {
        if (pendingConfirmation.current) return Promise.resolve(false)
        pendingConfirmation.current = true
        return new Promise(resolve => setConfirmation({ message, resolve: answer => { pendingConfirmation.current = false; resolve(answer) } }))
    }, [])
    const dirty = useRef(false)
    const statusRef = useRef(null)
    const refreshSequence = useRef(0)
    const closing = useRef(false)
    const location = useLocation()
    const navigate = useNavigate()
    const refresh = useCallback(async () => {
        const sequence = ++refreshSequence.current
        try { const next = await invoke('storage_status'); if (sequence === refreshSequence.current) { statusRef.current = next; setStatus(next) } return next }
        catch (e) { setError(e.message || 'Storage status could not be checked.'); return null }
    }, [])
    const setDirty = useCallback(value => { dirty.current = value }, [])
    const discardConfirmed = useCallback(async () => {
        if (dirty.current && !await confirmAction('You have unsaved changes. Discard them and continue? Choose Cancel to keep editing.')) return false
        return true
    }, [confirmAction])
    const run = useCallback(async function run(command, args = {}, { discard = false, message = '', quiet = false } = {}) {
        if (discard && !await discardConfirmed()) return null
        setBusy(true); if (!quiet) { setError(''); setNotice('') }
        try {
            const result = await invoke(command, args)
            if (['close_database', 'restore_database', 'open_database', 'initialize_portable_database'].includes(command)) {
                dirty.current = false; calendarDraft.current = null
            }
            await refresh(); if (message) setNotice(message); return result
        } catch (e) { setError(e.message || 'The storage operation failed.'); await refresh(); return null }
        finally { setBusy(false) }
    }, [discardConfirmed, refresh])
    useEffect(() => {
        Promise.resolve().then(refresh)
        const interval = setInterval(refresh, 3000)
        const saved = () => { dirty.current = false }
        const failed = () => { refresh() }
        const beforeUnload = e => { if (statusRef.current?.active) { e.preventDefault(); e.returnValue = '' } }
        window.addEventListener('storage-form-saved', saved)
        window.addEventListener('storage-operation-failed', failed)
        window.addEventListener('beforeunload', beforeUnload)
        let cleanup, disposed = false
        listen('storage-close-requested', async () => {
            if (closing.current) return
            closing.current = true
            try {
                if (!await discardConfirmed()) return
                let result = await run('close_database', { withoutBackup: false })
                if (!result && await confirmAction('The final backup could not be completed. Close the database without that backup? Unsaved changes will be lost.')) result = await run('close_database', { withoutBackup: true })
                if (result) await getCurrentWindow().destroy()
            } finally { closing.current = false }
        }).then(fn => { if (disposed) fn(); else cleanup = fn }).catch(() => setError('Close protection could not be connected. Use Close database in Data before exiting.'))
        return () => { disposed = true; cleanup?.(); clearInterval(interval); window.removeEventListener('storage-form-saved', saved); window.removeEventListener('storage-operation-failed', failed); window.removeEventListener('beforeunload', beforeUnload) }
    }, [refresh, run, discardConfirmed, confirmAction]) // Refs keep the current session and edits.
    function captureChange(e) {
        if (!location.pathname.endsWith('/data') && (e.target.closest('form') || e.target.closest('.rotation-builder'))) dirty.current = true
    }
    async function captureLink(e) {
        const link = e.target.closest('a[href]')
        if (!link || !dirty.current || link.getAttribute('href') === location.pathname) return
        e.preventDefault(); e.stopPropagation()
        // Data keeps the current page mounted below it, so recovery never loses a draft.
        if (link.getAttribute('href') === '/manager/data') { navigate('/manager/data'); return }
        if (await discardConfirmed()) { dirty.current = false; calendarDraft.current = null; navigate(link.getAttribute('href')) }
    }
    const value = { status, busy, error, notice, run, refresh, readOnly: status?.access !== 'READABLE_AND_WRITABLE', setDirty, discardConfirmed, confirmAction }
    if (!status) return <main className="storage-panel">Checking schedule storage…{error && <p role="alert">{error}</p>}</main>
    return <Context.Provider value={value}>
        <div inert={Boolean(confirmation)}>
        <div className="storage-banner no-print" role="status"><strong>{status.mode === 'portable' ? 'Portable data' : 'Local data'}</strong> · {status.access === 'READABLE_AND_WRITABLE' ? 'Ready' : status.access === 'READ_ONLY' ? 'Read-only — changes cannot be saved' : status.active ? 'Editing stopped' : 'Database closed'} <button onClick={() => navigate('/manager/data')}>Data & backups</button></div>
        {status.message && <p role="alert" className="storage-warning no-print">{status.message}</p>}
        {(!status.active || location.pathname === '/manager/data') && <DataPanel />}
        {status.active && <div key={status.sessionKey} hidden={location.pathname === '/manager/data'} onChangeCapture={captureChange} onClickCapture={captureLink}>
            {children}
        </div>}
        </div>
        {confirmation && <div className="storage-modal no-print" role="alertdialog" aria-modal="true" aria-label="Confirm data action"><div><p>{confirmation.message}</p><button autoFocus onClick={() => { confirmation.resolve(false); setConfirmation(null) }}>Cancel</button><button onClick={() => { confirmation.resolve(true); setConfirmation(null) }}>Continue</button></div></div>}
    </Context.Provider>
}
function DataPanel() {
    const { status, busy, error, notice, run, refresh, discardConfirmed, confirmAction } = useStorage()
    const [candidates, setCandidates] = useState([])
    const [directory, setDirectory] = useState(status.portablePackageDirectory || '')
    const [source, setSource] = useState('')
    const [backups, setBackups] = useState([])
    const [selected, setSelected] = useState('')
    const [report, setReport] = useState('')
    const navigate = useNavigate()
    const scan = useCallback(async () => { const found = await run('discover_databases', {}, { quiet: true }); if (found) setCandidates(found) }, [run])
    const reloadBackups = useCallback(async () => { const found = await run('list_database_backups', {}, { quiet: true }); if (found) setBackups(found) }, [run])
    useEffect(() => { Promise.resolve().then(status.active ? reloadBackups : scan) }, [status.sessionKey, status.active, reloadBackups, scan])
    async function open(mode, folder) {
        const result = await run('open_database', { mode, directory: folder || null })
        if (result?.access !== 'UNAVAILABLE' && result) navigate('/manager/schedule')
    }
    async function close(withoutBackup = false) {
        const result = await run('close_database', { withoutBackup }, { discard: true, message: 'The schedule database is closed. Close the application window, then eject the drive using Windows before unplugging it.' })
        if (result) await refresh()
    }
    return <main className="storage-panel no-print">
        <h1>Schedule data & backups</h1>
        {status.directory && <p className="storage-path">{status.directory}</p>}
        {error && <p className="form-error" role="alert">{error}</p>}{notice && <p role="status">{notice}</p>}
        <fieldset disabled={busy}>
            {status.active ? <>
                <div className="storage-actions"><button onClick={() => navigate('/manager/schedule')}>Return to schedule</button><button disabled={status.access === 'READ_ONLY'} onClick={async () => { if (await run('backup_database', {}, { message: 'Backup successful — the saved database was copied and verified.' })) await reloadBackups() }}>Back up now</button><button onClick={() => close()}>Close database</button></div>
                <p>Backups contain saved data. Keep independent copies on another device to protect against loss of this drive.</p>
                {error && <button onClick={async () => await confirmAction('Close without a final backup? Choose Cancel to retry the backup first.') && close(true)}>Close without final backup</button>}
                <h2>Restore a verified backup</h2><p>Restore replaces saved schedules. The current database is preserved first. Any unsaved changes must be discarded.</p>
                <div className="storage-actions"><select aria-label="Verified backup" value={selected} onChange={e => setSelected(e.target.value)}><option value="">Select backup</option>{backups.map(b => <option key={b.filename} value={b.filename}>{b.createdAt} · {b.reason}</option>)}</select><button onClick={reloadBackups}>Refresh backups</button><button disabled={!selected || status.access === 'READ_ONLY'} onClick={async () => {
                    if (!await confirmAction('Restore this backup and replace the saved schedule database?') || !await discardConfirmed()) return
                    if (await run('restore_database', { filename: selected }, { message: 'Restore successful. The restored database passed validation.' })) { await reloadBackups(); navigate('/manager/schedule') }
                }}>Restore selected backup</button></div>
            </> : <>
                <p>Open the existing data folder on your schedule drive. If the drive is missing, connect it and scan again.</p>
                {status.portablePackageDirectory && <p>This USB package opens the data folder beside the application. On first use, expand “Set up a new portable data folder” below; its location is already filled in. If you already have schedules, open or import that data instead of creating a blank database.</p>}
                <div className="storage-actions"><button onClick={scan}>Scan removable drives</button><button onClick={() => open('portable')}>{status.portablePackageDirectory ? 'Open this USB package’s data' : 'Open remembered portable database'}</button>{!status.portablePackageDirectory && <button onClick={() => open('local')}>Use local data</button>}</div>
                {candidates.map(c => <p key={c.directory}><button onClick={() => open('portable', c.directory)}>{c.directory}</button>{c.errorCode && ' · Needs attention'}</p>)}
                <label>Existing data folder<input value={directory} onChange={e => setDirectory(e.target.value)} placeholder="Full path to DentalSchedulerMiniData"/></label><button disabled={!directory} onClick={() => open('portable', directory)}>Open this data folder</button>
                <details><summary>Set up a new portable data folder</summary><p>Use a new folder name. Existing folders and source databases will not be overwritten.</p><label>New data folder<input value={directory} onChange={e => setDirectory(e.target.value)}/></label><label>Verified SQLite copy to import (optional)<input value={source} onChange={e => setSource(e.target.value)} placeholder="Leave empty only for an intentionally blank database"/></label><button disabled={!directory} onClick={async () => {
                    if (!await confirmAction(source ? 'Create a new portable database from this SQLite copy?' : 'Create an intentionally blank portable database in this new folder?')) return
                    const result = await run('initialize_portable_database', { directory, source: source || null })
                    if (result?.access === 'READABLE_AND_WRITABLE') navigate('/manager/schedule')
                }}>Create new portable data folder</button></details>
            </>}
            <h2>Diagnostics</h2><button onClick={async () => { const result = await run('diagnostic_report'); if (result) { setReport(result); try { await navigator.clipboard.writeText(result) } catch { /* Selectable report remains available. */ } } }}>Copy diagnostic report</button>
            {report && <textarea aria-label="Diagnostic report" readOnly rows={12} value={report}/>}
        </fieldset>
    </main>
}
