import { invoke } from '@tauri-apps/api/core'

type Unsubscribe = () => void

interface ApiRequest {
  body?: unknown
  method?: string
  path: string
  profile?: string
  timeoutMs?: number
}

interface DesktopBridge {
  api: <T>(request: ApiRequest) => Promise<T>
  getBootProgress: () => Promise<unknown>
  getConnection: (profile?: string) => Promise<unknown>
  getGatewayWsUrl: (profile?: string) => Promise<string>
  getVersion: () => Promise<unknown>
  onBackendExit: (callback: (payload: unknown) => void) => Unsubscribe
  onBootProgress: (callback: (payload: unknown) => void) => Unsubscribe
  onClosePreviewRequested: (callback: () => void) => Unsubscribe
  onDeepLink: (callback: (payload: unknown) => void) => Unsubscribe
  onFocusSession: (callback: (sessionId: string) => void) => Unsubscribe
  onNotificationAction: (callback: (payload: unknown) => void) => Unsubscribe
  onOpenUpdatesRequested: (callback: () => void) => Unsubscribe
  onPowerResume: (callback: () => void) => Unsubscribe
  openExternal: (url: string) => Promise<boolean>
  revalidateConnection: () => Promise<unknown>
  signalDeepLinkReady: () => Promise<boolean>
  touchBackend: (profile?: string) => Promise<boolean>
}

declare global {
  interface Window {
    hermesDesktop?: DesktopBridge
  }
}

const noopUnsubscribe = () => undefined

export function installHermesDesktopBridge() {
  if (window.hermesDesktop) {
    return
  }

  window.hermesDesktop = {
    api: request => invoke('hermes_api', { request }),
    getBootProgress: () => invoke('get_boot_progress'),
    getConnection: profile => invoke('get_connection', { profile }),
    getGatewayWsUrl: profile => invoke('get_gateway_ws_url', { profile }),
    getVersion: () => invoke('get_version'),
    onBackendExit: () => noopUnsubscribe,
    onBootProgress: () => noopUnsubscribe,
    onClosePreviewRequested: () => noopUnsubscribe,
    onDeepLink: () => noopUnsubscribe,
    onFocusSession: () => noopUnsubscribe,
    onNotificationAction: () => noopUnsubscribe,
    onOpenUpdatesRequested: () => noopUnsubscribe,
    onPowerResume: () => noopUnsubscribe,
    openExternal: async url => {
      window.open(url, '_blank', 'noopener,noreferrer')
      return true
    },
    revalidateConnection: () => invoke('get_connection'),
    signalDeepLinkReady: async () => true,
    touchBackend: async () => true
  }
}
