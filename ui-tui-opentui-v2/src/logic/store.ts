/**
 * Session/message store — the SOLID side (spec v4 §1, §7). Plain `createStore`
 * + an `apply(event)` reducer, à la opencode `context/sync-v2.tsx`. NOT Effect.
 * The boundary calls `apply` with already-decoded `GatewayEvent`s via
 * GatewayService.subscribe.
 *
 * Phase 2b: an assistant turn is ONE ordered `parts[]` of a discriminated union
 * (text / reasoning / tool), so tool calls render INLINE between text blocks
 * instead of dumped as separate rows below (§7 — the "dump-below" bug). Tools are
 * matched start↔complete by `tool_id`; `tool.complete` updates that part IN PLACE.
 * User/system rows stay flat `text` (no parts). Carried from Phase 1: streaming
 * concat (prefer `payload.text`), skin→theme, LRU dedup, hydrate-while-buffering.
 */
import { Option } from 'effect'
import { createStore, produce } from 'solid-js/store'

import type { GatewayEvent, GatewaySkinDecoded } from '../boundary/schema/GatewayEvent.ts'
import {
  decodeCatalog,
  decodeSessionInfoPatch,
  type CatalogDecoded,
  type SessionInfoPatchDecoded
} from '../boundary/schema/SessionInfo.ts'
import { stripAnsi, stripOmittedNote, stripToolEnvelope } from './toolOutput.ts'
import { DEFAULT_THEME, type Theme, themeFromSkin } from './theme.ts'

/** A tool call inside an assistant turn (matched start↔complete by `id`=tool_id). */
export interface ToolPartState {
  type: 'tool'
  id: string
  name: string
  state: 'running' | 'complete'
  /** Envelope-stripped output (multi-line → block render; the view caps it). */
  resultText?: string
  /** Short one-line status when there's no substantial output. */
  summary?: string
  error?: string
  lineCount?: number
  /** One-line primary-arg preview from gateway `context` (always sent; redaction-safe). */
  argsPreview?: string
  /** Full args (pretty JSON) for the expanded view — `args_text` (redacted) or stringified `args`. */
  argsText?: string
  /** Tool wall-clock seconds (gateway `duration_s`), shown dim in the header. */
  duration?: number
  /** Tidy note when the gateway truncated output (e.g. "5 lines / 234 chars"). */
  omittedNote?: string
}

/** One ordered piece of an assistant turn (§7). */
export type Part =
  | { type: 'text'; id: string; text: string }
  | { type: 'reasoning'; id: string; text: string }
  | ToolPartState

export interface Message {
  readonly role: 'user' | 'assistant' | 'system'
  /** Flat body for user/system rows (and settled/resumed assistant rows). */
  text: string
  /** Ordered parts for a live assistant turn; absent for user/system. */
  parts?: Part[]
  streaming?: boolean
}

/**
 * A BLOCKING interactive request from the agent (spec §8 #6 — unhandled = deadlock).
 * Each is answered via the matching `*.respond` RPC; Esc/Ctrl+C sends deny/empty.
 */
export type ActivePrompt =
  | { kind: 'clarify'; question: string; choices: string[] | null; requestId: string }
  | { kind: 'approval'; command: string; description: string }
  | { kind: 'sudo'; requestId: string }
  | { kind: 'secret'; envVar: string; prompt: string; requestId: string }
  // local (non-gateway) Y/N confirm — e.g. /clear, /new (spec §2a)
  | { kind: 'confirm'; message: string; onConfirm: () => void }

/** A full-screen scrollable text viewer (long slash output: /status, /logs, …). */
export interface PagerState {
  title: string
  text: string
}

/** One row in the session switcher (from `session.list`). */
export interface SessionItem {
  id: string
  title: string
  preview: string
  messageCount: number
}

/** A row in a generic `<select>` picker (model picker, skills hub, …). */
export interface PickerItem {
  label: string
  description?: string
  value: string
}

/** An open generic picker overlay: a titled list whose pick runs `onPick(value)`. */
export interface PickerState {
  title: string
  items: PickerItem[]
  onPick: (value: string) => void
}

/** A slash-completion candidate (from `complete.slash`). */
export interface CompletionItem {
  text: string
  display: string
  meta: string
}

