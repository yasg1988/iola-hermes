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

function unsupported(name: string) {
  return async () => {
    throw new Error(`${name} пока не реализован в Tauri-оболочке`)
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

export function installHermesDesktopBridge() {
  const target = bridgeWindow()
  if (target.hermesDesktop) {
    return
  }

  target.hermesDesktop = {
    api: <T>(request: ApiRequest) => invoke<T>('hermes_api', { request: normalizeApiRequest(request) }),
    applyConnectionConfig: async () => localConnectionConfig,
    cancelBootstrap: async () => ({ cancelled: false, ok: true }),
    fetchLinkTitle: async (url: string) => url,
    getBootProgress: () => invoke('get_boot_progress'),
    getBootstrapState: async () => emptyBootState,
    getConnection: (profile?: null | string) => invoke('get_connection', { profile }),
    getConnectionConfig: async () => localConnectionConfig,
    getGatewayWsUrl: (profile?: null | string) => invoke('get_gateway_ws_url', { profile }),
    getPathForFile: () => '',
    getRecentLogs: async () => ({ lines: [], path: '' }),
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
    oauthLoginConnectionConfig: async () => ({ baseUrl: '', connected: false, ok: false }),
    oauthLogoutConnectionConfig: async () => ({ connected: false, ok: true }),
    onBackendExit: () => noopUnsubscribe,
    onBootProgress: () => noopUnsubscribe,
    onBootstrapEvent: () => noopUnsubscribe,
    onClosePreviewRequested: () => noopUnsubscribe,
    onDeepLink: () => noopUnsubscribe,
    onFocusSession: () => noopUnsubscribe,
    onNotificationAction: () => noopUnsubscribe,
    onOpenUpdatesRequested: () => noopUnsubscribe,
    onPowerResume: () => noopUnsubscribe,
    onPreviewFileChanged: () => noopUnsubscribe,
    onWindowStateChanged: () => noopUnsubscribe,
    openExternal: (url: string) => invoke('open_external', { url }),
    openNewSessionWindow: async () => ok,
    openSessionWindow: async () => ok,
    profile: {
      get: async () => ({ profile: null }),
      set: async (name: null | string) => ({ profile: name })
    },
    probeConnectionConfig: async (remoteUrl: string) => ({
      authMode: 'unknown',
      baseUrl: remoteUrl,
      error: null,
      providers: [],
      reachable: false,
      version: null
    }),
    readDir: (path: string) => invoke('read_dir', { path }),
    readFileDataUrl: (filePath: string) => invoke('read_file_data_url', { filePath }),
    readFileText: (filePath: string) => invoke('read_file_text', { filePath }),
    repairBootstrap: async () => ok,
    requestMicrophoneAccess: async () => false,
    resetBootstrap: async () => ok,
    revalidateConnection: async () => ({ ok: true, rebuilt: false }),
    revealLogs: async () => ({ error: 'Логи Tauri пока не подключены', ok: false, path: '' }),
    sanitizeWorkspaceCwd: (cwd?: null | string) => invoke('sanitize_workspace_cwd', { cwd }),
    saveClipboardImage: unsupported('saveClipboardImage'),
    saveConnectionConfig: async () => localConnectionConfig,
    saveImageBuffer: unsupported('saveImageBuffer'),
    saveImageFromUrl: async () => false,
    selectPaths: (options?: unknown) => invoke('select_paths', { options }),
    setNativeTheme: () => undefined,
    setPreviewShortcutActive: () => undefined,
    setTitleBarTheme: () => undefined,
    setTranslucency: () => undefined,
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
    testConnectionConfig: async () => ({ baseUrl: '', ok: false, version: null }),
    themes: {
      fetchMarketplace: async () => ({ displayName: '', extensionId: '', themes: [] }),
      searchMarketplace: async () => []
    },
    touchBackend: async () => ok,
    uninstall: {
      run: async () => ({ error: 'Удаление через Tauri пока не реализовано', ok: false }),
      summary: async () => ({
        agent_installed: false,
        gui_installed: false,
        hermes_home: '',
        packaged_app_paths: [],
        platform: navigator.platform,
        source_built_artifacts: [],
        userdata_dir: '',
        userdata_exists: false
      })
    },
    updates: {
      apply: async () => ({ error: 'Обновления через Tauri пока не реализованы', ok: false }),
      check: async () => ({ reason: 'tauri-experimental', supported: false }),
      getBranch: async () => ({ branch: 'main' }),
      onProgress: () => noopUnsubscribe,
      setBranch: async (branch: string) => ({ branch })
    },
    watchPreviewFile: async (url: string) => ({ id: url, path: url }),
    worktrees: async () => ({}),
    writeClipboard: (text: string) => invoke('write_clipboard', { text })
  }
}
