import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { readonly, ref } from 'vue'
import type { DebugLogEntry, DebugSettings } from '@/types/debug'

type BackendLogPayload = Omit<DebugLogEntry, 'id' | 'timestamp'> & {
  timestamp?: string
}

const MAX_LOGS = 600
const MAX_DETAIL_LENGTH = 4000
const DEBUG_ENABLED_STORAGE_KEY = 'ai-markdown.debug.enabled'

const debugEnabled = ref(false)
const debugLogs = ref<DebugLogEntry[]>([])
let unlistenBackendLog: (() => void) | null = null
let unlistenFrontendLog: (() => void) | null = null
let unlistenClearLog: (() => void) | null = null
let persistingDebugLog = false
let consoleCaptureInstalled = false
const originalConsoleMethods: Partial<Record<'debug' | 'error' | 'info' | 'log' | 'warn', (...args: unknown[]) => void>> = {}

function createLogId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function normalizeDetail(detail: unknown) {
  let normalized: string | undefined

  if (typeof detail === 'string') {
    normalized = detail
  } else if (detail == null) {
    normalized = undefined
  } else {
    try {
      normalized = JSON.stringify(detail)
    } catch {
      normalized = String(detail)
    }
  }

  if (!normalized || normalized.length <= MAX_DETAIL_LENGTH) {
    return normalized
  }

  return `${normalized.slice(0, MAX_DETAIL_LENGTH)}\n...日志内容过长，已截断 ${normalized.length - MAX_DETAIL_LENGTH} 字符`
}

function summarizeResult(result: unknown) {
  if (result == null) {
    return undefined
  }

  if (Array.isArray(result)) {
    return { type: 'array', length: result.length }
  }

  if (typeof result === 'string') {
    return { type: 'string', length: result.length, preview: result.slice(0, 160) }
  }

  if (typeof result === 'object') {
    return { type: 'object', keys: Object.keys(result as Record<string, unknown>).slice(0, 20) }
  }

  return { type: typeof result, value: result }
}

function installConsoleCapture() {
  if (consoleCaptureInstalled || typeof window === 'undefined') {
    return
  }

  consoleCaptureInstalled = true
  const methods: Array<'debug' | 'error' | 'info' | 'log' | 'warn'> = ['debug', 'error', 'info', 'log', 'warn']

  methods.forEach((method) => {
    originalConsoleMethods[method] = console[method].bind(console)
    console[method] = (...args: unknown[]) => {
      originalConsoleMethods[method]?.(...args)
      if (persistingDebugLog) {
        return
      }

      pushLog(
        {
          source: 'frontend-console',
          level: method === 'error' ? 'error' : method === 'warn' ? 'warn' : 'info',
          action: `console.${method}`,
          detail: normalizeDetail(args.length === 1 ? args[0] : args),
        },
        true,
      )
    }
  })
}

function uninstallConsoleCapture() {
  if (!consoleCaptureInstalled) {
    return
  }

  ;(['debug', 'error', 'info', 'log', 'warn'] as const).forEach((method) => {
    if (originalConsoleMethods[method]) {
      console[method] = originalConsoleMethods[method] as typeof console[typeof method]
    }
  })
  consoleCaptureInstalled = false
}

function syncEnabledToStorage() {
  if (typeof window === 'undefined') {
    return
  }

  localStorage.setItem(DEBUG_ENABLED_STORAGE_KEY, debugEnabled.value ? '1' : '0')
}

function pushLog(entry: Omit<DebugLogEntry, 'id' | 'timestamp'> & { timestamp?: string }, broadcast = false) {
  if (!debugEnabled.value) {
    return
  }

  const log = {
    id: createLogId(),
    timestamp: entry.timestamp ?? new Date().toISOString(),
    source: entry.source,
    level: entry.level,
    phase: entry.phase,
    action: entry.action,
    detail: normalizeDetail(entry.detail),
  }

  debugLogs.value = [log, ...debugLogs.value].slice(0, MAX_LOGS)
  persistingDebugLog = true
  void tauriInvoke('append_debug_log', { entry: log })
    .catch(() => undefined)
    .finally(() => {
      persistingDebugLog = false
    })

  if (broadcast) {
    void emit('app-frontend-debug-log', log)
  }
}

