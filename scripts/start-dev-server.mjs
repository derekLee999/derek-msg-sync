import { spawn } from 'node:child_process'

const DEV_URL = 'http://localhost:1420'

async function devServerIsRunning() {
  try {
    const response = await fetch(DEV_URL, { method: 'HEAD' })
    return response.ok
  } catch {
    return false
  }
}

if (await devServerIsRunning()) {
  console.log(`Vite dev server is already running at ${DEV_URL}`)
  process.exit(0)
}

const vite = spawn(process.execPath, ['node_modules/vite/bin/vite.js'], {
  shell: true,
  stdio: 'inherit',
})

vite.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal)
    return
  }

  process.exit(code ?? 0)
})
