import { Box, Text, useInput } from '@hermes/ink'
import { useState } from 'react'

import type { BillingOverlayState } from '../app/interfaces.js'
import type { BillingStateResponse } from '../gatewayTypes.js'
import type { Theme } from '../theme.js'

import { TextInput } from './textInput.js'

const SPEND_BAR_CELLS = 10

interface BillingOverlayProps {
  /** Replace the overlay slot (screen transitions + pending data). */
  onPatch: (next: Partial<BillingOverlayState>) => void
  /** Close the overlay entirely. */
  onClose: () => void
  overlay: BillingOverlayState
  t: Theme
}

/** A numbered menu row with the ▸ cursor (mirrors ClarifyPrompt). */
function MenuRow({ active, index, label, t }: { active: boolean; index: number; label: string; t: Theme }) {
  return (
    <Text>
      <Text bold={active} color={active ? t.color.label : t.color.muted} inverse={active}>
        {active ? '▸ ' : '  '}
        {index}. {label}
      </Text>
    </Text>
  )
}

/** Plain (non-numbered) action row with the ▸ cursor (confirm screens). */
function ActionRow({ active, label, color, t }: { active: boolean; label: string; color?: string; t: Theme }) {
  return (
    <Text>
      <Text color={active ? t.color.accent : t.color.muted}>{active ? '▸ ' : '  '}</Text>
      <Text bold={active} color={active ? (color ?? t.color.text) : t.color.muted}>
        {label}
      </Text>
    </Text>
  )
}

/** 10-cell spend bar + percent (omit entirely when there's no usable cap). */
function spendBar(s: BillingStateResponse): null | string {
  const cap = s.monthly_cap

  if (!cap || cap.limit_usd == null) {
    return null
  }

  const limit = Number(cap.limit_usd)
  const spent = Number(cap.spent_this_month_usd ?? '0')

  if (!(limit > 0) || Number.isNaN(spent)) {
    return null
  }

  const ratio = Math.max(0, Math.min(1, spent / limit))
  const filled = Math.round(ratio * SPEND_BAR_CELLS)
  const bar = '█'.repeat(filled) + '░'.repeat(SPEND_BAR_CELLS - filled)
  const pct = Math.round(ratio * 100)
  const ceiling = cap.is_default_ceiling ? ' (порог по умолчанию)' : ''

  return `Использовано ${cap.spent_display} из ${cap.limit_display}   ${bar} ${pct}%${ceiling}`
}

function autoReloadLine(s: BillingStateResponse): null | string {
  if (!s.auto_reload) {
    return null
  }

  return s.auto_reload.enabled
    ? `Автопополнение: включено (ниже ${s.auto_reload.threshold_display} → ${s.auto_reload.reload_to_display})`
    : 'Автопополнение: выключено'
}

const footer = (extra: string, t: Theme) => <Text color={t.color.muted}>{extra}</Text>

/**
 * The /billing modal.  A self-contained state machine:
 *   overview → buy | autoreload | limit  (and buy → confirm).
 * Esc from a sub-screen returns to overview; Esc from overview closes.
 * All RPCs + error mapping live in billing.ts and are reached through
 * `overlay.ctx` — this component only renders + routes keys.
 */
export function BillingOverlay({ onClose, onPatch, overlay, t }: BillingOverlayProps) {
  const { ctx, screen, state: s } = overlay

  return (
    <Box borderColor={t.color.accent} borderStyle="round" flexDirection="column" paddingX={1}>
      {screen === 'overview' && <OverviewScreen ctx={ctx} onClose={onClose} onPatch={onPatch} s={s} t={t} />}
      {screen === 'buy' && <BuyScreen ctx={ctx} onClose={onClose} onPatch={onPatch} s={s} t={t} />}
      {screen === 'confirm' && (
        <ConfirmScreen
          amount={overlay.pendingCharge?.amount ?? ''}
          ctx={ctx}
          onBack={() => onPatch({ pendingCharge: null, screen: 'buy' })}
          onClose={onClose}
          s={s}
          t={t}
        />
      )}
      {screen === 'autoreload' && <AutoReloadScreen ctx={ctx} onClose={onClose} onPatch={onPatch} s={s} t={t} />}
      {screen === 'limit' && <LimitScreen ctx={ctx} onClose={onClose} onPatch={onPatch} s={s} t={t} />}
    </Box>
  )
}

