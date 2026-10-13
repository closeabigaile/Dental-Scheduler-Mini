import { createContext, useContext } from 'react'
export const StorageContext = createContext(null)
export const useStorage = () => useContext(StorageContext)
export const clearUnsaved = () => window.dispatchEvent(new Event('storage-form-saved'))
// Session-owned memory only; cleared on successful close/open/restore or explicit discard.
export const calendarDraft = { current: null }
