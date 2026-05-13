<script setup lang="ts">
import { reactive, watch } from 'vue'
import type { MailAccount } from '@/types/mail'

const props = defineProps<{
  account: MailAccount | null
  loading: boolean
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  save: [
    payload: {
      id?: string
      name: string
      address: string
      enabled: boolean
      imapHost: string
      imapPort: number
      smtpHost: string
      smtpPort: number
      username: string
      password: string
      useTls: boolean
      defaultSender: boolean
    },
  ]
}>()

const form = reactive({
  id: '',
  name: '',
  address: '',
  enabled: true,
  imapHost: '',
  imapPort: 993,
  smtpHost: '',
  smtpPort: 465,
  username: '',
  password: '',
  useTls: true,
  defaultSender: false,
})

watch(
  () => props.modelValue,
  (visible) => {
    if (!visible) {
      return
    }

    form.id = props.account?.id ?? ''
    form.name = props.account?.name ?? ''
    form.address = props.account?.address ?? ''
    form.enabled = props.account?.enabled ?? true
    form.imapHost = props.account?.imapHost ?? ''
    form.imapPort = props.account?.imapPort ?? 993
    form.smtpHost = props.account?.smtpHost ?? ''
    form.smtpPort = props.account?.smtpPort ?? 465
    form.username = props.account?.username ?? props.account?.address ?? ''
    form.password = props.account?.password ?? ''
    form.useTls = props.account?.useTls ?? true
    form.defaultSender = props.account?.defaultSender ?? false
  },
  { immediate: true },
)

function close() {
  emit('update:modelValue', false)
}

function submit() {
  emit('save', {
    id: form.id || undefined,
    name: form.name.trim(),
    address: form.address.trim(),
    enabled: form.enabled,
    imapHost: form.imapHost.trim(),
    imapPort: Number(form.imapPort),
    smtpHost: form.smtpHost.trim(),
    smtpPort: Number(form.smtpPort),
    username: form.username.trim(),
    password: form.password,
    useTls: form.useTls,
    defaultSender: form.defaultSender,
  })
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="mail-modal-mask" @click.self="close">
      <section class="mail-modal">
        <header class="mail-modal__header">
          <div>
            <p class="mail-modal__eyebrow">MAIL ACCOUNT</p>
            <h3>{{ account ? '编辑邮箱账号' : '添加邮箱账号' }}</h3>
          </div>
          <button type="button" class="mail-modal__close" @click="close">关闭</button>
        </header>

        <div class="mail-modal__grid">
          <label class="mail-field">
            <span>账号名称</span>
            <input v-model="form.name" type="text" placeholder="例如：工作邮箱" />
          </label>
          <label class="mail-field">
            <span>邮箱地址</span>
            <input v-model="form.address" type="email" placeholder="name@example.com" />
          </label>
          <label class="mail-field">
            <span>启用状态</span>
            <select v-model="form.enabled">
              <option :value="true">启用</option>
              <option :value="false">停用</option>
            </select>
          </label>
          <label class="mail-field">
            <span>IMAP 服务器</span>
            <input v-model="form.imapHost" type="text" placeholder="imap.example.com" />
          </label>
          <label class="mail-field">
            <span>IMAP 端口</span>
            <input v-model.number="form.imapPort" type="number" min="1" />
          </label>
          <label class="mail-field">
            <span>SMTP 服务器</span>
            <input v-model="form.smtpHost" type="text" placeholder="smtp.example.com" />
          </label>
          <label class="mail-field">
            <span>SMTP 端口</span>
            <input v-model.number="form.smtpPort" type="number" min="1" />
          </label>
          <label class="mail-field">
            <span>登录用户名</span>
            <input v-model="form.username" type="text" placeholder="通常是邮箱地址" />
          </label>
          <label class="mail-field">
            <span>密码 / 授权码</span>
            <input v-model="form.password" type="password" placeholder="请输入密码或授权码" />
          </label>
        </div>

        <div class="mail-modal__options">
          <label class="mail-check">
            <input v-model="form.useTls" type="checkbox" />
            <span>启用 TLS</span>
          </label>
          <label class="mail-check">
            <input v-model="form.defaultSender" type="checkbox" />
            <span>设为默认发信账号</span>
          </label>
        </div>

        <footer class="mail-modal__footer">
          <button type="button" class="ghost-btn" @click="close">取消</button>
          <button type="button" class="primary-btn" :disabled="loading" @click="submit">
            {{ loading ? '保存中...' : '保存账号' }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.mail-modal-mask {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.28);
  backdrop-filter: blur(10px);
}

.mail-modal {
  display: flex;
  width: min(860px, calc(100vw - 32px));
  max-height: min(760px, calc(100vh - 32px));
  flex-direction: column;
  overflow: auto;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: #fff;
  box-shadow: 0 28px 80px rgba(15, 23, 42, 0.16);
}

.mail-modal__header,
.mail-modal__footer,
.mail-modal__options {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.mail-modal__header {
  border-bottom: 1px solid #eef2f7;
  background: #fbfcff;
  padding: 18px 20px 14px;
}

.mail-modal__eyebrow {
  margin: 0 0 4px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.12em;
}

.mail-modal h3 {
  margin: 0;
  color: #101828;
  font-size: 22px;
  font-weight: 760;
  line-height: 1.2;
}

.mail-modal__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
  padding: 18px 20px 0;
}

.mail-field {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.mail-field span,
.mail-check span {
  color: #344054;
  font-size: 13px;
  font-weight: 700;
}

.mail-field input,
.mail-field select {
  border: 1px solid #e4e7ec;
  border-radius: 8px;
  background: #fff;
  padding: 10px 12px;
  color: #101828;
}

.mail-field input:focus,
.mail-field select:focus {
  border-color: rgba(37, 99, 235, 0.38);
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.08);
  outline: none;
}

.mail-modal__close,
.ghost-btn,
.primary-btn {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 9px 14px;
  font-size: 13px;
  font-weight: 700;
}

.mail-modal__close,
.ghost-btn {
  border-color: #e4e7ec;
  background: #fff;
  color: #344054;
}

.mail-modal__close:hover,
.ghost-btn:hover {
  border-color: rgba(37, 99, 235, 0.22);
  background: #f4f7ff;
  color: var(--accent);
}

.primary-btn {
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #fff;
  box-shadow: 0 12px 26px rgba(37, 99, 235, 0.2);
}

.primary-btn:disabled {
  cursor: not-allowed;
  opacity: 0.58;
  box-shadow: none;
}

.mail-modal__options {
  justify-content: flex-start;
  margin: 16px 20px 0;
}

.mail-check {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.mail-modal__footer {
  margin-top: 18px;
  border-top: 1px solid #eef2f7;
  padding: 14px 20px 18px;
}

@media (max-width: 760px) {
  .mail-modal__grid {
    grid-template-columns: 1fr;
  }
}
</style>
