<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import dayjs from 'dayjs'
import { storeToRefs } from 'pinia'
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  Mail,
  PencilLine,
  RefreshCcw,
  Reply,
  Search,
  Send,
} from '@lucide/vue'
import MailAccountSettingsModal from '@/components/mail/MailAccountSettingsModal.vue'
import MailComposerModal from '@/components/mail/MailComposerModal.vue'
import { useMailStore } from '@/stores/mail'
import type { MailAccount, MailAccountInput, MailMessageSummary } from '@/types/mail'

const store = useMailStore()
const {
  accounts,
  activeMessage,
  activeMessageKey,
  defaultAccount,
  loadingAccounts,
  loadingDetail,
  loadingMessages,
  messages,
  sending,
  syncingMessages,
} = storeToRefs(store)

const accountSettingsVisible = ref(false)
const composerModalVisible = ref(false)
const accountSaving = ref(false)
const searchQuery = ref('')
const activeAccountFilter = ref('all')
const composerDraft = reactive({
  accountId: '',
  to: '',
  cc: '',
  subject: '',
  body: '',
  isHtml: false,
})

const availableAccounts = computed(() => accounts.value.filter((account) => account.enabled))

const filteredMessages = computed(() =>
  messages.value.filter((message) => {
    const matchesAccount = activeAccountFilter.value === 'all' || message.accountId === activeAccountFilter.value
    const query = searchQuery.value.trim().toLowerCase()
    const matchesQuery =
      !query ||
      message.subject.toLowerCase().includes(query) ||
      message.from.toLowerCase().includes(query) ||
      message.preview.toLowerCase().includes(query)

    return matchesAccount && matchesQuery
  }),
)

const activeMessageIndex = computed(() =>
  filteredMessages.value.findIndex((message) => messageKey(message) === activeMessageKey.value),
)

const hasPreviousMessage = computed(() => activeMessageIndex.value > 0)
const hasNextMessage = computed(
  () => activeMessageIndex.value >= 0 && activeMessageIndex.value < filteredMessages.value.length - 1,
)

const isHeaderExpanded = ref(false)

function toggleHeaderExpand() {
  isHeaderExpanded.value = !isHeaderExpanded.value
}

const detailHtml = computed(() => activeMessage.value?.htmlBody ?? '')
const hasRenderedHtml = computed(() => Boolean(activeMessage.value?.htmlBody?.trim()))

function formatDate(value: string) {
  return dayjs(value).format('YYYY-MM-DD HH:mm')
}

function formatShortDate(value: string) {
  const date = dayjs(value)
  return date.isSame(dayjs(), 'day') ? date.format('HH:mm') : date.format('MM-DD')
}

function messageKey(message: Pick<MailMessageSummary, 'accountId' | 'uid'>) {
  return `${message.accountId}:${message.uid}`
}

function resetComposerDraft() {
  composerDraft.accountId = (defaultAccount.value?.enabled ? defaultAccount.value.id : availableAccounts.value[0]?.id) ?? ''
  composerDraft.to = ''
  composerDraft.cc = ''
  composerDraft.subject = ''
  composerDraft.body = ''
  composerDraft.isHtml = false
}

function quotedBody() {
  if (!activeMessage.value) {
    return ''
  }

  const source = activeMessage.value.textBody || activeMessage.value.htmlBody || ''
  return `\n\n---- 原始邮件 ----\n发件人：${activeMessage.value.from || '未知发件人'}\n时间：${formatDate(activeMessage.value.date)}\n主题：${activeMessage.value.subject}\n\n${source}`
}

function normalizeReplyAddress(value: string) {
  const match = value.match(/<([^>]+)>/)
  return (match?.[1] ?? value).trim()
}

async function handleSaveAccount(payload: MailAccountInput) {
  accountSaving.value = true
  try {
    await store.upsertAccount(payload)
  } finally {
    accountSaving.value = false
  }
}

async function handleDeleteAccount(account: MailAccount) {
  if (!window.confirm(`确认删除邮箱账号“${account.name}”吗？`)) {
    return
  }

  accountSaving.value = true
  try {
    await store.removeAccount(account.id)
  } finally {
    accountSaving.value = false
  }
}

