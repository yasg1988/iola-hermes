export interface CommandsCatalogSection {
  name: string
  pairs: [string, string][]
}

export interface CommandsCatalogLike {
  categories?: CommandsCatalogSection[]
  pairs?: [string, string][]
  skill_count?: number
  warning?: string
}

export interface DesktopSlashCompletion {
  display: string
  meta: string
  text: string
}

export interface DesktopThemeCommandOption {
  description: string
  label: string
  name: string
}

/**
 * Local client action a command resolves to. Each id maps to exactly one
 * handler in the dispatcher (`use-prompt-actions`), so adding a command never
 * means adding a branch to a switch ladder — you add a row here + a handler
 * keyed by the id.
 */
export type DesktopActionId =
  | 'branch'
  | 'browser'
  | 'handoff'
  | 'help'
  | 'new'
  | 'profile'
  | 'skin'
  | 'title'
  | 'yolo'

/** A command fulfilled by opening a desktop overlay picker. */
export type DesktopPickerId = 'model' | 'session'

/** Why a known Hermes command has no desktop UI surface. */
export type DesktopUnavailableReason = 'advanced' | 'messaging' | 'settings' | 'terminal'

/**
 * How the desktop fulfils a command. This is the single discriminator the
 * dispatcher, popover, pills, and pickers all read — no parallel block-lists.
 *
 * - `action`     → handled by a local client handler (new chat, branch, …)
 * - `picker`     → opens an overlay (`/model`, `/resume`); a typed arg is
 *                  resolved by that picker instead of falling through
 * - `exec`       → runs on the backend via slash.exec / command.dispatch and
 *                  renders its text output inline
 * - `unavailable`→ a known command with genuinely no desktop UI (terminal-only,
 *                  messaging-only, …); shows a reason instead of executing
 */
export type DesktopCommandSurface =
  | { kind: 'action'; action: DesktopActionId }
  | { kind: 'picker'; picker: DesktopPickerId }
  | { kind: 'exec' }
  | { kind: 'unavailable'; reason: DesktopUnavailableReason }

export interface DesktopCommandSpec {
  /** Canonical command, leading slash included (e.g. `/resume`). */
  name: string
  /** Popover/help label; omitted for unavailable commands (never surfaced). */
  description?: string
  aliases?: string[]
  surface: DesktopCommandSurface
  /**
   * Hide from the slash popover / completions while still letting it execute.
   * Used for picker commands reachable from chrome (the model picker lives on
   * the status bar), so the popover doesn't dead-end on inline completion.
   */
  hidden?: boolean
  /**
   * The command has an inline options "screen" (theme / personality / session /
   * platform / toolset list). Picking the bare command in the popover expands to
   * that argument step instead of committing — mirroring typing `/<cmd> ` by hand.
   */
  args?: boolean
}

const exec = (): DesktopCommandSurface => ({ kind: 'exec' })
const action = (id: DesktopActionId): DesktopCommandSurface => ({ kind: 'action', action: id })
const picker = (id: DesktopPickerId): DesktopCommandSurface => ({ kind: 'picker', picker: id })
const unavailable = (reason: DesktopUnavailableReason): DesktopCommandSurface => ({ kind: 'unavailable', reason })

/**
 * THE source of truth for desktop slash commands. Everything below — execution
 * gating, popover suggestions, catalog filtering, pill grouping, and the
 * dispatcher's behavior — derives from this one table.
 */
