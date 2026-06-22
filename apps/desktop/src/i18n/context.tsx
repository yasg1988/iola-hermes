import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'

import { getHermesConfigRecord, type HermesConfigRecord, saveHermesConfig } from '@/hermes'

import { TRANSLATIONS } from './catalog'
import { DEFAULT_LOCALE, localeConfigValue, normalizeLocale } from './languages'
import { setRuntimeI18nLocale } from './runtime'
import type { Locale, Translations } from './types'

export { LOCALE_META } from './languages'

export interface I18nConfigClient {
  getConfig: () => Promise<HermesConfigRecord>
  saveConfig: (config: HermesConfigRecord) => Promise<{ ok: boolean }>
}

const defaultConfigClient: I18nConfigClient = {
  getConfig: () => {
    if (typeof window === 'undefined' || !window.hermesDesktop?.api) {
      return Promise.resolve({})
    }

    return getHermesConfigRecord()
  },
  saveConfig: config => {
    if (typeof window === 'undefined' || !window.hermesDesktop?.api) {
      return Promise.resolve({ ok: true })
    }

    return saveHermesConfig(config)
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function getConfigDisplayLanguage(config: HermesConfigRecord): unknown {
  return isRecord(config.display) ? config.display.language : undefined
}

export function withConfigDisplayLanguage(config: HermesConfigRecord, locale: Locale): HermesConfigRecord {
  const display = isRecord(config.display) ? config.display : {}

  return {
    ...config,
    display: {
      ...display,
      language: localeConfigValue(locale)
    }
  }
}

function toError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

const RU_DEFAULT_MIGRATION_KEY = 'hermes-ru-default-locale-v1'

function shouldMigrateDesktopDefaultLocale(config: HermesConfigRecord): boolean {
  if (typeof window === 'undefined' || !window.hermesDesktop?.api) {
    return false
  }

  try {
    if (window.localStorage?.getItem(RU_DEFAULT_MIGRATION_KEY)) {
      return false
    }
  } catch {
    return false
  }

  const configured = getConfigDisplayLanguage(config)

  return configured == null || normalizeLocale(configured) === 'en'
}

function markDesktopDefaultLocaleMigrated() {
  try {
    window.localStorage?.setItem(RU_DEFAULT_MIGRATION_KEY, '1')
  } catch {
    // Best effort only. A blocked localStorage should not break boot.
  }
}

export interface I18nContextValue {
  configLoadError: Error | null
  isLoadingConfig: boolean
  isSavingLocale: boolean
  locale: Locale
  saveError: Error | null
  setLocale: (next: Locale) => Promise<void>
  t: Translations
}

const I18nContext = createContext<I18nContextValue>({
  configLoadError: null,
  isLoadingConfig: false,
  isSavingLocale: false,
  locale: DEFAULT_LOCALE,
  saveError: null,
  setLocale: async () => {},
  t: TRANSLATIONS[DEFAULT_LOCALE]
})

export interface I18nProviderProps {
  children: ReactNode
  configClient?: I18nConfigClient | null
  initialLocale?: unknown
}

export function I18nProvider({ children, configClient = defaultConfigClient, initialLocale }: I18nProviderProps) {
  const [locale, setLocaleState] = useState<Locale>(() => normalizeLocale(initialLocale))
  const [isLoadingConfig, setIsLoadingConfig] = useState(false)
  const [isSavingLocale, setIsSavingLocale] = useState(false)
  const [configLoadError, setConfigLoadError] = useState<Error | null>(null)
  const [saveError, setSaveError] = useState<Error | null>(null)
  const localeRef = useRef(locale)

  useEffect(() => {
    localeRef.current = locale
    setRuntimeI18nLocale(locale)
  }, [locale])

  useEffect(() => {
    if (!configClient) {
      return
    }

    let cancelled = false

    setIsLoadingConfig(true)
    setConfigLoadError(null)

    configClient
      .getConfig()
      .then(config => {
        if (!cancelled) {
          if (shouldMigrateDesktopDefaultLocale(config)) {
            const ruConfig = withConfigDisplayLanguage(config, DEFAULT_LOCALE)
            setLocaleState(DEFAULT_LOCALE)
            markDesktopDefaultLocaleMigrated()
            void configClient.saveConfig(ruConfig).catch(() => undefined)
          } else {
            setLocaleState(normalizeLocale(getConfigDisplayLanguage(config)))
          }
        }
      })
      .catch(error => {
        if (!cancelled) {
          setConfigLoadError(toError(error))
          setLocaleState(DEFAULT_LOCALE)
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoadingConfig(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [configClient, initialLocale])

  const setLocale = useCallback(
    async (next: Locale) => {
      const previousLocale = localeRef.current

      setSaveError(null)
      setLocaleState(next)

      if (!configClient) {
        return
      }

      setIsSavingLocale(true)

      try {
        const latestConfig = await configClient.getConfig()
        const result = await configClient.saveConfig(withConfigDisplayLanguage(latestConfig, next))

        if (!result.ok) {
          throw new Error('Failed to save language')
        }
      } catch (error) {
        const nextError = toError(error)

        setLocaleState(previousLocale)
        setSaveError(nextError)

        throw nextError
      } finally {
        setIsSavingLocale(false)
      }
    },
    [configClient]
  )

  const value = useMemo<I18nContextValue>(
    () => ({
      configLoadError,
      isLoadingConfig,
      isSavingLocale,
      locale,
      saveError,
      setLocale,
      t: TRANSLATIONS[locale]
    }),
    [configLoadError, isLoadingConfig, isSavingLocale, locale, saveError, setLocale]
  )

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export function useI18n(): I18nContextValue {
  return useContext(I18nContext)
}
