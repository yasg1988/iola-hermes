import { invoke } from '@tauri-apps/api/core'

type Unsubscribe = () => void
type DesktopBridge = Record<string, unknown>

interface ApiRequest {
  body?: unknown
  method?: string
  path: string
  profile?: null | string
  timeoutMs?: number
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
    gitRoot: async () => null,
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
    openExternal: async (url: string) => {
      window.open(url, '_blank', 'noopener,noreferrer')
    },
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
    readDir: async () => ({ entries: [], error: 'Чтение каталогов пока не реализовано в Tauri-оболочке' }),
    readFileDataUrl: unsupported('readFileDataUrl'),
    readFileText: unsupported('readFileText'),
    repairBootstrap: async () => ok,
    requestMicrophoneAccess: async () => false,
    resetBootstrap: async () => ok,
    revalidateConnection: async () => ({ ok: true, rebuilt: false }),
    revealLogs: async () => ({ error: 'Логи Tauri пока не подключены', ok: false, path: '' }),
    sanitizeWorkspaceCwd: async (cwd?: null | string) => ({ cwd: cwd ?? '', sanitized: false }),
    saveClipboardImage: unsupported('saveClipboardImage'),
    saveConnectionConfig: async () => localConnectionConfig,
    saveImageBuffer: unsupported('saveImageBuffer'),
    saveImageFromUrl: async () => false,
    selectPaths: async () => [],
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
      dispose: async () => false,
      onData: () => noopUnsubscribe,
      onExit: () => noopUnsubscribe,
      resize: async () => false,
      start: unsupported('terminal.start'),
      write: async () => false
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
    writeClipboard: async (text: string) => {
      await navigator.clipboard?.writeText(text)
      return true
    }
  }
}
