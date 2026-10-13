import process from 'node:process'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'node:path'
import fs from 'node:fs'
import { fileURLToPath } from 'node:url'

const host = process.env.TAURI_DEV_HOST

const root = path.dirname(fileURLToPath(import.meta.url))
const transports = new Set(['OfficeService', 'ManagerSchedulerService', 'DoctorWorkRuleService'])

function desktopBoundary() {
  return {
    name: 'desktop-runtime-boundary',
    enforce: 'pre',
    resolveId(source, importer) {
      if (!importer || !source.startsWith('.')) return
      const resolved = path.resolve(path.dirname(importer), source).replace(/\.js$/, '')
      if (path.dirname(resolved) === path.join(root, 'src/services') && transports.has(path.basename(resolved))) {
        return path.join(root, 'src/desktop/services', path.basename(resolved) + '.js')
      }
    },
    transformIndexHtml: {
      order: 'pre',
      handler: html => html.replace('/src/main.jsx', '/src/desktop/main.jsx').replace('Wake Orthodontics &amp; Pediatric Dentistry | Scheduling', 'Dental Scheduler Mini'),
    },
    generateBundle(_options, bundle) {
      const modules = [...new Set(Object.values(bundle).flatMap(item => Object.keys(item.modules || {})))].map(id => path.relative(root, id).replaceAll('\\', '/')).sort()
      const forbidden = modules.filter(id => /node_modules\/axios\/|src\/services\/(AuthService|AxiosConfig|apiConfig|CalendarService|ScheduleService|EmployeeService|TimeOffRequestService)\.|src\/pages\/(LoginPage|ManagerCalendar|ManagerEditCalendar|ManagerNewCalendar|ManagerDashboard)/.test(id))
      if (forbidden.length) this.error('Legacy code reached the desktop package: ' + forbidden.join(', '))
      // Build evidence stays outside embedded frontend assets.
      fs.mkdirSync(path.join(root, 'build-audit'), { recursive: true })
      fs.writeFileSync(path.join(root, 'build-audit/desktop-modules.json'), JSON.stringify(modules, null, 2) + '\n')
    },
  }
}

export default defineConfig(({ mode }) => ({
  plugins: [react(), ...(mode === 'tauri' ? [desktopBoundary()] : [])],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG),
  },
}))