async function refreshMessages() {
  await store.pullMessages()
}

function openComposer() {
  resetComposerDraft()
  composerModalVisible.value = true
}

function openReplyComposer() {
  if (!activeMessage.value) return

  resetComposerDraft()
  composerDraft.accountId = activeMessage.value.accountId
  composerDraft.to = activeMessage.value.from ? normalizeReplyAddress(activeMessage.value.from) : ''
  const originalCc = activeMessage.value.cc?.join('; ')
  if (originalCc) composerDraft.cc = originalCc

  composerDraft.subject = activeMessage.value.subject.startsWith('Re:')
    ? activeMessage.value.subject
    : `Re: ${activeMessage.value.subject}`
  composerDraft.body = quotedBody()
  composerModalVisible.value = true
}

function openForwardComposer() {
  if (!activeMessage.value) return

  resetComposerDraft()
  composerDraft.accountId = activeMessage.value.accountId
  const originalTo = activeMessage.value.to?.join('; ')
  if (originalTo) composerDraft.to = originalTo
  const originalCc = activeMessage.value.cc?.join('; ')
  if (originalCc) composerDraft.cc = originalCc

  composerDraft.subject = activeMessage.value.subject.startsWith('Fwd:')
    ? activeMessage.value.subject
    : `Fwd: ${activeMessage.value.subject}`
  composerDraft.body = quotedBody()
  composerModalVisible.value = true
}

async function openAdjacentMessage(direction: 'previous' | 'next') {
  const index = activeMessageIndex.value
  if (index < 0) return

  const nextIndex = direction === 'previous' ? index - 1 : index + 1
  const message = filteredMessages.value[nextIndex]
  if (!message) return

  await store.openMessage(message.accountId, message.uid)
}

async function handleSend(payload: {
  accountId: string
  to: string[]
  cc: string[]
  subject: string
  body: string
  isHtml: boolean
}) {
  await store.sendMessage(payload)
  composerModalVisible.value = false
}

onMounted(() => {
  void store.initialize()
})
</script>

