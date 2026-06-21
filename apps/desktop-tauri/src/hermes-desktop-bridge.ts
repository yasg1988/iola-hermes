import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { isPermissionGranted, onAction, registerActionTypes, requestPermission } from '@tauri-apps/plugin-notification'

type Unsubscribe = () => void
type DesktopBridge = Record<string, unknown>

interface ApiRequest {
  body?: unknown
  method?: string
  path: string
  profile?: null | string
  timeoutMs?: number
}

interface TerminalExit {
  code: null | number
  signal: null | string
}

interface BackendExit {
  code: null | number
  signal: null | string
}

interface BootProgress {
  error: null | string
  fakeMode: boolean
  message: string
  phase: string
  progress: number
  running: boolean
  timestamp: number
}

interface BootstrapStageDescriptor {
  category?: string
  name: string
  needs_user_input?: boolean
  title?: string
}

interface BootstrapStageResult {
  durationMs: null | number
  error: null | string
  json: null | { ok: boolean; reason?: null | string; skipped?: boolean; stage: string }
  startedAt: null | number
  state: 'failed' | 'pending' | 'running' | 'skipped' | 'succeeded'
}

interface BootstrapState {
  active: boolean
  completedAt: null | number
  error: null | string
  log: Array<{ line: string; stage: null | string; stream?: 'stderr' | 'stdout'; ts: number }>
  manifest: null | { protocolVersion: null | number; stages: BootstrapStageDescriptor[]; type: 'manifest' }
  stages: Record<string, BootstrapStageResult>
  startedAt: null | number
  unsupportedPlatform: null
}

type BootstrapEvent =
  | { protocolVersion: null | number; stages: BootstrapStageDescriptor[]; type: 'manifest' }
  | {
      durationMs?: number
      error?: null | string
      json?: BootstrapStageResult['json']
      name: string
      state: BootstrapStageResult['state']
      type: 'stage'
    }
  | { line: string; stage?: null | string; stream?: 'stderr' | 'stdout'; type: 'log' }
  | { marker: Record<string, unknown>; type: 'complete' }
  | { error: string; stage?: null | string; type: 'failed' }

interface HermesTitleBarTheme {
  background: string
  foreground: string
}

interface HermesWindowState {
  isFullscreen: boolean
  nativeOverlayWidth: number
  windowButtonPosition: null | { x: number; y: number }
}

interface HermesPreviewFileChanged {
  id: string
  path: string
  url: string
}

interface HermesPreviewWatch {
  id: string
  path: string
}

interface DeepLinkPayload {
  kind: string
  name: string
  params: Record<string, string>
}

interface HermesNotificationAction {
  id: string
  text: string
}

interface HermesNotification {
  actions?: HermesNotificationAction[]
  body?: string
  kind?: string
  sessionId?: null | string
  silent?: boolean
  title?: string
}

interface OauthLoginResult {
  baseUrl: string
  connected: boolean
  error?: string
  ok: boolean
  providerLabel?: string
  requiresCredentials?: boolean
  requiresExternalOauth?: boolean
}

const noopUnsubscribe: Unsubscribe = () => undefined

const ok = { ok: true }

const localEvents = new EventTarget()
const closePreviewCallbacks = new Set<() => void>()
const registeredNotificationActionTypes = new Set<string>()
const notificationPayloads = new Map<number, HermesNotification>()
let notificationActionListenerReady = false
let previewShortcutActive = false
let previewShortcutListenerReady = false

const TAURI_BOOTSTRAP_STAGE: BootstrapStageDescriptor = {
  category: 'tauri',
  name: 'tauri-backend',
  title: 'Tauri backend'
}

const tauriBootstrapManifest = {
  protocolVersion: 1,
  stages: [TAURI_BOOTSTRAP_STAGE],
  type: 'manifest' as const
}

const localConnectionConfig = {
  envOverride: false,
  mode: 'local',
  profile: null,
  remoteAuthMode: 'token',
  remoteOauthConnected: false,
  remoteTokenPreview: null,
  remoteTokenSet: false,
  remoteUrl: ''
}

