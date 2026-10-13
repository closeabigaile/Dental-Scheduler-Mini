// Build-host tool only. The USB application never launches a shell or Node.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

if (process.platform !== 'win32') throw new Error('Build the Windows USB ZIP on Windows or with the Windows USB workflow in GitHub Actions.')
const root = fileURLToPath(new URL('..', import.meta.url))
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Expected a numeric release version')
function run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: false, ...options })
    if (result.error) throw result.error
    if (result.status !== 0) throw new Error(`${command} failed (${result.status})`)
}
run(process.execPath, [path.join(root, 'node_modules/@tauri-apps/cli/tauri.js'), 'build', '--target', 'x86_64-pc-windows-msvc', '--no-bundle', '--', '--locked'], {
    // Avoid requiring the Visual C++ Redistributable on the destination computer.
    env: { ...process.env, RUSTFLAGS: `${process.env.RUSTFLAGS || ''} -C target-feature=+crt-static` },
})
run(process.execPath, [path.join(root, 'scripts/audit-desktop.mjs')])
const binaryDirectory = path.join(root, 'src-tauri/target/x86_64-pc-windows-msvc/release')
const binary = path.join(binaryDirectory, 'DentalSchedulerMini.exe')
const bytes = fs.readFileSync(binary)
const pe = bytes.length >= 64 ? bytes.readUInt32LE(60) : -1
if (bytes.toString('ascii', 0, 2) !== 'MZ' || pe < 0 || pe + 6 > bytes.length || bytes.readUInt32LE(pe) !== 0x4550 || bytes.readUInt16LE(pe + 4) !== 0x8664) {
    throw new Error('The release file is not a Windows x64 PE executable')
}
const stage = fs.mkdtempSync(path.join(os.tmpdir(), 'dental-scheduler-usb-'))
const folder = path.join(stage, 'DentalSchedulerMini')
fs.mkdirSync(folder)
try {
    fs.copyFileSync(binary, path.join(folder, 'DentalSchedulerMini.exe'))
    const loader = path.join(binaryDirectory, 'WebView2Loader.dll')
    if (fs.existsSync(loader)) fs.copyFileSync(loader, path.join(folder, 'WebView2Loader.dll'))
    fs.writeFileSync(path.join(folder, 'dental-scheduler-portable.json'), JSON.stringify({ formatVersion: 1, dataFolder: 'DentalSchedulerMiniData' }, null, 2) + '\n')
    fs.copyFileSync(path.join(root, '../portable/desktop-windows/START HERE.txt'), path.join(folder, 'START HERE.txt'))
    const files = fs.readdirSync(folder).sort().map(name => ({ name, sha256: createHash('sha256').update(fs.readFileSync(path.join(folder, name))).digest('hex') }))
    fs.writeFileSync(path.join(folder, 'PACKAGE.json'), JSON.stringify({ version, platform: 'windows-x64', distribution: 'usb', signing: 'unsigned development candidate', webview2: 'installed Evergreen runtime required', sourceCommit: process.env.GITHUB_SHA || null, files }, null, 2) + '\n')
    const output = path.join(root, '../release/windows-usb')
    fs.mkdirSync(output, { recursive: true })
    const stamp = new Date().toISOString().replace(/[:.]/g, '-')
    const zip = path.join(output, `DentalSchedulerMini-${version}-windows-x64-usb-${stamp}.zip`)
    run('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '$ErrorActionPreference = "Stop"; Compress-Archive -LiteralPath $env.DENTAL_USB_STAGE -DestinationPath $env.DENTAL_USB_ZIP -CompressionLevel Optimal'], {
        env: { ...process.env, DENTAL_USB_STAGE: folder, DENTAL_USB_ZIP: zip },
    })
    const hash = createHash('sha256').update(fs.readFileSync(zip)).digest('hex')
    fs.writeFileSync(zip + '.sha256', `${hash}  ${path.basename(zip)}\n`, { flag: 'wx' })
    console.log(`USB ZIP: ${zip}\nSHA-256: ${hash}\nUnsigned development candidate. Windows hardware acceptance remains required.`)
} finally {
    fs.rmSync(stage, { recursive: true, force: true })
}