// ── Screen 1: Overview ────────────────────────────────────────────────

interface ScreenProps {
  ctx: BillingOverlayState['ctx']
  onClose: () => void
  onPatch: (next: Partial<BillingOverlayState>) => void
  s: BillingStateResponse
  t: Theme
}

function OverviewScreen({ ctx, onClose, onPatch, s, t }: ScreenProps) {
  // Gate: full menu only for an admin with the kill-switch on. Otherwise the
  // menu collapses to Manage-on-portal / Cancel + a one-line note.
  const full = s.is_admin && s.cli_billing_enabled

  const note = !s.is_admin
    ? 'Действия с оплатой доступны администратору или владельцу организации.'
    : !s.cli_billing_enabled
      ? 'Оплата из терминала выключена для этой организации — включите ее на портале.'
      : null

  // Optimistic funnel: admin + kill-switch on but no saved card → a charge will
  // 403 no_payment_method. Advise up front (Buy stays available — /state.card
  // can't fully prove CLI-chargeability, so we hint rather than hide).
  const cardHint = full && !s.card ? 'Нет сохраненной карты для оплат из терминала — сначала добавьте ее на портале.' : null

  const items = full
    ? ['Купить кредиты', 'Настроить автопополнение', 'Настроить месячный лимит', 'Открыть портал', 'Отмена']
    : ['Открыть портал', 'Отмена']

  const [sel, setSel] = useState(0)

  const choose = (i: number) => {
    if (full) {
      if (i === 0) {
        onPatch({ screen: 'buy' })
      } else if (i === 1) {
        onPatch({ screen: 'autoreload' })
      } else if (i === 2) {
        onPatch({ screen: 'limit' })
      } else if (i === 3) {
        if (s.portal_url) {
          ctx.openPortal(s.portal_url)
        }

        onClose()
      } else {
        onClose()
      }
    } else {
      if (i === 0 && s.portal_url) {
        ctx.openPortal(s.portal_url)
      }

      onClose()
    }
  }

  useInput((ch, key) => {
    if (key.escape) {
      return onClose()
    }

    if (key.upArrow && sel > 0) {
      setSel(v => v - 1)
    }

    if (key.downArrow && sel < items.length - 1) {
      setSel(v => v + 1)
    }

    if (key.return) {
      return choose(sel)
    }

    const n = parseInt(ch, 10)

    if (n >= 1 && n <= items.length) {
      return choose(n - 1)
    }
  })

  const bar = spendBar(s)
  const auto = autoReloadLine(s)

  return (
    <Box flexDirection="column">
      <Text bold color={t.color.accent}>
        Кредиты использования
      </Text>
      {bar && <Text color={t.color.text}>{bar}</Text>}
      <Text color={t.color.text}>Баланс: {s.balance_display}</Text>
      {auto && <Text color={t.color.muted}>{auto}</Text>}
      {s.org_name && (
        <Text color={t.color.muted}>
          Организация: {s.org_name}
          {s.role ? ` · ${s.role}` : ''}
        </Text>
      )}
      {note && (
        <Box marginTop={1}>
          <Text color={t.color.warn}>{note}</Text>
        </Box>
      )}
      {cardHint && (
        <Box marginTop={1}>
          <Text color={t.color.warn}>{cardHint}</Text>
        </Box>
      )}
      {cardHint && s.portal_url && <Text color={t.color.muted}>Портал: {s.portal_url}</Text>}

      <Text />
      {items.map((label, i) => (
        <MenuRow active={sel === i} index={i + 1} key={label} label={label} t={t} />
      ))}

      <Text />
      {footer(`↑/↓ выбор · 1-${items.length} быстрый выбор · Enter подтвердить · Esc закрыть`, t)}
    </Box>
  )
}

// ── Screen 2: Buy credits ─────────────────────────────────────────────

