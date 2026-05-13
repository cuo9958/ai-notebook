export type DebugLogSource = 'frontend' | 'frontend-console' | 'backend' | 'request'
export type DebugLogLevel = 'info' | 'success' | 'warn' | 'error'
export type DebugLogPhase = 'send' | 'receive'

export interface DebugSettings {
  enabled: boolean
}

export interface DebugLogEntry {
  id: string
  timestamp: string
  source: DebugLogSource
  level: DebugLogLevel
  phase?: DebugLogPhase
  action: string
  detail?: string
}
