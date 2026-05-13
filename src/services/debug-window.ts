import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'

export async function openDebugLogWindow() {
  return invoke('open_debug_log_window')
}

export async function closeDebugLogWindow() {
  return invoke('close_debug_log_window')
}

export async function getAppVersionLabel() {
  return getVersion()
}