function BuyScreen({ ctx, onPatch, s, t }: ScreenProps) {
  const presets = s.charge_presets_display
  const rawPresets = s.charge_presets
  // rows: [...presets, 'Custom amount…', 'Cancel']
  const rows = [...presets, 'Своя сумма...', 'Отмена']
  const customIdx = presets.length

  const [sel, setSel] = useState(0)
  const [typing, setTyping] = useState(false)
  const [custom, setCustom] = useState('')
  const [error, setError] = useState<null | string>(null)

  const toConfirm = (amount: string) => {
    onPatch({ pendingCharge: { amount }, screen: 'confirm' })
  }

  const pickPreset = (i: number) => {
    // Prefer the raw (numeric) preset for the amount; fall back to stripping $.
    const raw = (rawPresets[i] ?? presets[i] ?? '').replace(/^\$/, '').trim()
    const v = ctx.validate(raw)

    if (v.error || !v.amount) {
      setError(v.error ?? 'Некорректный пресет.')

      return
    }

    toConfirm(v.amount)
  }

  const submitCustom = (raw: string) => {
    const v = ctx.validate(raw)

    if (v.error || !v.amount) {
      setError(v.error ?? 'Некорректная сумма.')

      return
    }

    toConfirm(v.amount)
  }

  const choose = (i: number) => {
    if (i < presets.length) {
      pickPreset(i)
    } else if (i === customIdx) {
      setError(null)
      setTyping(true)
    } else {
      onPatch({ screen: 'overview' })
    }
  }

  useInput((ch, key) => {
    if (key.escape) {
      return typing ? (setTyping(false), setError(null)) : onPatch({ screen: 'overview' })
    }

    if (typing) {
      return
    }

    if (key.upArrow && sel > 0) {
      setSel(v => v - 1)
    }

    if (key.downArrow && sel < rows.length - 1) {
      setSel(v => v + 1)
    }

    if (key.return) {
      return choose(sel)
    }

    const n = parseInt(ch, 10)

    if (n >= 1 && n <= rows.length) {
      return choose(n - 1)
    }
  })

  const payLine = s.card ? `Оплата: ${s.card.masked}` : 'Сохраненной карты нет'

  if (typing) {
    return (
      <Box flexDirection="column">
        <Text bold color={t.color.accent}>
          Купить кредиты использования
        </Text>
        <Text color={t.color.muted}>{payLine}</Text>
        <Text />
        <Text color={t.color.label}>Введите свою сумму:</Text>
        <Box>
          <Text color={t.color.label}>{'$'}</Text>
          <TextInput columns={20} onChange={setCustom} onSubmit={submitCustom} value={custom} />
        </Box>
        {error && <Text color={t.color.error}>{error}</Text>}
        <Text />
        {footer('Enter подтвердить · Esc назад', t)}
      </Box>
    )
  }

  return (
    <Box flexDirection="column">
      <Text bold color={t.color.accent}>
        Купить кредиты использования
      </Text>
      <Text color={t.color.muted}>{payLine}</Text>
      <Text />
      {rows.map((label, i) => (
        <MenuRow active={sel === i} index={i + 1} key={label} label={label} t={t} />
      ))}
      {error && <Text color={t.color.error}>{error}</Text>}
      <Text />
      {footer(`↑/↓ выбор · 1-${rows.length} быстрый выбор · Enter подтвердить · Esc назад`, t)}
    </Box>
  )
}

// ── Screen 3: Confirm purchase ────────────────────────────────────────

function ConfirmScreen({
  amount,
  ctx,
  onBack,
  onClose,
  s,
  t
}: {
  amount: string
  ctx: BillingOverlayState['ctx']
  onBack: () => void
  onClose: () => void
  s: BillingStateResponse
  t: Theme
}) {
  // rows: Pay $X now / Cancel
  const [sel, setSel] = useState(0)

  const pay = () => {
    ctx.charge(amount)
    // Settlement is reported via transcript lines; close the overlay now.
    onClose()
  }

  const back = () => onBack()

  useInput((ch, key) => {
    if (key.escape) {
      return back()
    }

    const lower = ch.toLowerCase()

    if (lower === 'y') {
      return pay()
    }

    if (lower === 'n') {
      return back()
    }

    if (key.upArrow) {
      setSel(0)
    }

    if (key.downArrow) {
      setSel(1)
    }

    if (key.return) {
      return sel === 0 ? pay() : back()
    }
  })

  const payLine = s.card ? `Оплата: ${s.card.masked}` : 'Сохраненной карты нет'

  return (
    <Box flexDirection="column">
      <Text bold color={t.color.accent}>
        Подтвердить покупку
      </Text>
      <Text color={t.color.text}>Итого: ${amount}</Text>
      <Text color={t.color.muted}>{payLine}</Text>
      <Text color={t.color.muted}>Подтверждая действие, вы разрешаете Nous Research списать средства с карты.</Text>
      <Text />
      <ActionRow active={sel === 0} color={t.color.ok} label={`Оплатить $${amount} сейчас`} t={t} />
      <ActionRow active={sel === 1} label="Отмена" t={t} />
      <Text />
      {footer('↑/↓ выбор · Enter подтвердить · Y/N быстро · Esc назад', t)}
    </Box>
  )
}

