#!/usr/bin/env node
'use strict'

const { spawnSync } = require('node:child_process')

const candidates = process.platform === 'win32' ? ['py', 'python', 'python3'] : ['python3', 'python']
const args = ['-m', 'hermes_cli.main', ...process.argv.slice(2)]

let lastError = null

for (const command of candidates) {
  const result = spawnSync(command, args, { stdio: 'inherit' })

  if (result.error && result.error.code === 'ENOENT') {
    lastError = result.error
    continue
  }

  if (result.error) {
    console.error(result.error.message)
    process.exit(1)
  }

  process.exit(result.status ?? 0)
}

console.error('iola-hermes требует Python 3.11-3.13. Установите Python и повторите npm install -g iola-hermes.')
if (lastError && process.env.IOLA_HERMES_DEBUG) {
  console.error(lastError.message)
}
process.exit(1)
