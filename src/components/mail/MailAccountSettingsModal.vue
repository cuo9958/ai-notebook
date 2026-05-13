<script setup lang="ts">
import { computed, ref } from 'vue'
import MailAccountModal from '@/components/mail/MailAccountModal.vue'
import type { MailAccount, MailAccountInput } from '@/types/mail'

const props = defineProps<{
  accounts: MailAccount[]
  loading: boolean
  saving: boolean
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  save: [payload: MailAccountInput]
  delete: [account: MailAccount]
}>()

const editorVisible = ref(false)
const editingAccount = ref<MailAccount | null>(null)

const hasAccounts = computed(() => props.accounts.length > 0)

function close() {
  emit('update:modelValue', false)
}

function openCreate() {
  editingAccount.value = null
  editorVisible.value = true
}

function openEdit(account: MailAccount) {
  editingAccount.value = account
  editorVisible.value = true
}

function handleSave(payload: MailAccountInput) {
  emit('save', payload)
}

function handleDelete(account: MailAccount) {
  emit('delete', account)
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="settings-mask" @click.self="close">
      <section class="settings-modal">
        <header class="settings-modal__header">
          <div>
            <p class="settings-modal__eyebrow">MAIL ACCOUNTS</p>
            <h3>邮箱账号管理</h3>
            <p class="settings-modal__desc">只展示名称、邮箱和启用状态，详细配置进入编辑窗口处理。</p>
          </div>
          <div class="settings-modal__actions">
            <button type="button" class="primary-btn" @click="openCreate">添加账号</button>
            <button type="button" class="ghost-btn" @click="close">关闭</button>
          </div>
        </header>

        <div class="settings-modal__body">
          <div v-if="loading" class="empty-state">邮箱账号加载中...</div>
          <div v-else-if="!hasAccounts" class="empty-state">还没有配置任何邮箱账号。</div>

          <article v-for="account in accounts" :key="account.id" v-else class="account-card">
            <div class="account-card__top">
              <div>
                <strong>{{ account.name }}</strong>
                <p>{{ account.address }}</p>
              </div>
              <span class="account-card__badge" :class="{ 'account-card__badge--muted': !account.enabled }">
                {{ account.enabled ? '已启用' : '已停用' }}
              </span>
            </div>

            <div class="account-card__actions">
              <button type="button" class="ghost-btn" @click="openEdit(account)">编辑</button>
              <button type="button" class="danger-btn" @click="handleDelete(account)">删除</button>
            </div>
          </article>
        </div>
      </section>
    </div>
  </Teleport>

  <MailAccountModal v-model="editorVisible" :account="editingAccount" :loading="saving" @save="handleSave" />
</template>

<style scoped>
.settings-mask {
  position: fixed;
  inset: 0;
  z-index: 68;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.28);
  backdrop-filter: blur(10px);
}

.settings-modal {
  display: flex;
  width: min(980px, calc(100vw - 32px));
  max-height: calc(100vh - 48px);
  flex-direction: column;
  overflow: hidden;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: #fff;
  box-shadow: 0 28px 80px rgba(15, 23, 42, 0.16);
}

.settings-modal__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  border-bottom: 1px solid #eef2f7;
  background: #fbfcff;
  padding: 18px 20px 14px;
}

.settings-modal__eyebrow {
  margin: 0 0 4px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.12em;
}

.settings-modal h3 {
  margin: 0;
  color: #101828;
  font-size: 22px;
  font-weight: 760;
  line-height: 1.2;
}

.settings-modal__desc {
  margin: 6px 0 0;
  color: #667085;
  font-size: 13px;
}

.settings-modal__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.settings-modal__body {
  display: grid;
  min-height: 0;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  overflow: auto;
  padding: 16px 18px 20px;
}

.account-card {
  display: flex;
  flex-direction: column;
  gap: 14px;
  border: 1px solid #e4e7ec;
  border-radius: 10px;
  background: #fff;
  padding: 14px;
}

.account-card__top,
.account-card__actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.account-card__top strong {
  display: block;
  color: #101828;
  font-size: 16px;
}

.account-card__top p {
  margin: 5px 0 0;
  color: #667085;
  font-size: 13px;
}

.account-card__badge {
  border-radius: 999px;
  background: #eef4ff;
  color: var(--accent);
  padding: 5px 9px;
  font-size: 12px;
  font-weight: 700;
}

.account-card__badge--muted {
  background: #f2f4f7;
  color: #667085;
}

.empty-state {
  grid-column: 1 / -1;
  border: 1px dashed #d0d5dd;
  border-radius: 10px;
  background: #fbfcff;
  padding: 18px;
  color: #667085;
}

.primary-btn,
.ghost-btn,
.danger-btn {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 9px 13px;
  font-size: 13px;
  font-weight: 700;
}

.primary-btn {
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #fff;
  box-shadow: 0 12px 26px rgba(37, 99, 235, 0.2);
}

.ghost-btn {
  border-color: #e4e7ec;
  background: #fff;
  color: #344054;
}

.ghost-btn:hover {
  border-color: rgba(37, 99, 235, 0.22);
  background: #f4f7ff;
  color: var(--accent);
}

.danger-btn {
  border-color: rgba(220, 38, 38, 0.16);
  background: rgba(220, 38, 38, 0.06);
  color: #b42318;
}

@media (max-width: 900px) {
  .settings-modal__header {
    flex-direction: column;
  }

  .settings-modal__body {
    grid-template-columns: 1fr;
  }
}
</style>