// ── Screen 4: Auto-reload (the 2-field form) ──────────────────────────

function AutoReloadScreen({ ctx, onClose, onPatch, s, t }: ScreenProps) {
  const ar = s.auto_reload
  const enabled = Boolean(ar?.enabled)

  // Prefill from state (strip the $ from the *_usd raw fields if present).
  const prefill = (raw?: null | string) => (raw == null ? '' : String(raw).replace(/^\$/, '').trim())
  const [threshold, setThreshold] = useState(prefill(ar?.threshold_usd))
  const [reloadTo, setReloadTo] = useState(prefill(ar?.reload_to_usd))
  const [field, setField] = useState<'reloadTo' | 'threshold'>('threshold')
  const [error, setError] = useState<null | string>(null)
  // focusRow: 0=threshold field, 1=reloadTo field, 2=Agree, 3=Turn off (if enabled), last=Cancel
  const agreeLabel = 'Согласиться и включить'
  const turnOffLabel = 'Выключить'
  const cancelLabel = 'Отмена'
  const actionRows = enabled ? [agreeLabel, turnOffLabel, cancelLabel] : [agreeLabel, cancelLabel]
  const FIELD_ROWS = 2
  const [row, setRow] = useState(0)

  const noCard = !s.card

  const validatePair = (): null | { reloadTo: string; threshold: string } => {
    const tv = ctx.validate(threshold)

    if (tv.error || !tv.amount) {
      setError(`Порог: ${tv.error ?? 'некорректное значение'}`)

      return null
    }

    const rv = ctx.validate(reloadTo)

    if (rv.error || !rv.amount) {
      setError(`Пополнение до: ${rv.error ?? 'некорректное значение'}`)

      return null
    }

    if (Number(rv.amount) <= Number(tv.amount)) {
      setError('Сумма пополнения должна быть больше порога.')

      return null
    }

    setError(null)

    return { reloadTo: rv.amount, threshold: tv.amount }
  }

  const turnOn = () => {
    if (noCard) {
      ctx.sys('🔴 Сохраненной карты нет — сначала добавьте ее на портале.')

      if (s.portal_url) {
        ctx.openPortal(s.portal_url)
      }

      onClose()

      return
    }

    const pair = validatePair()

    if (!pair) {
      return
    }

    void ctx.applyAutoReload(true, Number(pair.threshold), Number(pair.reloadTo)).then(ok => {
      if (ok) {
        ctx.sys(`✅ Автопополнение включено: ниже $${pair.threshold} → пополнить до $${pair.reloadTo}.`)
      }
    })
    onClose()
  }

  const turnOff = () => {
    void ctx.applyAutoReload(false).then(ok => {
      if (ok) {
        ctx.sys('✅ Автопополнение выключено.')
      }
    })
    onClose()
  }

  const onAction = (label: string) => {
    if (label === agreeLabel) {
      turnOn()
    } else if (label === turnOffLabel) {
      turnOff()
    } else {
      onPatch({ screen: 'overview' })
    }
  }

  const editingField = row < FIELD_ROWS

  useInput((ch, key) => {
    if (key.escape) {
      return onPatch({ screen: 'overview' })
    }

    if (key.upArrow && row > 0) {
      setRow(v => v - 1)
      setField(row - 1 === 0 ? 'threshold' : 'reloadTo')
    }

    if (key.downArrow && row < FIELD_ROWS + actionRows.length - 1) {
      setRow(v => v + 1)
      setField(row + 1 === 0 ? 'threshold' : 'reloadTo')
    }

    // Tab cycles between the two fields when focused on a field.
    if (key.tab && editingField) {
      const next = field === 'threshold' ? 'reloadTo' : 'threshold'
      setField(next)
      setRow(next === 'threshold' ? 0 : 1)
    }

    if (key.return && !editingField) {
      const idx = row - FIELD_ROWS

      return onAction(actionRows[idx] ?? cancelLabel)
    }

    // a number quick-picks an action row (1..actionRows.length)
    if (!editingField) {
      const n = parseInt(ch, 10)

      if (n >= 1 && n <= actionRows.length) {
        return onAction(actionRows[n - 1]!)
      }
    }
  })

  const cardLine = s.card ? `Сохраненная карта: ${s.card.masked}` : 'Сохраненной карты нет'

  const fieldBox = (label: string, value: string, onChange: (v: string) => void, focused: boolean, key: string) => (
    <Box flexDirection="column" key={key}>
      <Text color={focused ? t.color.label : t.color.muted}>{label}</Text>
      <Box borderColor={focused ? t.color.accent : t.color.border} borderStyle="round" paddingX={1}>
        <Text color={t.color.label}>{'$'}</Text>
        <TextInput
          columns={16}
          focus={focused}
          onChange={onChange}
          onSubmit={() => {
            // Enter inside the threshold field jumps to reload-to; inside
            // reload-to jumps to the Agree action.
            if (key === 'threshold') {
              setField('reloadTo')
              setRow(1)
            } else {
              setRow(FIELD_ROWS)
            }
          }}
          value={value}
        />
      </Box>
    </Box>
  )

  return (
    <Box flexDirection="column">
      <Text bold color={t.color.accent}>
        Автопополнение
      </Text>
      <Text color={t.color.muted}>Автоматически покупать кредиты, когда баланс становится низким.</Text>
      <Text color={t.color.muted}>{cardLine}</Text>
      <Text />
      {fieldBox('Когда баланс ниже:', threshold, setThreshold, row === 0, 'threshold')}
      {fieldBox('Пополнять баланс до:', reloadTo, setReloadTo, row === 1, 'reloadTo')}
      <Text />
      <Text color={t.color.muted}>
        Подтверждая действие, вы разрешаете Nous Research списывать средства с {s.card ? s.card.masked : 'вашей карты'},
        когда баланс опускается ниже порога. Отключить это можно здесь или на портале.
      </Text>
      {error && <Text color={t.color.error}>{error}</Text>}
      <Text />
      {actionRows.map((label, i) => (
        <ActionRow
          active={!editingField && row - FIELD_ROWS === i}
          color={label === turnOffLabel ? t.color.warn : label === agreeLabel ? t.color.ok : t.color.text}
          key={label}
          label={label}
          t={t}
        />
      ))}
      <Text />
      {footer('↑/↓ перемещение · Tab сменить поле · Enter дальше/подтвердить · Esc назад', t)}
    </Box>
  )
}

