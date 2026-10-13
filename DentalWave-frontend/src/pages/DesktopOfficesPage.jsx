import { useStorage, clearUnsaved } from '../storage/context'
import { useEffect, useState } from 'react'
import ManagerHeader from '../components/ManagerHeader'
import { getAllOffices, createOffice, updateOffice, deleteOffice } from '../services/OfficeService'
const blank = { name: '', address: '', phoneNumber: '' }
export default function DesktopOfficesPage() {const storage = useStorage(); const readOnly = storage?.readOnly || false;
    const [offices, setOffices] = useState([])
    const [form, setForm] = useState(blank)
    const [error, setError] = useState('')
    const load = () => getAllOffices().then(r => setOffices(r.data))
    const failed = e => setError(e.response?.data?.message || 'Unable to save office.')
    useEffect(() => { getAllOffices().then(r => setOffices(r.data)).catch(e => setError(e.message)) }, [])
    async function save(event) {
        event.preventDefault(); setError('')
        try {
            if (form.id) await updateOffice(form.id, form)
            else await createOffice(form)
            clearUnsaved();setForm(blank); await load()
        } catch (e) { failed(e) }
    }
    async function remove() {
        if (!confirm('Delete this office? Referenced offices cannot be deleted.')) return
        try { await deleteOffice(form.id); clearUnsaved();setForm(blank); await load() } catch (e) { failed(e) }
    }
    return <div className="calendar-page"><ManagerHeader/><main className="lite-shell">
        <div className="page-heading"><h1>Offices</h1><button className="lite-button" onClick={() => setForm(blank)}>New office</button></div>
        {error && <p className="form-error">{error}</p>}
        <div className="resource-layout"><section className="resource-list">
            {offices.map(o => <article className="team-card" key={o.id}><button onClick={() => setForm(o)}><strong>{o.name}</strong><small>{o.address}</small></button></article>)}
        </section><fieldset className="storage-readonly" disabled={readOnly}><form className="lite-form" onSubmit={save}><h2>{form.id ? 'Edit office' : 'New office'}</h2>
            <label>Name<input required value={form.name} onChange={e => setForm({...form, name: e.target.value})}/></label>
            <label>Address<input value={form.address} onChange={e => setForm({...form, address: e.target.value})}/></label>
            <label>Phone<input value={form.phoneNumber} onChange={e => setForm({...form, phoneNumber: e.target.value})}/></label>
            <button className="lite-button">Save office</button>
            {form.id && <button type="button" className="danger-button" onClick={remove}>Delete office</button>}
        </form></fieldset></div>
    </main></div>
}