const DESKTOP_COMMAND_SPECS: readonly DesktopCommandSpec[] = [
  // Local client actions
  { name: '/new', description: 'Начать новый desktop-чат', aliases: ['/reset'], surface: action('new') },
  { name: '/branch', description: 'Ответвить последнее сообщение в новый чат', aliases: ['/fork'], surface: action('branch') },
  { name: '/yolo', description: 'Переключить YOLO — автоодобрение опасных команд', surface: action('yolo') },
  { name: '/handoff', description: 'Передать эту сессию в мессенджер', surface: action('handoff'), args: true },
  { name: '/profile', description: 'Переключить активный профиль Hermes', surface: action('profile') },
  { name: '/skin', description: 'Переключить desktop-тему или перейти к следующей', surface: action('skin'), args: true },
  { name: '/title', description: 'Переименовать текущую сессию', surface: action('title') },
  { name: '/help', description: 'Показать desktop slash-команды', aliases: ['/commands'], surface: action('help') },
  {
    name: '/browser',
    description: 'Управлять browser CDP-подключением [connect|disconnect|status]',
    surface: action('browser'),
    args: true
  },

  // Overlay pickers
  { name: '/model', description: 'Переключить модель для этой сессии', surface: picker('model'), hidden: true },
  {
    name: '/resume',
    description: 'Продолжить сохранённую сессию',
    aliases: ['/sessions', '/switch'],
    surface: picker('session'),
    args: true
  },

  // Backend-executed commands that render useful inline output
  { name: '/agents', description: 'Показать активные desktop-сессии и задачи', aliases: ['/tasks'], surface: exec() },
  { name: '/background', description: 'Запустить запрос в фоне', aliases: ['/bg', '/btw'], surface: exec() },
  { name: '/compress', description: 'Сжать контекст этого диалога', surface: exec() },
  { name: '/debug', description: 'Создать debug-отчёт', surface: exec() },
  { name: '/goal', description: 'Управлять постоянной целью этой сессии', surface: exec() },
  { name: '/personality', description: 'Переключить личность для этой сессии', surface: exec(), args: true },
  { name: '/queue', description: 'Поставить запрос в очередь на следующий ход', aliases: ['/q'], surface: exec() },
  { name: '/retry', description: 'Повторить последнее сообщение пользователя', surface: exec() },
  { name: '/rollback', description: 'Показать или восстановить файловые checkpoints', surface: exec() },
  { name: '/save', description: 'Сохранить текущий transcript в JSON', surface: exec() },
  { name: '/status', description: 'Показать статус текущей сессии', surface: exec() },
  { name: '/steer', description: 'Направить текущий запуск после следующего tool call', surface: exec() },
  { name: '/stop', description: 'Остановить фоновые процессы', surface: exec() },
  { name: '/tools', description: 'Показать или переключить инструменты агента', surface: exec(), args: true },
  { name: '/undo', description: 'Удалить последний обмен пользователь/ассистент', surface: exec() },
  { name: '/usage', description: 'Показать расход токенов этой сессии', surface: exec() },
  { name: '/version', description: 'Показать версию Hermes RU Iola', surface: exec() },

  // No desktop surface, but carry an alias (underscore spelling variants).
  { name: '/reload-mcp', aliases: ['/reload_mcp'], surface: unavailable('advanced') },
  { name: '/reload-skills', aliases: ['/reload_skills'], surface: unavailable('advanced') }
]

// Known commands with no desktop surface (and no alias) — a flat name list
// per reason beats 40 identical object literals.
const NO_DESKTOP_SURFACE: Record<DesktopUnavailableReason, readonly string[]> = {
  terminal: [
    '/busy', '/clear', '/compact', '/config', '/copy', '/cron', '/details',
    '/exit', '/footer', '/gateway', '/gquota', '/history', '/image', '/indicator', '/logs',
    '/mouse', '/paste', '/platforms', '/plugins', '/quit', '/redraw', '/reload', '/restart',
    '/sb', '/set-home', '/sethome', '/snap', '/snapshot', '/statusbar', '/toolsets', '/update', '/verbose'
  ],
  messaging: ['/approve', '/deny'],
  settings: ['/skills'],
  advanced: ['/curator', '/fast', '/insights', '/kanban', '/reasoning', '/voice']
}

