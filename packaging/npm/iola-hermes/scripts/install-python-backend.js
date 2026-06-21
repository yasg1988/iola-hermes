'use strict'

const { spawnSync } = require('node:child_process')
const pkg = require('../package.json')

if (process.env.IOLA_HERMES_SKIP_PYTHON_INSTALL === '1') {
  console.log('Установка Python backend Hermes RU Iola пропущена.')
  process.exit(0)
}

const spec = process.env.IOLA_HERMES_PYTHON_SPEC || `iola-hermes==${pkg.version}`
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

for (const candidate of candidates) {
  const probe = spawnSync(candidate.command, [...candidate.prefix, '--version'], { stdio: 'ignore' })
  if (probe.error && probe.error.code === 'ENOENT') {
    continue
  }

  const install = spawnSync(candidate.command, [...candidate.prefix, '-m', 'pip', 'install', '--upgrade', spec], {
    stdio: 'inherit'
  })
  if (install.error) {
    console.error(install.error.message)
    process.exit(1)
  }
  process.exit(install.status ?? 0)
}

console.error('Python не найден. Установите Python 3.11-3.14 и повторите npm install -g iola-hermes.')
process.exit(1)
