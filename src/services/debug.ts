import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { readonly, ref } from 'vue'
import type { DebugLogEntry, DebugSettings } from '@/types/debug'

type BackendLogPayload = Omit<DebugLogEntry, 'id' | 'timestamp'> & {
  timestamp?: string
}

const MAX_LOGS = 600
const DEBUG_ENABLED_STORAGE_KEY = 'ai-markdown.debug.enabled'

const debugEnabled = ref(false)
const debugLogs = ref<DebugLogEntry[]>([])
let unlistenBackendLog: (() => void) | null = null
let unlistenFrontendLog: (() => void) | null = null
let unlistenClearLog: (() => void) | null = null

function createLogId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function normalizeDetail(detail: unknown) {
  if (typeof detail === 'string') {
    return detail
  }

  if (detail == null) {
    return undefined
  }

  try {
    return JSON.stringify(detail)
  } catch {
    return String(detail)
  }
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
    action: entry.action,
    detail: entry.detail,
  }

  debugLogs.value = [log, ...debugLogs.value].slice(0, MAX_LOGS)
  void tauriInvoke('append_debug_log', { entry: log }).catch(() => undefined)

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

  if (!unlistenBackendLog) {
    unlistenBackendLog = await listen<BackendLogPayload>('app-debug-log', (event) => {
      pushLog({
        source: event.payload.source,
        level: event.payload.level,
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
  logFrontendAction(`invoke.${command}.request`, logArgs)

  try {
    const result = await tauriInvoke<T>(command, args)
    pushLog({
      source: 'backend',
      level: 'success',
      action: `invoke.${command}.success`,
    })
    return result
  } catch (error) {
    pushLog({
      source: 'backend',
      level: 'error',
      action: `invoke.${command}.error`,
      detail: error instanceof Error ? error.message : String(error),
    })
    throw error
  }
}
