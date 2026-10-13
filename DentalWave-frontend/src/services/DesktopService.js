import { invoke } from '@tauri-apps/api/core'
export const isDesktop = import.meta.env.MODE === 'tauri'

// Keep the existing page-facing response shape; web calls retain their REST path.
export async function desktopCall(command, args = {}) {
    try {
        return { data: await invoke(command, args) }
    } catch (failure) {
        if (typeof window !== 'undefined') window.dispatchEvent(new Event('storage-operation-failed'))
        const error = new Error(failure?.message || 'The desktop operation failed.')
        error.code = failure?.code || 'DESKTOP_ERROR'
        error.conflicts = failure?.conflicts || []
        error.response = { data: { code: error.code, message: error.message, conflicts: error.conflicts } }
        throw error
    }
}