const ALL_SPECS: readonly DesktopCommandSpec[] = [
  ...DESKTOP_COMMAND_SPECS,
  ...(Object.entries(NO_DESKTOP_SURFACE) as [DesktopUnavailableReason, readonly string[]][]).flatMap(
    ([reason, names]) => names.map(name => ({ name, surface: unavailable(reason) }))
  )
]

const SPEC_BY_NAME = new Map<string, DesktopCommandSpec>(ALL_SPECS.map(spec => [spec.name, spec]))

const ALIAS_TO_CANONICAL = new Map<string, string>(
  ALL_SPECS.flatMap(spec => (spec.aliases ?? []).map(alias => [alias, spec.name] as const))
)

const UNAVAILABLE_MESSAGE: Record<DesktopUnavailableReason, (command: string) => string> = {
  advanced: command =>
    `${command} не показывается в desktop slash-палитре. Используйте соответствующий desktop-контрол или терминал.`,
  messaging: command => `${command} используется только из мессенджеров.`,
  settings: command => `${command} управляется из desktop sidebar.`,
  terminal: command => `${command} доступна только в терминальном интерфейсе.`
}

const PICKER_UNAVAILABLE_MESSAGE: Record<DesktopPickerId, (command: string) => string> = {
  model: command => `${command} использует desktop picker модели вместо slash-команды.`,
  session: command => `${command} использует desktop picker сессии вместо slash-команды.`
}

function normalizeCommand(command: string): string {
  const trimmed = command.trim()
  const base = (trimmed.startsWith('/') ? trimmed : `/${trimmed}`).split(/\s+/, 1)[0]?.toLowerCase() || ''

  return base
}

export function canonicalDesktopSlashCommand(command: string): string {
  const normalized = normalizeCommand(command)

  return ALIAS_TO_CANONICAL.get(normalized) || normalized
}

/** Resolve a command (or alias) to its desktop spec, or null for unknown/extension commands. */
export function resolveDesktopCommand(command: string): DesktopCommandSpec | null {
  return SPEC_BY_NAME.get(canonicalDesktopSlashCommand(command)) ?? null
}

function isKnownHermesSlashCommand(command: string): boolean {
  const normalized = normalizeCommand(command)

  return SPEC_BY_NAME.has(normalized) || ALIAS_TO_CANONICAL.has(normalized)
}

/**
 * An "extension" command is anything the backend surfaces that is NOT one of
 * Hermes' built-in slash commands — i.e. skill commands (`/gif-search`,
 * `/codex`, …) and user-defined quick commands. These are user-activated, so
 * they appear in the desktop slash palette and execute when typed.
 */
export function isDesktopSlashExtensionCommand(command: string): boolean {
  const normalized = normalizeCommand(command)

  if (!normalized || normalized === '/') {
    return false
  }

  return !isKnownHermesSlashCommand(normalized)
}

/** Gates execution: true unless the command is a known no-desktop-surface command. */
export function isDesktopSlashCommand(command: string): boolean {
  const spec = resolveDesktopCommand(command)

  if (spec) {
    return spec.surface.kind !== 'unavailable'
  }

  return isDesktopSlashExtensionCommand(command)
}

/** Gates discovery in the popover/completions. */
export function isDesktopSlashSuggestion(command: string): boolean {
  const normalized = normalizeCommand(command)

  // Aliases stay hidden so the popover isn't cluttered with duplicates.
  if (ALIAS_TO_CANONICAL.has(normalized)) {
    return false
  }

  const spec = SPEC_BY_NAME.get(normalized)

  if (spec) {
    return spec.surface.kind !== 'unavailable' && !spec.hidden
  }

  // Skill / quick commands the backend provides.
  return isDesktopSlashExtensionCommand(normalized)
}

/**
 * True for commands the desktop fulfils by opening an overlay picker
 * (`/model`, `/resume`/`/sessions`/`/switch`). Optionally pin to one picker.
 */