/** A delegated subagent, tracked from the `subagent.*` event stream (agents dashboard). */
export interface SubagentInfo {
  id: string
  goal: string
  status: string
  depth: number
  model?: string
  parentId?: string
  summary?: string
  lastTool?: string
  /** Live activity trace (item 15) — tool/progress/summary lines, newest last. */
  trace?: string[]
  /** Latest thinking text (transient; not appended to the trace to avoid flooding). */
  thought?: string
}

/** Cap on a subagent's retained trace lines. */
const SUBAGENT_TRACE_LIMIT = 200

/**
 * Live session chrome (the status bar — item 14). Sourced from the `session.info`
 * event (and the `session.create`/`resume` result's `info`), refreshed whenever
 * the gateway's agent/config state changes. `running` is the turn-active flag the
 * Ctrl-C interrupt (item 11) reads; we also flip it locally on message.start/
 * complete so the bar reacts instantly even if a `session.info` lags.
 */
export interface SessionInfo {
  model?: string
  effort?: string
  fast?: boolean
  cwd?: string
  branch?: string
  running?: boolean
  contextUsed?: number
  contextMax?: number
  contextPercent?: number
  compressions?: number
}

/** Startup catalog (tools/skills/MCP) for the home-screen panel (item 9 / banner parity). */
export interface Catalog {
  readonly tools: {
    readonly total: number
    readonly toolsets: ReadonlyArray<{ name: string; count: number; enabled: boolean; tools: ReadonlyArray<string> }>
  }
  readonly skills: { readonly total: number; readonly categories: ReadonlyArray<{ name: string; count: number }> }
  readonly mcp: { readonly servers: ReadonlyArray<string> }
}

export interface StoreState {
  ready: boolean
  messages: Message[]
  /** Count of oldest messages trimmed from the DISPLAY by the rolling cap (live
   *  overflow + resume slice). Drives the "N earlier messages" truncation notice;
   *  0 when nothing's been dropped. NOT context loss — the model's history lives on
   *  the gateway (see MESSAGE_CAP); this only bounds in-TUI scrollback. */
  dropped: number
  theme: Theme
  /** The active blocking prompt (composer is hidden while set); undefined when none. */
  prompt: ActivePrompt | undefined
  /** The open pager overlay (replaces the transcript while set); undefined when none. */
  pager: PagerState | undefined
  /** The open session switcher (replaces the composer while set); undefined when none. */
  switcher: SessionItem[] | undefined
  /** The open generic picker (model/skills/…); undefined when none. */
  picker: PickerState | undefined
  /** Live completion candidates (slash-name/args or file/@-mention) shown above the composer. */
  completions: CompletionItem[] | undefined
  /** Char offset in the input where an accepted completion should start replacing
   *  (gateway `replace_from` for slash args; the path-token start for @-mentions). */
  completionFrom: number
  /** Delegated subagents (from `subagent.*`), shown in the agents dashboard. */
  subagents: SubagentInfo[]
  /** Whether the agents dashboard overlay is open (/agents). */
  dashboard: boolean
  /** Transient busy indicator (the kaomoji face/verb from `thinking.delta`/`status.update`);
   *  shown above the composer WHILE a turn runs, cleared on `message.complete`. NOT transcript. */
  status: string | undefined
  /** Live session chrome for the status bar (model/effort/cwd/branch/context/running). */
  info: SessionInfo
  /** Transient hint shown above the composer (e.g. "Ctrl+C again to quit" — item 11);
   *  takes visual priority over the busy `status` face. Undefined when none. */
  hint: string | undefined
  /** Startup tools/skills/MCP catalog (from `startup.catalog`) for the home panel (item 9). */
  catalog: Catalog | undefined
  /** The current session id (shown in the home panel; updated on create/resume). */
  sessionId: string | undefined
}

const LRU_LIMIT = 1000

/** Read a string field off an unknown payload record (no `any`, no cast). */
function readStr(payload: { readonly [k: string]: unknown }, key: string): string | undefined {
  const v = payload[key]
  return typeof v === 'string' ? v : undefined
}

/** Read a number field off an unknown payload record. */
function readNum(payload: { readonly [k: string]: unknown }, key: string): number {
  const v = payload[key]
  return typeof v === 'number' ? v : 0
}

