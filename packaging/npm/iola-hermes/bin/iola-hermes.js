#!/usr/bin/env node
'use strict'

const { spawnSync } = require('node:child_process')

const candidates =
  process.platform === 'win32'
    ? [
        { command: 'py', prefix: ['-3.14'] },
        { command: 'py', prefix: ['-3.13'] },
        { command: 'py', prefix: ['-3.12'] },
        { command: 'py', prefix: ['-3.11'] },
        { command: 'python', prefix: [] },
        { command: 'python3', prefix: [] }
      ]
    : [
        { command: 'python3', prefix: [] },
        { command: 'python', prefix: [] }
      ]
const moduleArgs = ['-m', 'hermes_cli.main', ...process.argv.slice(2)]

let lastError = null

for (const candidate of candidates) {
  const result = spawnSync(candidate.command, [...candidate.prefix, ...moduleArgs], { stdio: 'inherit' })

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

console.error('iola-hermes требует Python 3.11-3.14. Установите совместимую версию Python и повторите npm install -g iola-hermes.')
if (lastError && process.env.IOLA_HERMES_DEBUG) {
  console.error(lastError.message)
}
process.exit(1)