export function isPickerCommand(command: string, picker?: DesktopPickerId): boolean {
  const surface = resolveDesktopCommand(command)?.surface

  if (surface?.kind !== 'picker') {
    return false
  }

  return picker ? surface.picker === picker : true
}

/** Back-compat shim for the model picker check. */
export function isModelPickerCommand(command: string): boolean {
  return isPickerCommand(command, 'model')
}

export function desktopSlashUnavailableMessage(command: string): string | null {
  const canonical = canonicalDesktopSlashCommand(command)
  const surface = SPEC_BY_NAME.get(canonical)?.surface

  if (!surface) {
    return null
  }

  if (surface.kind === 'unavailable') {
    return UNAVAILABLE_MESSAGE[surface.reason](canonical)
  }

  if (surface.kind === 'picker') {
    return PICKER_UNAVAILABLE_MESSAGE[surface.picker](canonical)
  }

  return null
}

export function desktopSlashDescription(command: string, fallback = ''): string {
  return SPEC_BY_NAME.get(canonicalDesktopSlashCommand(command))?.description || fallback
}

/**
 * True when picking the bare command should expand to its inline argument
 * options (theme / personality / session / platform / toolset) rather than
 * committing immediately. Lets the popover act as a two-step picker.
 */
export function desktopSlashCommandTakesArgs(command: string): boolean {
  return resolveDesktopCommand(command)?.args ?? false
}

export function desktopSkinSlashCompletions(
  themes: DesktopThemeCommandOption[],
  activeThemeName: string,
  argPrefix: string
): DesktopSlashCompletion[] {
  const prefix = argPrefix.trim().toLowerCase()

  const commands: DesktopSlashCompletion[] = [
    {
      text: '/skin list',
      display: '/skin list',
      meta: 'Показать доступные desktop-темы'
    },
    {
      text: '/skin next',
      display: '/skin next',
      meta: 'Перейти к следующей desktop-теме'
    },
    ...themes.map(theme => ({
      text: `/skin ${theme.name}`,
      display: `/skin ${theme.name}`,
      meta: `${theme.label}${theme.name === activeThemeName ? ' (текущая)' : ''} - ${theme.description}`
    }))
  ]

  if (!prefix) {
    return commands
  }

  return commands.filter(item => item.text.slice('/skin '.length).toLowerCase().startsWith(prefix))
}

export function filterDesktopCommandsCatalog(catalog: CommandsCatalogLike): CommandsCatalogLike {
  const categories = catalog.categories
    ?.map(section => ({
      ...section,
      pairs: section.pairs
        .filter(([command]) => isDesktopSlashSuggestion(command))
        .map(([command, description]) => [command, desktopSlashDescription(command, description)] as [string, string])
    }))
    .filter(section => section.pairs.length > 0)

  const pairs = catalog.pairs
    ?.filter(([command]) => isDesktopSlashSuggestion(command))
    .map(([command, description]) => [command, desktopSlashDescription(command, description)] as [string, string])

  // Recount skill commands from the filtered output so /help's footer reflects
  // what the user actually sees. Backend's skill_count includes commands the
  // desktop hides (terminal-only, picker-owned, advanced), producing a footer
  // like "60 skill commands available" while only ~29 appear in the list.
  const filteredCommands = new Set<string>()

  for (const section of categories ?? []) {
    for (const [command] of section.pairs) {
      filteredCommands.add(canonicalDesktopSlashCommand(command))
    }
  }

  for (const [command] of pairs ?? []) {
    filteredCommands.add(canonicalDesktopSlashCommand(command))
  }

  let skillCount = 0

  for (const command of filteredCommands) {
    if (isDesktopSlashExtensionCommand(command)) {
      skillCount += 1
    }
  }

  const hasSkillCount = catalog.skill_count !== undefined || skillCount > 0

  return {
    ...catalog,
    ...(categories ? { categories } : {}),
    ...(pairs ? { pairs } : {}),
    ...(hasSkillCount ? { skill_count: skillCount } : {})
  }
}