function bridgeWindow() {
  return window as unknown as { hermesDesktop?: DesktopBridge }
}

function ensurePreviewShortcutListener() {
  if (previewShortcutListenerReady) {
    return
  }
  previewShortcutListenerReady = true
  window.addEventListener(
    'keydown',
    event => {
      const key = event.key.toLowerCase()
      const isCloseShortcut = key === 'w' && (event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey

      if (!previewShortcutActive || !isCloseShortcut) {
        return
      }

      event.preventDefault()
      event.stopPropagation()
      for (const callback of [...closePreviewCallbacks]) {
        callback()
      }
    },
    { capture: true }
  )
}

function onClosePreviewRequested(callback: () => void): Unsubscribe {
  closePreviewCallbacks.add(callback)

  return () => closePreviewCallbacks.delete(callback)
}

function onPowerResume(callback: () => void): Unsubscribe {
  let disposed = false
  let lastSignalAt = 0
  const signal = () => {
    if (disposed) {
      return
    }

    const now = Date.now()
    if (now - lastSignalAt < 750) {
      return
    }

    lastSignalAt = now
    callback()
  }
  const onVisible = () => {
    if (document.visibilityState === 'visible') {
      signal()
    }
  }

  window.addEventListener('online', signal)
  window.addEventListener('focus', signal)
  document.addEventListener('visibilitychange', onVisible)

  return () => {
    disposed = true
    window.removeEventListener('online', signal)
    window.removeEventListener('focus', signal)
    document.removeEventListener('visibilitychange', onVisible)
  }
}

function normalizeApiRequest(request: ApiRequest): ApiRequest {
  return {
    ...request,
    profile: request.profile ?? undefined
  }
}

function subscribe<T>(event: string, callback: (payload: T) => void): Unsubscribe {
  let active = true
  let cleanup: Unsubscribe = noopUnsubscribe

  void listen<T>(event, message => {
    if (active) {
      callback(message.payload)
    }
  }).then(unlisten => {
    if (active) {
      cleanup = unlisten
    } else {
      unlisten()
    }
  })

  return () => {
    active = false
    cleanup()
  }
}

function subscribeLocal<T>(event: string, callback: (payload: T) => void): Unsubscribe {
  const listener = (message: Event) => callback((message as CustomEvent<T>).detail)
  localEvents.addEventListener(event, listener)
  return () => localEvents.removeEventListener(event, listener)
}

function emitLocal<T>(event: string, payload: T) {
  localEvents.dispatchEvent(new CustomEvent(event, { detail: payload }))
}

function bootstrapStageFromProgress(progress: BootProgress): BootstrapStageResult {
  const failed = Boolean(progress.error)
  const succeeded = !progress.running && !failed && progress.progress >= 100

  return {
    durationMs: null,
    error: progress.error,
    json: progress.error
      ? { ok: false, reason: progress.error, stage: TAURI_BOOTSTRAP_STAGE.name }
      : { ok: true, stage: TAURI_BOOTSTRAP_STAGE.name },
    startedAt: progress.running ? progress.timestamp : null,
    state: failed ? 'failed' : succeeded ? 'succeeded' : progress.running ? 'running' : 'pending'
  }
}

function bootstrapStateFromProgress(progress: BootProgress): BootstrapState {
  const stage = bootstrapStageFromProgress(progress)
  const completed = !progress.running && !progress.error

  return {
    active: progress.running,
    completedAt: completed ? progress.timestamp : null,
    error: progress.error,
    log: progress.message
      ? [
          {
            line: progress.message,
            stage: progress.phase || TAURI_BOOTSTRAP_STAGE.name,
            stream: progress.error ? 'stderr' : 'stdout',
            ts: progress.timestamp
          }
        ]
      : [],
    manifest: tauriBootstrapManifest,
    stages: {
      [TAURI_BOOTSTRAP_STAGE.name]: stage
    },
    startedAt: progress.running ? progress.timestamp : null,
    unsupportedPlatform: null
  }
}

function bootstrapEventsFromProgress(progress: BootProgress): BootstrapEvent[] {
  const stage = bootstrapStageFromProgress(progress)
  const events: BootstrapEvent[] = [
    {
      error: stage.error,
      json: stage.json,
      name: TAURI_BOOTSTRAP_STAGE.name,
      state: stage.state,
      type: 'stage'
    }
  ]

  if (progress.message) {
    events.push({
      line: progress.message,
      stage: progress.phase || TAURI_BOOTSTRAP_STAGE.name,
      stream: progress.error ? 'stderr' : 'stdout',
      type: 'log'
    })
  }

  if (progress.error) {
    events.push({ error: progress.error, stage: progress.phase || TAURI_BOOTSTRAP_STAGE.name, type: 'failed' })
  } else if (!progress.running && progress.progress >= 100) {
    events.push({ marker: { runtime: 'tauri', stage: TAURI_BOOTSTRAP_STAGE.name }, type: 'complete' })
  }

  return events
}

async function getBootstrapState(): Promise<BootstrapState> {
  const progress = await invoke<BootProgress>('get_boot_progress')

  return bootstrapStateFromProgress(progress)
}

function onBootstrapEvent(callback: (payload: BootstrapEvent) => void): Unsubscribe {
  callback(tauriBootstrapManifest)

  return subscribe<BootProgress>('hermes:boot-progress', progress => {
    for (const event of bootstrapEventsFromProgress(progress)) {
      callback(event)
    }
  })
}

function bytesForInvoke(data: ArrayBuffer | Uint8Array): number[] {
  return Array.from(data instanceof Uint8Array ? data : new Uint8Array(data))
}

async function requestMicrophoneAccess() {
  if (!navigator.mediaDevices?.getUserMedia) {
    return false
  }
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
    stream.getTracks().forEach(track => track.stop())
    return true
  } catch {
    return false
  }
}

