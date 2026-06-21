import { installHermesDesktopBridge } from './hermes-desktop-bridge'

installHermesDesktopBridge()

void import('../../desktop/src/main')
