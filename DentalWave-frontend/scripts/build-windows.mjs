// Build-time wrapper only. Never shipped or invoked by the desktop application.
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
if (process.platform !== 'win32') {
    throw new Error('Build the Windows candidate on an approved Windows x64 build host. No cross-compilation or macOS bundle is configured by this command.')
}
const cli = fileURLToPath(new URL('../node_modules/@tauri-apps/cli/tauri.js', import.meta.url))
const result = spawnSync(process.execPath, [cli, 'build', '--target', 'x86_64-pc-windows-msvc', '--bundles', 'nsis', '--', '--locked'], { stdio: 'inherit', shell: false })
if (result.error) throw result.error
process.exitCode = result.status ?? 1