function normalizeUninstallMode(input: unknown) {
  if (typeof input === 'string') {
    return input
  }
  if (input && typeof input === 'object' && 'mode' in input) {
    return String((input as { mode?: unknown }).mode ?? 'lite')
  }
  return 'lite'
}

async function oauthLoginConnectionConfig(remoteUrl: string, credentials?: { password: string; username: string }) {
  return invoke<OauthLoginResult>('oauth_login_connection_config', { credentials, remoteUrl })
}

async function ensureNotificationPermission() {
  if (await isPermissionGranted()) {
    return true
  }

  return (await requestPermission()) === 'granted'
}

async function ensureNotificationActions(payload: HermesNotification) {
  const actions = Array.isArray(payload.actions) ? payload.actions.filter(action => action?.id && action?.text) : []
  if (actions.length === 0) {
    return undefined
  }

  const actionTypeId = `hermes-${actions.map(action => action.id).join('-')}`
  if (!registeredNotificationActionTypes.has(actionTypeId)) {
    await registerActionTypes([
      {
        actions: actions.map(action => ({ id: action.id, title: action.text })),
        id: actionTypeId
      }
    ])
    registeredNotificationActionTypes.add(actionTypeId)
  }

  return actionTypeId
}

function ensureNotificationActionListener() {
  if (notificationActionListenerReady) {
    return
  }

  notificationActionListenerReady = true
  void onAction(notification => {
    const id = typeof notification.id === 'number' ? notification.id : null
    const payload = id === null ? undefined : notificationPayloads.get(id)
    const actionId =
      typeof notification.extra?.actionId === 'string'
        ? notification.extra.actionId
        : typeof notification.extra?.action === 'string'
          ? notification.extra.action
          : undefined
    if (payload?.sessionId && actionId) {
      emitLocal('hermes:notification-action', { actionId, sessionId: payload.sessionId })
    }
  }).catch(() => {
    notificationActionListenerReady = false
  })
}

