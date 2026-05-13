export type DebugLogSource = 'frontend' | 'backend'
export type DebugLogLevel = 'info' | 'success' | 'error'

export interface DebugSettings {
  enabled: boolean
}

export interface DebugLogEntry {
  id: string
  timestamp: string
  source: DebugLogSource
  level: DebugLogLevel
  action: string
  detail?: string
}
