import { invoke } from '@tauri-apps/api/core'
import './styles.css'

interface BackendProbe {
  ok: boolean
  python: string | null
  version: string | null
  error: string | null
}

interface BackendProcess {
  pid: number
  python: string
  url: string
}

const app = document.querySelector<HTMLDivElement>('#app')

if (!app) {
  throw new Error('App root not found')
}

app.innerHTML = `
  <section class="shell">
    <header class="header">
      <div>
        <h1 class="title">Hermes RU Iola Tauri</h1>
        <p class="subtitle">Экспериментальная легкая оболочка на Rust + системный WebView.</p>
      </div>
      <div class="badge">MVP</div>
    </header>

    <section class="panel">
      <div class="controls">
        <button class="button" id="probe" type="button">Проверить backend</button>
        <button class="button secondary" id="version" type="button">Версия Hermes</button>
        <button class="button secondary" id="start" type="button">Запустить dashboard</button>
      </div>

      <div class="grid">
        <div class="field">
          <div class="label">Python</div>
          <div class="value" id="python">не проверено</div>
        </div>
        <div class="field">
          <div class="label">Статус backend</div>
          <div class="value" id="status">ожидание</div>
        </div>
        <div class="field">
          <div class="label">Версия</div>
          <div class="value" id="versionValue">не проверено</div>
        </div>
        <div class="field">
          <div class="label">Dashboard</div>
          <div class="value" id="dashboard">не запущен</div>
        </div>
      </div>

      <pre class="log" id="log"></pre>
    </section>
  </section>
`

const elements = {
  dashboard: getElement('dashboard'),
  log: getElement('log'),
  probe: getElement('probe') as HTMLButtonElement,
  python: getElement('python'),
  start: getElement('start') as HTMLButtonElement,
  status: getElement('status'),
  version: getElement('version') as HTMLButtonElement,
  versionValue: getElement('versionValue')
}

function getElement(id: string) {
  const el = document.getElementById(id)
  if (!el) {
    throw new Error(`Element #${id} not found`)
  }
  return el
}

function log(line: string) {
  const stamp = new Date().toLocaleTimeString()
  elements.log.textContent += `[${stamp}] ${line}\n`
  elements.log.scrollTop = elements.log.scrollHeight
}

async function runButton(button: HTMLButtonElement, task: () => Promise<void>) {
  button.disabled = true
  try {
    await task()
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    log(`Ошибка: ${message}`)
  } finally {
    button.disabled = false
  }
}

elements.probe.addEventListener('click', () =>
  runButton(elements.probe, async () => {
    log('Проверяю Python backend...')
    const probe = await invoke<BackendProbe>('backend_probe')
    elements.python.textContent = probe.python ?? 'не найден'
    elements.status.textContent = probe.ok ? 'готов' : 'нужна установка/настройка'
    elements.versionValue.textContent = probe.version ?? 'неизвестно'
    log(probe.ok ? 'Backend доступен.' : `Backend недоступен: ${probe.error ?? 'неизвестная ошибка'}`)
  })
)

elements.version.addEventListener('click', () =>
  runButton(elements.version, async () => {
    log('Запрашиваю версию Hermes...')
    const version = await invoke<string>('backend_version')
    elements.versionValue.textContent = version || 'пустой ответ'
    log(`Версия: ${version}`)
  })
)

elements.start.addEventListener('click', () =>
  runButton(elements.start, async () => {
    log('Запускаю dashboard на 127.0.0.1:9119...')
    const proc = await invoke<BackendProcess>('start_backend', { host: '127.0.0.1', port: 9119 })
    elements.dashboard.innerHTML = `<a href="${proc.url}" target="_blank" rel="noreferrer">${proc.url}</a> · PID ${proc.pid}`
    log(`Dashboard запущен: ${proc.url}, PID ${proc.pid}`)
  })
)

void (async () => {
  try {
    const probe = await invoke<BackendProbe>('backend_probe')
    elements.python.textContent = probe.python ?? 'не найден'
    elements.status.textContent = probe.ok ? 'готов' : 'нужна установка/настройка'
    elements.versionValue.textContent = probe.version ?? 'неизвестно'
    log(probe.ok ? 'Автопроверка backend успешна.' : `Автопроверка: ${probe.error ?? 'backend недоступен'}`)
  } catch (error) {
    log(`Автопроверка не выполнена: ${error instanceof Error ? error.message : String(error)}`)
  }
})()