<template>
  <section class="mail-shell">
    <MailAccountSettingsModal
      v-model="accountSettingsVisible"
      :accounts="accounts"
      :loading="loadingAccounts"
      :saving="accountSaving"
      @save="handleSaveAccount"
      @delete="handleDeleteAccount"
    />

    <MailComposerModal
      v-model="composerModalVisible"
      :accounts="availableAccounts"
      :default-account-id="composerDraft.accountId"
      :initial-draft="composerDraft"
      :loading="sending"
      @send="handleSend"
    />

    <section class="mail-panel">
      <header class="mail-toolbar">
        <div class="mail-toolbar__left">
          <div class="mail-account-select">
            <label class="mail-account-select__label">账号</label>
            <select v-model="activeAccountFilter" class="mail-account-select__input">
              <option value="all">全部账号</option>
              <option v-for="account in availableAccounts" :key="account.id" :value="account.id">
                {{ account.name }}
              </option>
            </select>
          </div>

          <div class="mail-search">
            <Search :size="16" :stroke-width="2" />
            <input v-model="searchQuery" type="text" placeholder="搜索邮件..." />
          </div>
        </div>

        <div class="mail-toolbar__actions">
          <button
            type="button"
            class="mail-icon-btn"
            :class="{ 'mail-icon-btn--loading': syncingMessages }"
            title="拉取邮件"
            :disabled="syncingMessages"
            @click="refreshMessages"
          >
            <RefreshCcw :size="18" :stroke-width="2" />
          </button>

          <button type="button" class="mail-icon-btn" title="写邮件" @click="openComposer">
            <PencilLine :size="16" :stroke-width="2" />
            <span>写邮件</span>
          </button>
        </div>
      </header>

      <main class="mail-layout">
        <!-- 邮件列表 -->
        <div class="mail-list">
          <div class="mail-list__header">
            <span>收件箱</span>
            <span class="mail-list__count">{{ filteredMessages.length }}</span>
          </div>

          <div v-if="loadingMessages" class="mail-empty-tip">加载中...</div>
          <div v-else-if="!filteredMessages.length" class="mail-empty-tip">当前没有邮件。</div>

          <button
            v-for="message in filteredMessages"
            v-else
            :key="messageKey(message)"
            type="button"
            class="message-row"
            :class="{
              'message-row--active': activeMessageKey === messageKey(message),
              'message-row--unread': message.unread,
            }"
            @click="store.openMessage(message.accountId, message.uid)"
          >
            <div class="message-row__content">
              <div class="message-row__head">
                <strong class="message-row__subject">{{ message.subject || '无主题' }}</strong>
                <span class="message-row__date">{{ formatShortDate(message.date) }}</span>
              </div>
              <div class="message-row__body">
                <span class="message-row__sender">{{ message.from || '未知发件人' }}</span>
                <p class="message-row__preview">{{ message.preview || '暂无内容预览' }}</p>
              </div>
            </div>
          </button>
        </div>

        <!-- 邮件详情 -->
        <section class="mail-detail">
          <div v-if="loadingDetail" class="mail-empty-tip">加载邮件详情中...</div>

          <article v-else-if="activeMessage" class="mail-detail__container">
            <header class="mail-detail__header">
              <div class="mail-detail__header-top">
                <div class="mail-detail__header-info">
                  <h1 class="mail-detail__subject">{{ activeMessage.subject || '无主题' }}</h1>
                  <div class="mail-detail__header-sub">
                    <span class="mail-detail__sender">发件人：{{ activeMessage.from }}</span>
                    <span v-if="isHeaderExpanded" class="mail-detail__sender-time">{{ formatDate(activeMessage.date) }}</span>
                  </div>
                </div>

                <div class="mail-detail__tags">
                  <span class="mail-tag">{{ activeMessage.accountName }}</span>
                  <span v-if="activeMessage.unread" class="mail-tag mail-tag--unread">未读</span>
                </div>

                <button type="button" class="mail-expand-btn" @click="toggleHeaderExpand" title="展开详情">
                  <ChevronDown v-if="!isHeaderExpanded" :size="18" :stroke-width="2" />
                  <ChevronUp v-else :size="18" :stroke-width="2" />
                </button>
              </div>

              <div class="mail-detail__action-bar">
                <button type="button" class="detail-action-btn" @click="openReplyComposer">
                  <Reply :size="16" :stroke-width="2" /> 回复
                </button>
                <button type="button" class="detail-action-btn" @click="openForwardComposer">
                  <Send :size="16" :stroke-width="2" /> 转发
                </button>
                
                <div class="detail-nav-btns">
                  <button
                    type="button"
                    class="detail-nav-btn"
                    :disabled="!hasPreviousMessage"
                    title="上一封"
                    @click="openAdjacentMessage('previous')"
                  >
                    <ChevronLeft :size="16" />
                  </button>
                  <button
                    type="button"
                    class="detail-nav-btn"
                    :disabled="!hasNextMessage"
                    title="下一封"
                    @click="openAdjacentMessage('next')"
                  >
                    <ChevronRight :size="16" />
                  </button>
                </div>
              </div>

              <Transition name="expand-details">
                <div v-if="isHeaderExpanded" class="mail-detail__meta">
                  <div class="mail-detail__meta-row">
                    <span class="meta-label">收件人</span>
                    <span class="meta-value">{{ activeMessage.to.join(', ') }}</span>
                  </div>
                  <div v-if="activeMessage.cc?.length" class="mail-detail__meta-row">
                    <span class="meta-label">抄送</span>
                    <span class="meta-value">{{ activeMessage.cc.join(', ') }}</span>
                  </div>
                </div>
              </Transition>
            </header>

            <div class="mail-detail__body">
              <iframe
                v-if="hasRenderedHtml"
                class="mail-detail__frame"
                :srcdoc="detailHtml"
                sandbox="allow-scripts allow-same-origin"
                title="邮件 HTML 预览"
              />
              <pre v-else class="mail-detail__text-body">{{ activeMessage.textBody || '暂无正文内容' }}</pre>
            </div>
          </article>

          <div v-else class="mail-empty-state">
            <div class="mail-empty-state__icon">
              <Mail :size="48" :stroke-width="1.5" />
            </div>
            <p>选择左侧列表中的邮件以查看内容</p>
          </div>
        </section>
      </main>
    </section>
  </section>