export function useDebugLogState() {
  return {
    enabled: readonly(debugEnabled),
    logs: readonly(debugLogs),
  }
}

export async function initializeDebugLogging() {
  const settings = await tauriInvoke<DebugSettings>('get_debug_settings').catch(() => ({ enabled: false }))
  debugEnabled.value = settings.enabled
  syncEnabledToStorage()
  installConsoleCapture()

  if (!unlistenBackendLog) {
    unlistenBackendLog = await listen<BackendLogPayload>('app-debug-log', (event) => {
      pushLog({
        source: event.payload.source,
        level: event.payload.level,
        phase: event.payload.phase,
        action: event.payload.action,
        detail: event.payload.detail,
        timestamp: event.payload.timestamp,
      })
    })
  }
}

export async function initializeDebugLogViewer() {
  debugEnabled.value = localStorage.getItem(DEBUG_ENABLED_STORAGE_KEY) === '1'
  debugLogs.value = []

  if (!unlistenBackendLog) {
    unlistenBackendLog = await listen<BackendLogPayload>('app-debug-log', (event) => {
      pushLog({
        source: event.payload.source,
        level: event.payload.level,
        phase: event.payload.phase,
        action: event.payload.action,
        detail: event.payload.detail,
        timestamp: event.payload.timestamp,
      })
    })
  }

  if (!unlistenFrontendLog) {
    unlistenFrontendLog = await listen<DebugLogEntry>('app-frontend-debug-log', (event) => {
      pushLog(event.payload)
    })
  }

  if (!unlistenClearLog) {
    unlistenClearLog = await listen('app-debug-clear', () => {
      debugLogs.value = []
    })
  }
}

export function disposeDebugLogging() {
  if (unlistenBackendLog) {
    unlistenBackendLog()
    unlistenBackendLog = null
  }
  if (unlistenFrontendLog) {
    unlistenFrontendLog()
    unlistenFrontendLog = null
  }
  if (unlistenClearLog) {
    unlistenClearLog()
    unlistenClearLog = null
  }
  uninstallConsoleCapture()
}

export async function getDebugSettings(): Promise<DebugSettings> {
  const settings = await tauriInvoke<DebugSettings>('get_debug_settings')
  debugEnabled.value = settings.enabled
  syncEnabledToStorage()
  return settings
}

export async function updateDebugSettings(enabled: boolean): Promise<DebugSettings> {
  const settings = await tauriInvoke<DebugSettings>('update_debug_settings', { enabled })
  debugEnabled.value = settings.enabled
  syncEnabledToStorage()

  if (!settings.enabled) {
    return settings
  }

  pushLog({
    source: 'frontend',
    level: 'info',
    action: 'debug.mode.enabled',
    detail: '调试模式已开启',
  })
  return settings
}

export function clearDebugLogs() {
  debugLogs.value = []
  void emit('app-debug-clear')
}

export function logFrontendAction(action: string, detail?: unknown, level: DebugLogEntry['level'] = 'info') {
  pushLog({
    source: 'frontend',
    level,
    action,
    detail: normalizeDetail(detail),
  }, true)
}

export async function loggedInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
  logArgs: Record<string, unknown> | undefined = args,
): Promise<T> {
  pushLog(
    {
      source: 'request',
      phase: 'send',
      level: 'info',
      action: command,
      detail: normalizeDetail(logArgs),
    },
    true,
  )

  try {
    const result = await tauriInvoke<T>(command, args)
    pushLog(
      {
        source: 'request',
        phase: 'receive',
        level: 'success',
        action: command,
        detail: normalizeDetail(summarizeResult(result)),
      },
      true,
    )
    return result
  } catch (error) {
    pushLog(
      {
        source: 'request',
        phase: 'receive',
        level: 'error',
        action: command,
        detail: error instanceof Error ? error.message : String(error),
      },
      true,
    )
    throw error
  }
}
