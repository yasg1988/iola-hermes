import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

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

interface HermesTitleBarTheme {
  background: string
  foreground: string
}

interface HermesWindowState {
  isFullscreen: boolean
  nativeOverlayWidth: number
  windowButtonPosition: null | { x: number; y: number }
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

const emptyBootState = {
  active: false,
  completedAt: Date.now(),
  error: null,
  log: [],
  manifest: null,
  stages: {},
  startedAt: null,
  unsupportedPlatform: null
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

export function installHermesDesktopBridge() {
  const target = bridgeWindow()
  if (target.hermesDesktop) {
    return
  }

  target.hermesDesktop = {
    api: <T>(request: ApiRequest) => invoke<T>('hermes_api', { request: normalizeApiRequest(request) }),
    applyConnectionConfig: (payload: unknown) => invoke('apply_connection_config', { payload }),
    cancelBootstrap: async () => ({ cancelled: false, ok: true }),
    fetchLinkTitle: async (url: string) => url,
    getBootProgress: () => invoke('get_boot_progress'),
    getBootstrapState: async () => emptyBootState,
    getConnection: (profile?: null | string) => invoke('get_connection', { profile }),
    getConnectionConfig: (profile?: null | string) => invoke('get_connection_config', { profile }),
    getGatewayWsUrl: (profile?: null | string) => invoke('get_gateway_ws_url', { profile }),
    getPathForFile: () => '',
    getRecentLogs: () => invoke('get_recent_logs'),
    getRemoteDisplayReason: async () => null,
    getVersion: () => invoke('get_version'),
    gitRoot: (path: string) => invoke('git_root', { path }),
    normalizePreviewTarget: async (targetPath: string) => ({
      kind: /^https?:\/\//i.test(targetPath) ? 'url' : 'file',
      label: targetPath,
      source: targetPath,
      url: targetPath
    }),
    notify: async () => true,
    oauthLoginConnectionConfig,
    oauthLogoutConnectionConfig: (remoteUrl?: string) => invoke('oauth_logout_connection_config', { remoteUrl }),
    onBackendExit: (callback: (payload: BackendExit) => void) => subscribe('hermes:backend-exit', callback),
    onBootProgress: (callback: (payload: BootProgress) => void) => subscribe('hermes:boot-progress', callback),
    onBootstrapEvent: () => noopUnsubscribe,
    onClosePreviewRequested: () => noopUnsubscribe,
    onDeepLink: () => noopUnsubscribe,
    onFocusSession: () => noopUnsubscribe,
    onNotificationAction: () => noopUnsubscribe,
    onOpenUpdatesRequested: () => noopUnsubscribe,
    onPowerResume: () => noopUnsubscribe,
    onPreviewFileChanged: () => noopUnsubscribe,
    onWindowStateChanged: (callback: (payload: HermesWindowState) => void) =>
      subscribe('hermes:window-state-changed', callback),
    openExternal: (url: string) => invoke('open_external', { url }),
    openNewSessionWindow: async () => ok,
    openSessionWindow: async () => ok,
    profile: {
      get: async () => ({ profile: null }),
      set: async (name: null | string) => ({ profile: name })
    },
    probeConnectionConfig: (remoteUrl: string) => invoke('probe_connection_config', { remoteUrl }),
    readDir: (path: string) => invoke('read_dir', { path }),
    readFileDataUrl: (filePath: string) => invoke('read_file_data_url', { filePath }),
    readFileText: (filePath: string) => invoke('read_file_text', { filePath }),
    repairBootstrap: async () => ok,
    requestMicrophoneAccess,
    resetBootstrap: async () => ok,
    revalidateConnection: async () => ({ ok: true, rebuilt: false }),
    revealLogs: () => invoke('reveal_logs'),
    sanitizeWorkspaceCwd: (cwd?: null | string) => invoke('sanitize_workspace_cwd', { cwd }),
    saveClipboardImage: () => invoke('save_clipboard_image'),
    saveConnectionConfig: (payload: unknown) => invoke('save_connection_config', { payload }),
    saveImageBuffer: (data: ArrayBuffer | Uint8Array, ext: string) =>
      invoke('save_image_buffer', { data: bytesForInvoke(data), ext }),
    saveImageFromUrl: (url: string) => invoke('save_image_from_url', { url }),
    selectPaths: (options?: unknown) => invoke('select_paths', { options }),
    setNativeTheme: (mode: 'dark' | 'light' | 'system') => void invoke('set_native_theme', { mode }),
    setPreviewShortcutActive: () => undefined,
    setTitleBarTheme: (payload: HermesTitleBarTheme) => void invoke('set_title_bar_theme', { payload }),
    setTranslucency: (payload: { intensity: number }) => void invoke('set_translucency', { payload }),
    settings: {
      getDefaultProjectDir: async () => ({ defaultLabel: '', dir: null, resolvedCwd: '' }),
      pickDefaultProjectDir: async () => ({ canceled: true, dir: null }),
      setDefaultProjectDir: async (dir: null | string) => ({ dir })
    },
    signalDeepLinkReady: async () => ok,
    stopPreviewFileWatch: async () => true,
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
      fetchMarketplace: async () => ({ displayName: '', extensionId: '', themes: [] }),
      searchMarketplace: async () => []
    },
    touchBackend: async () => ok,
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
    watchPreviewFile: async (url: string) => ({ id: url, path: url }),
    worktrees: async () => ({}),
    writeClipboard: (text: string) => invoke('write_clipboard', { text })
  }
}