</template>

<style scoped>
/* =========================================
   MAIL PAGE UI - Complete Reset & Layout
   ========================================= */

.mail-shell {
  height: 100vh;
  display: flex;
  background: #ffffff;
  overflow: hidden;
}

.mail-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  background: #ffffff;
}

/* 1. 顶部工具栏 */
.mail-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  height: 64px;
  border-bottom: 1px solid #e2e8f0;
  background: #ffffff;
  flex-shrink: 0;
}

.mail-toolbar__left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.mail-account-select {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mail-account-select__label {
  color: #64748b;
  font-size: 13px;
  font-weight: 600;
}

.mail-account-select__input {
  height: 32px;
  padding: 0 10px;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #ffffff;
  color: #0f172a;
  font-size: 13px;
  outline: none;
  transition: all 0.2s;
}

.mail-account-select__input:hover { border-color: #cbd5e1; }
.mail-account-select__input:focus {
  border-color: #2563eb;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1);
}

.mail-search {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 240px;
  padding: 6px 12px;
  background: #f8fafc;
  border: 1px solid transparent;
  border-radius: 8px;
  transition: all 0.2s;
}

.mail-search:focus-within {
  background: #ffffff;
  border-color: #2563eb;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1);
}

.mail-search svg { color: #64748b; }

.mail-search input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: #0f172a;
  font-size: 14px;
}
.mail-search input::placeholder { color: #94a3b8; }

.mail-toolbar__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mail-icon-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: auto;
  height: 36px;
  padding: 0 12px;
  margin-left: 8px;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #ffffff;
  color: #64748b;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}
.mail-icon-btn:hover {
  background: #f1f5f9;
  color: #0f172a;
  border-color: #cbd5e1;
}
.mail-icon-btn--loading svg { animation: spin 1s linear infinite; }
@keyframes spin { 100% { transform: rotate(360deg); } }

/* 2. 布局与列表 */
.mail-layout {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
}

.mail-list {
  width: 340px;
  background: #ffffff;
  border-right: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.mail-list__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px 12px;
  color: #64748b;
  font-size: 14px;
  font-weight: 500;
  background: #ffffff;
}

.mail-list__count {
  background: #f1f5f9;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 11px;
  color: #334155;
}

/* ---- 列表项 (修复样式) ---- */
.message-row {
  /* 核心：清除 Button 默认样式 */
  display: flex;
  align-items: flex-start;
  width: 100%;
  text-align: left;
  background: transparent;
  border: none;
  border-radius: 0;
  outline: none;
  cursor: pointer;
  padding: 14px 20px;
  gap: 0;
  
  /* 视觉分割 */
  border-bottom: 1px solid #f8fafc; 
  transition: background 0.15s ease;
  position: relative;
}

.message-row:hover {
  background: #f8fafc;
  border-bottom-color: transparent;
}

.message-row--active {
  background: #eff6ff; /* 选中背景 */
  border-bottom-color: transparent;
}

/* 左侧指示条 */
.message-row--active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: #2563eb;
}

.message-row__content {
  flex: 1;
  min-width: 0; 
}

.message-row__head {
  display: flex;
  justify-content: space-between;
  margin-bottom: 4px;
  align-items: center;
}

.message-row__subject {
  font-size: 14px;
  font-weight: 600;
  color: #334155;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.message-row--active .message-row__subject {
  color: #2563eb;
}

.message-row__date {
  font-size: 11px;
  color: #94a3b8;
  white-space: nowrap;
  margin-left: 10px;
}

.message-row__body {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.message-row__sender {
  font-size: 12px;
  font-weight: 500;
  color: #475569;
}

.message-row__preview {
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin: 0;
  color: #94a3b8;
  font-size: 12px;
  line-height: 1.4;
}

/* 3. 邮件详情 */
.mail-detail {
  flex: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #f8fafc;
  overflow: hidden;
}

.mail-empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #64748b;
  gap: 16px;
}
.mail-empty-state__icon {
  padding: 20px;
  background: #ffffff;
  border-radius: 50%;
  color: #cbd5e1;
}

.mail-detail__container {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #ffffff;
  margin: 16px;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  overflow: hidden;
  box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
}

.mail-detail__header {
  background: #ffffff;
  border-bottom: 1px solid #e2e8f0;
  padding: 20px 24px;
}

.mail-detail__header-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}

.mail-detail__header-info {
  flex: 1;
  min-width: 0;
}

.mail-detail__subject {
  margin: 0 0 6px;
  color: #0f172a;
  font-size: 18px;
  font-weight: 700;
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mail-detail__header-sub {
  display: flex;
  align-items: center;
  gap: 12px;
  color: #64748b;
  font-size: 12px;
}

.mail-detail__sender { font-weight: 500; }
.mail-detail__sender-time {
  padding-left: 12px;
  border-left: 1px solid #e2e8f0;
}

.mail-detail__tags {
  display: flex;
  gap: 8px;
  margin-left: 16px;
  flex-shrink: 0;
}

.mail-tag {
  padding: 3px 8px;
  font-size: 11px;
  font-weight: 600;
  background: #f1f5f9;
  color: #64748b;
  border-radius: 4px;
}

.mail-tag--unread {
  background: #fee2e2;
  color: #b91c1c;
}

.mail-expand-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  background: #ffffff;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
  margin-left: 8px;
  flex-shrink: 0;
}
.mail-expand-btn:hover {
  background: #f1f5f9;
  color: #2563eb;
  border-color: #cbd5e1;
}

/* 详情快捷操作 */
.mail-detail__action-bar {
  display: flex;
  align-items: center;
  padding-bottom: 0;
}

.detail-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  margin-right: 8px;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #ffffff;
  color: #334155;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.detail-action-btn:hover {
  border-color: #2563eb;
  color: #2563eb;
  background: #eff6ff;
}