async function notify(payload: HermesNotification) {
  try {
    if (!(await ensureNotificationPermission())) {
      return false
    }

    ensureNotificationActionListener()
    const id = Date.now() % 2147483647
    const actionTypeId = await ensureNotificationActions(payload)
    notificationPayloads.set(id, payload)
    window.setTimeout(() => notificationPayloads.delete(id), 10 * 60 * 1000)

    const notification = new window.Notification(payload.title || 'Hermes RU Iola', {
      body: payload.body || '',
      data: {
        kind: payload.kind,
        sessionId: payload.sessionId ?? undefined
      },
      silent: Boolean(payload.silent),
      tag: payload.sessionId ? `hermes:${payload.sessionId}` : undefined,
      // Tauri's notification plugin consumes these extended fields where the
      // platform supports them; browsers ignore unknown NotificationOptions.
      ...(actionTypeId ? { actionTypeId, extra: { sessionId: payload.sessionId ?? undefined } } : {}),
      id
    } as NotificationOptions)

    notification.onclick = () => {
      if (payload.sessionId) {
        emitLocal('hermes:focus-session', payload.sessionId)
      }
      window.focus()
    }

    return true
  } catch {
    return false
  }
}

export function installHermesDesktopBridge() {
  const target = bridgeWindow()
  if (target.hermesDesktop) {
    return
  }

  target.hermesDesktop = {
    api: <T>(request: ApiRequest) => invoke<T>('hermes_api', { request: normalizeApiRequest(request) }),
    applyConnectionConfig: (payload: unknown) => invoke('apply_connection_config', { payload }),
    cancelBootstrap: () => invoke('cancel_bootstrap'),
    fetchLinkTitle: (url: string) => invoke('fetch_link_title', { url }),
    getBootProgress: () => invoke('get_boot_progress'),
    getBootstrapState,
    getConnection: (profile?: null | string) => invoke('get_connection', { profile }),
    getConnectionConfig: (profile?: null | string) => invoke('get_connection_config', { profile }),
    getGatewayWsUrl: (profile?: null | string) => invoke('get_gateway_ws_url', { profile }),
    getRecentLogs: () => invoke('get_recent_logs'),
    getVersion: () => invoke('get_version'),
    gitRoot: (path: string) => invoke('git_root', { path }),
    normalizePreviewTarget: (target: string, baseDir?: string) =>
      invoke('normalize_preview_target', { baseDir, rawTarget: target }),
    notify,
    oauthLoginConnectionConfig,
    oauthLogoutConnectionConfig: (remoteUrl?: string) => invoke('oauth_logout_connection_config', { remoteUrl }),
    onBackendExit: (callback: (payload: BackendExit) => void) => subscribe('hermes:backend-exit', callback),
    onBootProgress: (callback: (payload: BootProgress) => void) => subscribe('hermes:boot-progress', callback),
    onBootstrapEvent,
    onClosePreviewRequested,
    onDeepLink: (callback: (payload: DeepLinkPayload) => void) => subscribe('hermes:deep-link', callback),
    onFocusSession: (callback: (sessionId: string) => void) => subscribeLocal('hermes:focus-session', callback),
    onNotificationAction: (callback: (payload: { actionId: string; sessionId?: string }) => void) =>
      subscribeLocal('hermes:notification-action', callback),
    onOpenUpdatesRequested: (callback: () => void) => subscribe('hermes:open-updates', callback),
    onPowerResume,
    onPreviewFileChanged: (callback: (payload: HermesPreviewFileChanged) => void) =>
      subscribe('hermes:preview-file-changed', callback),
    onWindowStateChanged: (callback: (payload: HermesWindowState) => void) =>
      subscribe('hermes:window-state-changed', callback),
    openExternal: (url: string) => invoke('open_external', { url }),
    openNewSessionWindow: () => invoke('open_new_session_window'),
    openSessionWindow: (sessionId: string, opts?: { watch?: boolean }) =>
      invoke('open_session_window', { opts, sessionId }),
    profile: {
      get: () => invoke('get_active_profile'),
      set: (name: null | string) => invoke('set_active_profile', { name })
    },
    probeConnectionConfig: (remoteUrl: string) => invoke('probe_connection_config', { remoteUrl }),
    readDir: (path: string) => invoke('read_dir', { path }),
    readFileDataUrl: (filePath: string) => invoke('read_file_data_url', { filePath }),
    readFileText: (filePath: string) => invoke('read_file_text', { filePath }),
    repairBootstrap: () => invoke('repair_bootstrap'),
    requestMicrophoneAccess,
    resetBootstrap: () => invoke('reset_bootstrap'),
    revalidateConnection: () => invoke('revalidate_connection'),
    revealLogs: () => invoke('reveal_logs'),
    sanitizeWorkspaceCwd: (cwd?: null | string) => invoke('sanitize_workspace_cwd', { cwd }),
    saveClipboardImage: () => invoke('save_clipboard_image'),
    saveConnectionConfig: (payload: unknown) => invoke('save_connection_config', { payload }),
    saveImageBuffer: (data: ArrayBuffer | Uint8Array, ext: string) =>
      invoke('save_image_buffer', { data: bytesForInvoke(data), ext }),
    saveImageFromUrl: (url: string) => invoke('save_image_from_url', { url }),
    selectPaths: (options?: unknown) => invoke('select_paths', { options }),
    setNativeTheme: (mode: 'dark' | 'light' | 'system') => void invoke('set_native_theme', { mode }),
    setPreviewShortcutActive: (active: boolean) => {
      previewShortcutActive = Boolean(active)
      ensurePreviewShortcutListener()
    },
    setTitleBarTheme: (payload: HermesTitleBarTheme) => void invoke('set_title_bar_theme', { payload }),
    setTranslucency: (payload: { intensity: number }) => void invoke('set_translucency', { payload }),
    settings: {
      getDefaultProjectDir: () => invoke('get_default_project_dir'),
      pickDefaultProjectDir: () => invoke('pick_default_project_dir'),
      setDefaultProjectDir: (dir: null | string) => invoke('set_default_project_dir', { dir })
    },
    signalDeepLinkReady: () => invoke('signal_deep_link_ready'),
    stopPreviewFileWatch: (id: string) => invoke<boolean>('stop_preview_file_watch', { id }),
    terminal: {
      dispose: (id: string) => invoke('terminal_dispose', { id }),
      onData: (id: string, callback: (payload: string) => void) => subscribe(`hermes:terminal:${id}:data`, callback),
      onExit: (id: string, callback: (payload: TerminalExit) => void) =>
        subscribe(`hermes:terminal:${id}:exit`, callback),
      resize: (id: string, size: { cols: number; rows: number }) => invoke('terminal_resize', { id, size }),
      start: (options?: unknown) => invoke('terminal_start', { options }),
      write: (id: string, data: string) => invoke('terminal_write', { id, data })
    },
    testConnectionConfig: (payload: unknown) => invoke('test_connection_config', { payload }),
    themes: {
      fetchMarketplace: (id: string) => invoke('fetch_marketplace_themes', { id }),
      searchMarketplace: (query: string) => invoke('search_marketplace_themes', { query })
    },
    touchBackend: (profile?: string | null) => invoke('touch_backend', { profile }),
    uninstall: {
      run: (mode?: unknown) => invoke('uninstall_run', { mode: normalizeUninstallMode(mode) }),
      summary: () => invoke('uninstall_summary')
    },
    updates: {
      apply: (opts?: unknown) => invoke('updates_apply', { opts }),
      check: () => invoke('updates_check'),
      getBranch: () => invoke('updates_get_branch'),
      onProgress: (callback: (payload: unknown) => void) => subscribe('hermes:updates:progress', callback),
      setBranch: (name: string) => invoke('updates_set_branch', { name })
    },
    watchPreviewFile: (url: string) => invoke<HermesPreviewWatch>('watch_preview_file', { url }),
    worktrees: (cwds: string[]) => invoke('worktrees', { cwds }),
    writeClipboard: (text: string) => invoke('write_clipboard', { text })
  }
}