/** Read an optional number (undefined when absent) — distinguishes "0" from "missing". */
function readOptNum(payload: { readonly [k: string]: unknown }, key: string): number | undefined {
  const v = payload[key]
  return typeof v === 'number' ? v : undefined
}

/**
 * Fold a `session.info` / `session.create.info` payload into a partial SessionInfo.
 * The loose wire JSON is decoded ONCE via `SessionInfoPatchSchema` (decode-at-
 * boundary); context/usage numbers are read from the nested `usage` object first,
 * falling back to the top level (the gateway shapes vary by RPC vs event). A
 * malformed payload decodes to `Option.none` → an empty patch (never crashes).
 * Only present fields are included so a partial patch can't clobber prior chrome.
 */
function readInfoPatch(payload: { readonly [k: string]: unknown }): Partial<SessionInfo> {
  const decoded = decodeSessionInfoPatch(payload)
  if (Option.isNone(decoded)) return {}
  return infoPatchFrom(decoded.value)
}

/** Build the SessionInfo patch from a decoded session.info payload. */
function infoPatchFrom(d: SessionInfoPatchDecoded): Partial<SessionInfo> {
  const patch: Partial<SessionInfo> = {}
  if (d.model) patch.model = d.model
  if (d.reasoning_effort) patch.effort = d.reasoning_effort
  if (d.fast !== undefined) patch.fast = d.fast
  if (d.cwd) patch.cwd = d.cwd
  if (d.branch) patch.branch = d.branch
  if (d.running !== undefined) patch.running = d.running
  // prefer the nested usage.context_* numbers, else the top-level fallback.
  const used = d.usage?.context_used ?? d.context_used
  if (used !== undefined) patch.contextUsed = used
  const max = d.usage?.context_max ?? d.context_max
  if (max !== undefined) patch.contextMax = max
  const pct = d.usage?.context_percent ?? d.context_percent
  if (pct !== undefined) patch.contextPercent = pct
  const comp = d.usage?.compressions ?? d.compressions
  if (comp !== undefined) patch.compressions = comp
  return patch
}

/** Keep only the string elements of a decoded (unknown-element) array. */
function onlyStrings(items: ReadonlyArray<unknown> | undefined): string[] {
  return (items ?? []).filter((s): s is string => typeof s === 'string')
}

/** Build the typed Catalog from a decoded startup.catalog result (item 9). An
 *  absent `enabled` flag means on; nameless toolsets/categories are dropped and
 *  non-string tool/server names are filtered (defensive — wire arrays are loose). */
function catalogFrom(d: CatalogDecoded): Catalog {
  return {
    mcp: { servers: onlyStrings(d.mcp?.servers) },
    skills: {
      total: d.skills?.total ?? 0,
      categories: (d.skills?.categories ?? [])
        .map(c => ({ count: c.count ?? 0, name: c.name ?? '' }))
        .filter(c => c.name)
    },
    tools: {
      total: d.tools?.total ?? 0,
      toolsets: (d.tools?.toolsets ?? [])
        .map(t => ({
          count: t.count ?? 0,
          enabled: t.enabled !== false,
          name: t.name ?? '',
          tools: onlyStrings(t.tools)
        }))
        .filter(t => t.name)
    }
  }
}

/** The subagent status implied by an event type (an explicit payload `status` wins). */
function subagentStatusFor(type: string): string {
  if (type === 'subagent.complete') return 'complete'
  if (type === 'subagent.thinking') return 'thinking'
  if (type === 'subagent.tool') return 'tool'
  if (type === 'subagent.progress') return 'working'
  return 'running'
}