.detail-nav-btns {
  margin-left: auto;
  display: flex;
  gap: 4px;
}

.detail-nav-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: 1px solid #e2e8f0;
  background: #ffffff;
  border-radius: 6px;
  cursor: pointer;
  color: #334155;
  transition: all 0.2s;
}
.detail-nav-btn:hover:not(:disabled) { background: #f1f5f9; }
.detail-nav-btn:disabled { opacity: 0.4; cursor: not-allowed; }

/* 详情展开区 */
.mail-detail__meta {
  display: grid;
  gap: 10px;
  padding: 16px 0 0;
  border-top: 1px dashed #e2e8f0;
  color: #334155;
  font-size: 13px;
  margin-top: 16px;
}

.mail-detail__meta-row {
  display: flex;
  align-items: baseline;
}

.meta-label {
  width: 60px;
  color: #64748b;
  font-weight: 500;
  flex-shrink: 0;
}

.meta-value {
  color: #0f172a;
  line-height: 1.4;
  word-break: break-word;
}

.mail-detail__body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.mail-detail__frame {
  width: 100%;
  height: 100%;
  border: none;
  padding: 0;
}

.mail-detail__text-body {
  margin: 0;
  padding: 24px;
  height: 100%;
  overflow-y: auto;
  font-family: inherit;
  font-size: 14px;
  line-height: 1.7;
  color: #0f172a;
  white-space: pre-wrap;
  background: #ffffff;
}

/* 动画 */
.expand-details-enter-active,
.expand-details-leave-active {
  overflow: hidden;
  transition: max-height 0.3s ease, opacity 0.3s ease;
}
.expand-details-enter-from,
.expand-details-leave-to { max-height: 0; opacity: 0; }
.expand-details-enter-to,
.expand-details-leave-from { max-height: 200px; opacity: 1; }

.mail-empty-tip {
  padding: 20px;
  text-align: center;
  color: #64748b;
}

@media (max-width: 900px) {
  .mail-layout { flex-direction: column; }
  .mail-list {
    width: 100%;
    height: 40%;
    border-right: none;
    border-bottom: 1px solid #e2e8f0;
  }
  .mail-detail__container { margin: 0; border-radius: 0; border: none; }
}
</style>
