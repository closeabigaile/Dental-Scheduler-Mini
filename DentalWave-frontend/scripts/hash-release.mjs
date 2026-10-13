// Build/review workstation utility; does not run inside the installed application.
import fs from 'node:fs'
import path from 'node:path'
import { createHash } from 'node:crypto'
const files = process.argv.slice(2)
if (!files.length) throw new Error('Supply the final EXE and NSIS setup EXE paths, after signing if applicable.')
for (const file of files) {
    if (path.extname(file).toLowerCase() !== '.exe') throw new Error('Expected a Windows executable artifact')
    const hash = createHash('sha256').update(fs.readFileSync(file)).digest('hex')
    console.log(`${hash}  ${path.basename(file)}`)
}