export function createSessionStore() {
  // Rolling cap on retained transcript rows. OpenTUI lays out via Yoga (WASM), whose
  // linear memory is grow-only — every live `<For>` row is a Yoga-node subtree, so an
  // uncapped `messages[]` ratchets the high-water mark up over a long session and never
  // gives it back. Capping the array in place (see `capMessages`) makes Solid's keyed
  // `<For>` UNMOUNT exactly the evicted oldest rows → `Renderable.destroy()` →
  // `yogaNode.free()`, returning those nodes to the WASM allocator's free list.
  //
  // Default 3000 (≈1500 turns of scrollback): the highest cap whose steady-state RSS
  // stays within a sane TUI budget on the realistic-fixture bench (~20.4 renderables/
  // msg, ~0.65 MB/msg → ~2 GB at 3000 — and that ceiling is only reached by marathon
  // 3000+-message sessions; typical sessions cost a fraction). opencode caps at 100;
  // we trade memory for far more in-TUI scrollback (the dashboard holds the rest).
  // Read once per store from `HERMES_TUI_MAX_MESSAGES`. Turns trimmed beyond the cap
  // aren't lost — they live on the gateway and are recoverable via `/resume`.
  const MESSAGE_CAP = (() => {
    const raw = Number.parseInt(process.env.HERMES_TUI_MAX_MESSAGES ?? '', 10)
    return Number.isFinite(raw) && raw > 0 ? raw : 3000
  })()

  const [state, setState] = createStore<StoreState>({
    ready: false,
    messages: [],
    dropped: 0,
    theme: DEFAULT_THEME,
    prompt: undefined,
    pager: undefined,
    switcher: undefined,
    picker: undefined,
    completions: undefined,
    completionFrom: 0,
    subagents: [],
    dashboard: false,
    status: undefined,
    info: {},
    hint: undefined,
    catalog: undefined,
    sessionId: undefined
  })

  // Monotonic part id (stable `key` per part so a new tool part below a streaming
  // text part doesn't remount/re-tokenize it).
  let partSeq = 0
  const nextId = () => `p${++partSeq}`

  // LRU id-dedup: events that carry a stable id are applied at most once.
  const applied = new Set<string>()
  function duplicate(id: string | undefined): boolean {
    if (!id) return false
    if (applied.has(id)) return true
    applied.add(id)
    if (applied.size > LRU_LIMIT) {
      const oldest = applied.values().next()
      if (!oldest.done) applied.delete(oldest.value)
    }
    return false
  }

  // Hydrate-while-buffering (resume): while a snapshot is loading, live events
  // queue here and replay after the snapshot is reconciled (opencode sync-v2).
  let buffering: GatewayEvent[] | null = null

  // Anti-flood for `gateway.stderr`: a crashing child can emit a torrent of
  // stderr lines, so we do NOT push each to the transcript. Instead we keep a
  // small ring of the most-recent lines and only surface a TAIL of it when a
  // failure event (start_timeout / exited) actually needs the diagnostic
  // context — so a healthy-but-chatty gateway never spams the chat.
  const STDERR_RING_LIMIT = 20
  const STDERR_TAIL = 5
  const stderrRing: string[] = []
  function stderrTail(): string {
    return stderrRing.slice(-STDERR_TAIL).join('\n')
  }

  function setSkin(skin: GatewaySkinDecoded | undefined): void {
    setState('theme', themeFromSkin(skin))
  }

  // Trim the transcript to MESSAGE_CAP, dropping the OLDEST rows IN PLACE via
  // `splice` (NOT a `slice`-reassign). A keyed `<For>` keeps rows by item
  // REFERENCE, so splicing the head unmounts only the evicted rows (freeing their
  // Yoga nodes) while the survivors keep their refs and are not remounted. A live
  // streaming assistant turn is always the LAST row, so head-trimming never drops it.
  function capMessages(draft: StoreState): void {
    const overflow = draft.messages.length - MESSAGE_CAP
    if (overflow > 0) {
      draft.messages.splice(0, overflow)
      draft.dropped += overflow
    }
  }

  // ── parts helpers (operate on a draft message inside produce) ───────────
  function appendPart(m: Message, type: 'text' | 'reasoning', text: string): void {
    const parts = (m.parts ??= [])
    const last = parts[parts.length - 1]
    if (last && last.type === type) last.text += text
    else parts.push({ type, id: nextId(), text })
  }

  /** The live (last) assistant message, optionally only when still streaming. */
  function liveAssistant(draft: StoreState, streamingOnly = false): Message | undefined {
    const last = draft.messages[draft.messages.length - 1]
    if (last && last.role === 'assistant' && (!streamingOnly || last.streaming)) return last
    return undefined
  }

  /** Ensure there's an open assistant turn to attach parts to (tool/reasoning). */
  function ensureAssistant(draft: StoreState): Message {
    const live = liveAssistant(draft, true)
    if (live) return live
    const created: Message = { role: 'assistant', text: '', parts: [], streaming: true }
    draft.messages.push(created)
    return created
  }

  /** Find a tool part by id in the CURRENT (last) assistant turn — a tool.complete
   *  always pairs with a tool.start in the live turn, so scoping there avoids
   *  matching a same-id tool in an older/resumed turn (and is O(parts), not O(all)). */
  function findToolPart(draft: StoreState, id: string): ToolPartState | undefined {
    const parts = liveAssistant(draft)?.parts
    if (!parts) return undefined
    for (let j = parts.length - 1; j >= 0; j--) {
      const p = parts[j]
      if (p && p.type === 'tool' && p.id === id) return p
    }
    return undefined
  }

  /** Push a user message (composer submit). */
  function pushUser(text: string) {
    setState(
      produce(draft => {
        draft.messages.push({ role: 'user', text })
        capMessages(draft)
      })
    )
  }

  /** Push a system line (slash output, errors, notices). */
  function pushSystem(text: string) {
    // slash/notice text is often ANSI-colored for the Ink TUI; strip codes so
    // they don't render as literal `[1;38m…` glyphs in the native engine (item 8).
    const clean = stripAnsi(text)
    setState(
      produce(draft => {
        draft.messages.push({ role: 'system', text: clean })
        capMessages(draft)
      })
    )
  }

  /** Clear the transcript (e.g. /clear, /new) and any tracked subagents. */
  function clearTranscript() {
    setState('messages', [])
    setState('subagents', [])
    setState('dropped', 0)
    // Drop the dedup history too — a fresh transcript should re-process any id.
    applied.clear()
  }

  /** Open / close the agents dashboard overlay (/agents). */
  function openDashboard() {
    setState('dashboard', true)
  }
  function closeDashboard() {
    setState('dashboard', false)
  }

  /** Open a local Y/N confirm dialog (non-gateway; e.g. /clear). */
  function setConfirm(message: string, onConfirm: () => void) {
    setState('prompt', { kind: 'confirm', message, onConfirm })
  }

  /** Open the pager overlay (long slash output: /status, /logs, …). */
  function openPager(title: string, text: string) {
    setState('pager', { title, text: stripAnsi(text) })
  }

  /** Close the pager overlay. */
  function closePager() {
    setState('pager', undefined)
  }

  /** Open the session switcher with the given session rows (/sessions, /resume). */
  function openSwitcher(sessions: SessionItem[]) {
    setState('switcher', sessions)
  }

  /** Close the session switcher. */
  function closeSwitcher() {
    setState('switcher', undefined)
  }

  /** Open the generic picker (model picker, skills hub, …). */
  function openPicker(picker: PickerState) {
    setState('picker', picker)
  }

  /** Close the generic picker. */
  function closePicker() {
    setState('picker', undefined)
  }

  /** Set / clear the transient composer hint ("Ctrl+C again to quit" — item 11). */
  function setHint(text: string | undefined): void {
    setState('hint', text)
  }

  /** Merge a session-info patch into the chrome state (status bar — item 14). */
  function applyInfo(raw: { readonly [k: string]: unknown }): void {
    const patch = readInfoPatch(raw)
    if (Object.keys(patch).length) setState('info', prev => ({ ...prev, ...patch }))
  }

  /** Set / clear the live completion candidates (composer dropdown). `from` is the
   *  input char offset an accepted item replaces from (slash-arg / @-mention splice). */
  function setCompletions(items: CompletionItem[], from = 0) {
    setState('completions', items.length ? items : undefined)
    setState('completionFrom', items.length ? Math.max(0, from) : 0)
  }
  function clearCompletions() {
    setState('completions', undefined)
    setState('completionFrom', 0)
  }

  /** Reduce a decoded gateway event into the store. The sole boundary->Solid sink. */
  function apply(event: GatewayEvent): void {
    if (buffering) {
      buffering.push(event)
      return
    }
    applyNow(event)
  }

  function applyNow(event: GatewayEvent): void {
    switch (event.type) {
      case 'gateway.ready':
        setState('ready', true)
        // Clear any transient status: on a recovery-respawn ready this drops the
        // lingering 'gateway recovering (attempt N)…' line; no-op on first connect.
        setState('status', undefined)
        setSkin(event.payload?.skin)
        break
      case 'skin.changed':
        setSkin(event.payload)
        break
      case 'session.info':
        applyInfo(event.payload)
        break
      case 'message.start':
        setState('status', undefined)
        setState('info', prev => ({ ...prev, running: true }))
        setState(
          produce(draft => {
            draft.messages.push({ role: 'assistant', text: '', parts: [], streaming: true })
            capMessages(draft)
          })
        )
        break
      case 'message.delta': {
        // prefer `text` over `rendered` (gotcha §8 #4 — rendered is incremental Rich-ANSI).
        const text = event.payload?.text ?? ''
        if (!text) break
        setState(
          produce(draft => {
            const live = liveAssistant(draft, true)
            if (live) appendPart(live, 'text', text)
          })
        )
        break
      }
      case 'message.complete':
        setState(
          produce(draft => {
            // complete-only gateways may send `message.complete{text}` with no prior
            // start/delta → create the turn so the final text isn't dropped.
            const finalText = event.payload?.text
            const live = liveAssistant(draft, true) ?? (finalText ? ensureAssistant(draft) : undefined)
            if (!live) return
            // If no deltas arrived (complete-only gateways), seed the full text once.
            const hasText = (live.parts ?? []).some(p => p.type === 'text' && p.text.length > 0)
            if (finalText && !hasText) appendPart(live, 'text', finalText)
            live.streaming = false
          })
        )
        setState('status', undefined)
        setState('info', prev => ({ ...prev, running: false }))
        // message.complete carries the latest usage/context — refresh the bar.
        if (event.payload) applyInfo(event.payload)
        break
      // thinking.delta / status.update are the TRANSIENT busy indicator (kaomoji
      // face/verb) — route them to the status line, NOT the transcript (gotcha: Ink
      // shows these as a FaceTicker, not message content).
      case 'thinking.delta':
      case 'status.update': {
        const text = event.payload?.text ?? ''
        if (text) setState('status', text)
        break
      }
      // reasoning.delta is the model's actual reasoning — a (dim) transcript part.
      case 'reasoning.delta': {
        const text = event.payload?.text ?? ''
        if (!text) break
        setState(
          produce(draft => {
            appendPart(ensureAssistant(draft), 'reasoning', text)
          })
        )
        break
      }
      case 'tool.start': {
        const id = readStr(event.payload, 'tool_id')
        if (!id) break
        const name = readStr(event.payload, 'name') ?? 'tool'
        // `context` = build_tool_preview's primary-arg line (always sent); `args_text`
        // = redacted full-arg JSON (verbose mode only). Surfacing these is item 2.
        const argsPreview = readStr(event.payload, 'context')
        const argsText = readStr(event.payload, 'args_text')
        setState(
          produce(draft => {
            const live = ensureAssistant(draft)
            const part: ToolPartState = { type: 'tool', id, name, state: 'running' }
            if (argsPreview) part.argsPreview = argsPreview
            if (argsText) part.argsText = argsText
            ;(live.parts ??= []).push(part)
          })
        )
        break
      }
      case 'tool.complete': {
        const id = readStr(event.payload, 'tool_id')
        if (!id) break
        const name = readStr(event.payload, 'name')
        const error = readStr(event.payload, 'error')
        const summary = readStr(event.payload, 'summary')
        // Peel the gateway's "[showing verbose tail; omitted …]" label (item 2) before
        // envelope-stripping, so the body is clean and the note renders tidily.
        const { body: rawBody, omittedNote } = stripOmittedNote(readStr(event.payload, 'result_text') ?? summary ?? '')
        const resultText = stripToolEnvelope(rawBody)
        const lineCount = resultText ? resultText.replace(/\s+$/, '').split('\n').length : 0
        // `args` (full dict) is always sent; stringify as the expanded-view args
        // when verbose `args_text` wasn't captured on start. `duration_s` → header.
        const argsObj = event.payload['args']
        const duration = readOptNum(event.payload, 'duration_s')
        setState(
          produce(draft => {
            let part = findToolPart(draft, id)
            if (!part) {
              // complete without a matching start — append a settled tool part.
              part = { type: 'tool', id, name: name ?? 'tool', state: 'running' }
              ;(ensureAssistant(draft).parts ??= []).push(part)
            }
            part.state = 'complete'
            part.lineCount = lineCount
            if (name) part.name = name
            if (resultText) part.resultText = resultText
            if (summary) part.summary = summary
            if (error) part.error = error
            if (duration !== undefined) part.duration = duration
            if (omittedNote) part.omittedNote = omittedNote
            // argsPreview (from tool.start `context`) is intentionally NOT overwritten.
            if (!part.argsText && argsObj && typeof argsObj === 'object') {
              try {
                part.argsText = JSON.stringify(argsObj, null, 2)
              } catch {
                /* unstringifiable args — leave unset */
              }
            }
          })
        )
        break
      }
      // ── blocking prompts (spec §8 #6 — unhandled = the agent deadlocks) ──
      case 'clarify.request':
        setState('prompt', {
          kind: 'clarify',
          question: event.payload.question ?? '',
          // decoded choices are readonly — copy to the store's mutable string[]
          choices: event.payload.choices ? [...event.payload.choices] : null,
          requestId: event.payload.request_id
        })
        break
      case 'approval.request':
        setState('prompt', { kind: 'approval', command: event.payload.command, description: event.payload.description })
        break
      case 'sudo.request':
        setState('prompt', { kind: 'sudo', requestId: event.payload.request_id })
        break
      case 'secret.request':
        setState('prompt', {
          kind: 'secret',
          envVar: event.payload.env_var,
          prompt: event.payload.prompt,
          requestId: event.payload.request_id
        })
        break
      // ── subagents (agents dashboard) — track the delegation tree by id ──
      case 'subagent.spawn_requested':
      case 'subagent.start':
      case 'subagent.thinking':
      case 'subagent.tool':
      case 'subagent.progress':
      case 'subagent.complete': {
        const id = readStr(event.payload, 'subagent_id')
        if (!id) break
        setState(
          produce(draft => {
            let sa = draft.subagents.find(s => s.id === id)
            if (!sa) {
              sa = { depth: readNum(event.payload, 'depth'), goal: '', id, status: 'running' }
              draft.subagents.push(sa)
            }
            const goal = readStr(event.payload, 'goal')
            if (goal) sa.goal = goal
            const model = readStr(event.payload, 'model')
            if (model) sa.model = model
            const parent = readStr(event.payload, 'parent_id')
            if (parent) sa.parentId = parent
            const summary = readStr(event.payload, 'summary')
            if (summary) sa.summary = summary
            const tool = readStr(event.payload, 'tool_name')
            if (tool) sa.lastTool = tool
            sa.status = readStr(event.payload, 'status') ?? subagentStatusFor(event.type)

            // Live trace (item 15): a concise per-subagent activity log. Thinking
            // deltas update a transient `thought` (not appended — they'd flood).
            const text = readStr(event.payload, 'text')
            const trace = (sa.trace ??= [])
            if (event.type === 'subagent.start') trace.push(`▶ ${goal ?? sa.goal ?? 'started'}`)
            else if (event.type === 'subagent.tool' && tool) trace.push(`⚡ ${tool}${text ? ` — ${text}` : ''}`)
            else if (event.type === 'subagent.progress' && text) trace.push(text)
            else if (event.type === 'subagent.complete') trace.push(`✓ ${summary ?? 'done'}`)
            else if (event.type === 'subagent.thinking' && text) sa.thought = text
            if (trace.length > SUBAGENT_TRACE_LIMIT) trace.splice(0, trace.length - SUBAGENT_TRACE_LIMIT)
          })
        )
        break
      }
      // ── gateway lifecycle / transport errors (auto-heal foundations) ──
      // The child exited mid-turn. THE key bug fix: clear the frozen `running`
      // spinner (no message.complete will ever arrive for the lost reply), tell
      // the user their in-flight reply was lost, and show a recovering status.
      case 'gateway.exited': {
        setState('info', prev => ({ ...prev, running: false }))
        // Neutral status: we don't ALWAYS recover (budget exhaustion). The
        // "recovering…" wording now comes from the gateway.recovering case,
        // which fires only when a respawn is actually scheduled.
        setState('status', 'gateway exited')
        const reason = event.payload?.reason
        const base = 'gateway exited — recovering your session (any in-flight reply was lost)'
        pushSystem(reason ? `${base}: ${reason}` : base)
        break
      }
      // A respawn+resume attempt is in flight — reflect the attempt in the status.
      case 'gateway.recovering': {
        const attempt = event.payload?.attempt
        setState('status', attempt ? `gateway recovering (attempt ${attempt})…` : 'gateway recovering…')
        break
      }
      // Collect stderr into a bounded ring (NOT the transcript) — see stderrRing.
      case 'gateway.stderr': {
        stderrRing.push(event.payload.line)
        if (stderrRing.length > STDERR_RING_LIMIT) stderrRing.splice(0, stderrRing.length - STDERR_RING_LIMIT)
        break
      }
      // The gateway never reached `gateway.ready` — surface the failure with any
      // stderr tail (payload is a loose Record; read defensively).
      case 'gateway.start_timeout': {
        const detail = readStr(event.payload, 'stderr') ?? readStr(event.payload, 'message') ?? stderrTail()
        pushSystem(detail ? `gateway failed to start:\n${detail}` : 'gateway failed to start')
        break
      }
      case 'gateway.protocol_error': {
        const preview = event.payload?.preview
        pushSystem(preview ? `gateway protocol error: ${preview}` : 'gateway protocol error')
        break
      }
      case 'error': {
        const message = event.payload?.message
        pushSystem(message ? `error: ${message}` : 'error')
        break
      }
      // Other event types (chrome) are reduced in later phases; unhandled members
      // are intentionally ignored here.
    }
  }

  /** Clear the active blocking prompt (after it's answered/cancelled). */
  function clearPrompt(): void {
    setState('prompt', undefined)
  }

  // ── resume hydrate (opencode sync-v2): buffer live events while the snapshot
  // loads, then replace history + replay the buffer in order. Split into begin/
  // commit so the buffer can span an async `session.resume` RPC.
  /** Start buffering live events (call BEFORE the async resume RPC). Idempotent. */
  function beginBuffer(): void {
    if (!buffering) buffering = []
  }

  /** Replace history with the resume snapshot, then replay events buffered meanwhile. */
  function commitSnapshot(snapshot: Message[]): void {
    // Slice to the cap BEFORE the first setState, not after. Yoga (WASM) layout
    // memory is grow-only, so even a TRANSIENT mount of an over-cap resume
    // snapshot would permanently ratchet the high-water mark — a set-then-trim
    // briefly hands the full fetched history to <For>. Pre-slicing guarantees
    // resuming ANY session mounts at most MESSAGE_CAP rows. (Events buffered
    // across the resume RPC, replayed below, self-cap via capMessages per push.)
    const capped = snapshot.length > MESSAGE_CAP ? snapshot.slice(-MESSAGE_CAP) : snapshot
    setState('messages', capped)
    // A resume is a fresh view → SET (not accumulate) the dropped count to what the
    // snapshot slice hid, so the notice reflects this session. Live pushes add to it.
    setState('dropped', snapshot.length - capped.length)
    const pending = buffering ?? []
    buffering = null
    for (const event of pending) applyNow(event)
  }

  /** Synchronous convenience: buffer → load → commit (used by tests). */
  function hydrate(loadSnapshot: () => Message[]): void {
    beginBuffer()
    commitSnapshot(loadSnapshot())
  }

  /**
   * Map the loose `startup.catalog` response into the typed Catalog (item 9).
   * Decoded ONCE via `CatalogSchema` (decode-at-boundary); garbage decodes to
   * `Option.none` → the catalog is left unset rather than crashing the panel.
   */
  function setCatalog(raw: unknown): void {
    const decoded = decodeCatalog(raw)
    if (Option.isNone(decoded)) return
    setState('catalog', catalogFrom(decoded.value))
  }

  function setSessionId(sid: string | undefined): void {
    setState('sessionId', sid)
  }

  return {
    state,
    apply,
    pushUser,
    pushSystem,
    setCatalog,
    setSessionId,
    clearTranscript,
    setConfirm,
    openPager,
    closePager,
    openSwitcher,
    closeSwitcher,
    openPicker,
    closePicker,
    setCompletions,
    clearCompletions,
    applyInfo,
    setHint,
    openDashboard,
    closeDashboard,
    hydrate,
    beginBuffer,
    commitSnapshot,
    duplicate,
    clearPrompt
  } as const
}

export type SessionStore = ReturnType<typeof createSessionStore>
