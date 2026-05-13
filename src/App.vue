<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import AppShell from '@/layouts/AppShell.vue'
import DebugLogsPage from '@/pages/debug/DebugLogsPage.vue'
import { disposeDebugLogging, initializeDebugLogging, initializeDebugLogViewer } from '@/services/debug'

const isDebugLogWindow = getCurrentWebviewWindow().label === 'debug-log-window'

function preventNativeContextMenu(event: MouseEvent) {
  event.preventDefault()
}

function isDebugEnvironment() {
  return typeof window !== 'undefined' && Boolean((window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
}

function handleGlobalKeydown(event: KeyboardEvent) {
  if (!isDebugEnvironment()) {
    return
  }

  if (event.repeat || !event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) {
    return
  }

  if (event.key.toLowerCase() !== 'p') {
    return
  }

  event.preventDefault()
  void invoke('open_debug_devtools')
}

onMounted(() => {
  if (isDebugLogWindow) {
    void initializeDebugLogViewer()
    return
  }

  void initializeDebugLogging()
  window.addEventListener('contextmenu', preventNativeContextMenu)
  window.addEventListener('keydown', handleGlobalKeydown)
})

onBeforeUnmount(() => {
  if (isDebugLogWindow) {
    return
  }

  disposeDebugLogging()
  window.removeEventListener('contextmenu', preventNativeContextMenu)
  window.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <DebugLogsPage v-if="isDebugLogWindow" />
  <AppShell v-else />
</template>
