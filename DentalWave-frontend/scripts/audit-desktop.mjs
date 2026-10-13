// Build-time package-input audit. Run after build:desktop; no runtime dependency.
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'
const root = fileURLToPath(new URL('..', import.meta.url))
const read = name => fs.readFileSync(path.join(root, name), 'utf8')
const config = JSON.parse(read('src-tauri/tauri.conf.json'))
const capability = JSON.parse(read('src-tauri/capabilities/default.json'))
const modules = JSON.parse(read('build-audit/desktop-modules.json'))
assert.equal(config.build.beforeBuildCommand, 'npm run build:desktop')
assert.equal(config.mainBinaryName, 'DentalSchedulerMini')
assert.match(read('src-tauri/Cargo.toml'), /custom-protocol = \["tauri\/custom-protocol"\]/)
assert.equal(config.bundle.windows.webviewInstallMode.type, 'skip')
assert.equal(config.bundle.windows.minimumWebview2Version, undefined, 'Minimum-version installer updates require a separate reviewed distribution strategy')
assert.equal(config.bundle.windows.nsis.installMode, 'currentUser')
assert.deepEqual(config.bundle.targets, ['nsis'])
assert.deepEqual(config.bundle.resources, [])
assert.deepEqual(config.bundle.externalBin, [])
assert.equal(config.app.security.csp.split(';').find(v => v.trim().startsWith('connect-src')).trim(), 'connect-src ipc: http://ipc.localhost')
assert.equal(config.app.security.dangerousDisableAssetCspModification, undefined)
assert.equal(capability.remote, undefined)
assert.deepEqual(capability.windows, ['main'])
assert.deepEqual(capability.permissions, ['core:event:allow-listen', 'core:event:allow-unlisten', 'core:window:allow-destroy'])
assert.equal(JSON.parse(read('package.json')).version, config.version)
assert.match(read('src-tauri/Cargo.toml'), new RegExp('version = "' + config.version.replaceAll('.', '\\.') + '"'))
assert(modules.includes('src/desktop/main.jsx'))
assert(!modules.some(id => /axios|services\/(AuthService|AxiosConfig|apiConfig)|src\/App\.jsx|LoginPage/.test(id)))
const files = []
function inspect(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
        const file = path.join(directory, entry.name)
        assert(!entry.isSymbolicLink(), 'No symlinks allowed in embedded assets')
        if (entry.isDirectory()) { inspect(file); continue }
        const name = path.relative(path.join(root, 'dist'), file).replaceAll('\\', '/')
        assert(/^(index\.html|favicon\.(svg|ico)|assets\/[\w-]+\.(js|css|png|svg|ico))$/.test(name), 'Unexpected embedded file: ' + name)
        const bytes = fs.readFileSync(file)
        if (/\.(js|css|html|svg)$/.test(name)) {
            const text = bytes.toString('utf8')
            assert(!/localhost:5173|localhost:8080|127\.0\.0\.1|\/api\/auth|VITE_API|Bearer |jwt\.secret|BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY|\/Users\/|C:\\\\Users\\\\/.test(text), 'Unexpected development/backend/secret reference in ' + name)
            assert(!/(?:src|href)=["']https?:|@import\s+["']https?:|url\(["']?https?:/.test(text), 'Remote asset in ' + name)
        }
        files.push({ name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') })
    }
}
inspect(path.join(root, 'dist'))
const report = { version: config.version, scope: 'Embedded frontend and checked packaging configuration; not a Windows artifact inspection', modules: modules.length, files, resources: config.bundle.resources, sidecars: config.bundle.externalBin, webviewInstallMode: 'skip', installer: 'NSIS/currentUser', networkRequiredByApplication: false }
fs.writeFileSync(path.join(root, 'build-audit/package-inputs.json'), JSON.stringify(report, null, 2) + '\n')
console.log(JSON.stringify(report, null, 2))
