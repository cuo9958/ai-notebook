<script setup lang="ts">
import { computed } from 'vue'
import { clearDebugLogs, useDebugLogState } from '@/services/debug'

const { logs } = useDebugLogState()

const logCountText = computed(() => `共 ${logs.value.length} 条`)

function formatTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleString('zh-CN', {
    hour12: false,
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

function getSourceLabel(source: string) {
  if (source === 'backend') {
    return 'Rust 后端'
  }

  if (source === 'frontend-console') {
    return '前端 Console'
  }

  if (source === 'request') {
    return '请求'
  }

  return '前端动作'
}

function getPhaseLabel(phase?: string) {
  if (phase === 'send') {
    return '发出'
  }

  if (phase === 'receive') {
    return '收到'
  }

  return ''
}

function getLevelLabel(level: string) {
  if (level === 'success') {
    return '成功'
  }

  if (level === 'warn') {
    return '警告'
  }

  if (level === 'error') {
    return '错误'
  }

  return '信息'
}
</script>

<template>
  <section class="debug-page">
    <header class="debug-page__header">
      <div>
        <p class="debug-page__eyebrow">Debug Console</p>
        <h2>运行日志</h2>
      </div>

      <div class="debug-page__actions">
        <span class="debug-page__count">{{ logCountText }}</span>
        <button type="button" class="debug-page__clear" @click="clearDebugLogs">清空</button>
      </div>
    </header>

    <div class="debug-log-list">
      <article
        v-for="log in logs"
        :key="log.id"
        class="debug-log-item"
        :data-level="log.level"
        :data-source="log.source"
        :data-phase="log.phase"
      >
        <div class="debug-log-item__meta">
          <span>{{ formatTime(log.timestamp) }}</span>
          <span class="debug-log-badge">{{ getSourceLabel(log.source) }}</span>
          <span v-if="log.phase" class="debug-log-badge debug-log-badge--phase">{{ getPhaseLabel(log.phase) }}</span>
          <span class="debug-log-badge">{{ getLevelLabel(log.level) }}</span>
        </div>
        <strong>{{ log.action }}</strong>
        <p v-if="log.detail">{{ log.detail }}</p>
      </article>

      <div v-if="!logs.length" class="debug-log-empty">调试模式已开启，等待新的日志事件。</div>
    </div>
  </section>
</template>

<style scoped>
.debug-page {
  display: flex;
  height: 100%;
  min-height: 0;
  flex-direction: column;
  padding: 4px 0 0;
}

.debug-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border: 1px solid var(--line-strong);
  border-bottom: 0;
  background: rgba(255, 251, 245, 0.84);
  padding: 14px 18px;
}

.debug-page__eyebrow {
  margin: 0 0 4px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.debug-page__header h2 {
  margin: 0;
  color: var(--ink);
  font-family: var(--font-display);
  font-size: 24px;
}

.debug-page__actions {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.debug-page__count,
.debug-page__clear {
  border: 1px solid var(--line);
  background: rgba(255, 251, 245, 0.88);
  padding: 8px 12px;
  color: var(--ink);
  font-size: 12px;
}

.debug-log-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  border: 1px solid var(--line-strong);
  background: rgba(24, 22, 19, 0.94);
  padding: 12px;
}

.debug-log-item,
.debug-log-empty {
  border: 1px solid rgba(255, 247, 239, 0.12);
  background: rgba(255, 247, 239, 0.04);
  padding: 12px 14px;
  color: #f3e4d6;
}

.debug-log-item + .debug-log-item {
  margin-top: 10px;
}

.debug-log-item[data-level='error'] {
  border-color: rgba(225, 94, 67, 0.36);
  background: rgba(180, 62, 36, 0.12);
}

.debug-log-item[data-level='warn'] {
  border-color: rgba(245, 158, 11, 0.38);
  background: rgba(245, 158, 11, 0.11);
}

.debug-log-item[data-level='success'] {
  border-color: rgba(88, 168, 114, 0.36);
  background: rgba(88, 168, 114, 0.1);
}

.debug-log-item[data-source='backend'] {
  border-left: 3px solid #60a5fa;
}

.debug-log-item[data-source='frontend-console'] {
  border-left: 3px solid #f59e0b;
}

.debug-log-item[data-source='request'][data-phase='send'] {
  border-left: 3px solid #a78bfa;
}

.debug-log-item[data-source='request'][data-phase='receive'] {
  border-left: 3px solid #34d399;
}

.debug-log-item__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 6px;
  color: rgba(243, 228, 214, 0.72);
  font-family: var(--font-mono);
  font-size: 11px;
  text-transform: uppercase;
}

.debug-log-badge {
  border: 1px solid rgba(243, 228, 214, 0.16);
  border-radius: 999px;
  background: rgba(255, 247, 239, 0.08);
  padding: 2px 7px;
  color: rgba(255, 247, 239, 0.86);
}

.debug-log-badge--phase {
  border-color: rgba(96, 165, 250, 0.28);
  background: rgba(96, 165, 250, 0.12);
  color: #bfdbfe;
}

.debug-log-item strong {
  display: block;
  margin-bottom: 4px;
  font-size: 13px;
}

.debug-log-item p,
.debug-log-empty {
  margin: 0;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