// ── Screen 5: Monthly spend limit (read-only) ─────────────────────────

function LimitScreen({ ctx, onClose, onPatch, s, t }: ScreenProps) {
  const rows = ['Открыть портал', 'Отмена']
  const [sel, setSel] = useState(0)

  const choose = (i: number) => {
    if (i === 0 && s.portal_url) {
      ctx.openPortal(s.portal_url)

      return onClose()
    }

    onPatch({ screen: 'overview' })
  }

  useInput((ch, key) => {
    if (key.escape) {
      return onPatch({ screen: 'overview' })
    }

    if (key.upArrow && sel > 0) {
      setSel(v => v - 1)
    }

    if (key.downArrow && sel < rows.length - 1) {
      setSel(v => v + 1)
    }

    if (key.return) {
      return choose(sel)
    }

    const n = parseInt(ch, 10)

    if (n >= 1 && n <= rows.length) {
      return choose(n - 1)
    }
  })

  const cap = s.monthly_cap

  const usageLine =
    cap && cap.limit_usd != null
      ? `Использовано ${cap.spent_display} из ${cap.limit_display} за этот месяц${cap.is_default_ceiling ? ' (порог по умолчанию)' : ''}`
      : 'Месячный лимит не виден здесь, он управляется на портале.'

  return (
    <Box flexDirection="column">
      <Text bold color={t.color.accent}>
        Месячный лимит расходов
      </Text>
      <Text color={t.color.text}>{usageLine}</Text>
      <Text color={t.color.muted}>Месячный лимит задается на портале, здесь он показан только для чтения.</Text>
      <Text />
      {rows.map((label, i) => (
        <MenuRow active={sel === i} index={i + 1} key={label} label={label} t={t} />
      ))}
      <Text />
      {footer(`↑/↓ выбор · 1-${rows.length} быстрый выбор · Enter подтвердить · Esc назад`, t)}
    </Box>
  )
}
