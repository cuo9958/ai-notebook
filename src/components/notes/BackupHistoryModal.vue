<script setup lang="ts">
import { computed } from 'vue'
import dayjs from 'dayjs'
import type { BackupDocument, BackupHistoryItem } from '@/types/note'

const props = defineProps<{
  activeBackupPath: string
  backups: BackupHistoryItem[]
  loading: boolean
  modelValue: boolean
  preview: BackupDocument | null
  previewLoading: boolean
  restoring: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  preview: [path: string]
  restore: [path: string]
}>()

const sortedBackups = computed(() => props.backups)

function close() {
  emit('update:modelValue', false)
}

function formatTime(value: string | null) {
  return value ? dayjs(value).format('YYYY-MM-DD HH:mm:ss') : '未知时间'
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="backup-mask" @click.self="close">
      <section class="backup-modal">
        <header class="backup-modal__header">
          <div>
            <p class="backup-modal__eyebrow">BACKUP</p>
            <h3>备份历史</h3>
          </div>
          <button type="button" class="backup-modal__close" @click="close">关闭</button>
        </header>

        <div class="backup-modal__body">
          <aside class="backup-list">
            <div class="backup-list__header">
              <strong>历史记录</strong>
              <span>{{ sortedBackups.length }} 条</span>
            </div>

            <div v-if="loading" class="backup-list__empty">备份列表加载中...</div>
            <div v-else-if="!sortedBackups.length" class="backup-list__empty">还没有找到备份记录。</div>

            <button
              v-for="item in sortedBackups"
              v-else
              :key="item.path"
              type="button"
              class="backup-list__item"
              :class="{ 'backup-list__item--active': activeBackupPath === item.path }"
              @click="emit('preview', item.path)"
            >
              <div class="backup-list__item-title">{{ item.noteTitle }}</div>
              <div class="backup-list__item-time">{{ formatTime(item.createdAt) }}</div>
              <div class="backup-list__item-name">{{ item.name }}</div>
            </button>
          </aside>

          <section class="backup-preview">
            <div class="backup-preview__header">
              <div>
                <strong>{{ preview?.noteTitle ?? '选择一条备份记录' }}</strong>
                <p>{{ preview ? formatTime(preview.createdAt) : '点击左侧记录后预览备份内容' }}</p>
              </div>
              <button
                type="button"
                class="backup-preview__restore"
                :disabled="!preview || restoring"
                @click="preview && emit('restore', preview.path)"
              >
                {{ restoring ? '恢复中...' : '恢复到笔记' }}
              </button>
            </div>

            <div v-if="previewLoading" class="backup-preview__empty">正在加载备份内容...</div>
            <pre v-else-if="preview" class="backup-preview__content">{{ preview.content }}</pre>
            <div v-else class="backup-preview__empty">还没有选中的备份记录。</div>
          </section>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.backup-mask {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.28);
  backdrop-filter: blur(10px);
}

.backup-modal {
  display: flex;
  width: min(1120px, calc(100vw - 32px));
  height: min(760px, calc(100vh - 32px));
  flex-direction: column;
  overflow: hidden;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: #fff;
  box-shadow: 0 28px 80px rgba(15, 23, 42, 0.16);
}

.backup-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 20px 14px;
  border-bottom: 1px solid #eef2f7;
  background: #fbfcff;
}

.backup-modal__eyebrow {
  margin: 0 0 4px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.12em;
}

.backup-modal__header h3,
.backup-list__header strong {
  margin: 0;
}

.backup-modal__header h3 {
  color: #101828;
  font-family: var(--font-display);
  font-size: 22px;
  font-weight: 760;
  line-height: 1.2;
}

.backup-modal__close,
.backup-preview__restore {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 9px 14px;
  font-size: 13px;
  font-weight: 700;
}

.backup-modal__close {
  border-color: #e4e7ec;
  background: #fff;
  color: #344054;
}

.backup-modal__close:hover {
  border-color: rgba(37, 99, 235, 0.22);
  background: #f4f7ff;
  color: var(--accent);
}

.backup-preview__restore {
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #fff;
  box-shadow: 0 12px 26px rgba(37, 99, 235, 0.2);
}

.backup-preview__restore:disabled {
  cursor: not-allowed;
  opacity: 0.56;
  box-shadow: none;
}

.backup-modal__body {
  display: grid;
  grid-template-columns: 320px minmax(0, 1fr);
  min-height: 0;
  flex: 1;
}

.backup-list {
  display: flex;
  min-height: 0;
  flex-direction: column;
  gap: 8px;
  overflow: auto;
  border-right: 1px solid #eef2f7;
  padding: 14px;
  background: #fff;
}

.backup-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 2px 6px;
  color: #667085;
  font-size: 12px;
}

.backup-list__header strong {
  color: #344054;
  font-size: 13px;
}

.backup-list__item {
  border: 0;
  border-radius: 8px;
  background: transparent;
  padding: 11px 12px;
  text-align: left;
}

.backup-list__item:hover {
  background: #f4f7ff;
}

.backup-list__item--active {
  background: #eef4ff;
}

.backup-list__item-title {
  overflow: hidden;
  color: #1d2939;
  font-size: 13px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.backup-list__item--active .backup-list__item-title {
  color: #1d4ed8;
}

.backup-list__item-time,
.backup-list__item-name,
.backup-preview__header p,
.backup-list__empty,
.backup-preview__empty {
  color: #667085;
  font-size: 12px;
}

.backup-list__item-time {
  margin-top: 5px;
}

.backup-list__item-name {
  margin-top: 3px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.backup-preview {
  display: flex;
  min-height: 0;
  flex-direction: column;
  background: #fff;
}

.backup-preview__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 20px;
  border-bottom: 1px solid #eef2f7;
}

.backup-preview__header strong {
  display: block;
  color: #101828;
  font-size: 20px;
  font-weight: 760;
}

.backup-preview__header p {
  margin: 5px 0 0;
}

.backup-preview__content,
.backup-preview__empty {
  margin: 0;
  flex: 1;
  overflow: auto;
  padding: 20px 24px 28px;
}

.backup-preview__content {
  background: #fff;
  color: #182230;
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
