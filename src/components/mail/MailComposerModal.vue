<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { X, ChevronDown } from '@lucide/vue'
import VditorEditor from '@/components/common/VditorEditor.vue'
import { listImageHosts, uploadImageWithHost } from '@/services/image-host'
import type { MailAccount } from '@/types/mail'

const props = defineProps<{
  accounts: MailAccount[]
  defaultAccountId: string
  initialDraft?: {
    accountId?: string
    to?: string
    cc?: string
    subject?: string
    body?: string
    isHtml?: boolean
  }
  loading: boolean
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  send: [
    payload: {
      accountId: string
      to: string[]
      cc: string[]
      subject: string
      body: string
      isHtml: boolean
    },
  ]
}>()

const form = ref({
  accountId: '',
  to: '',
  cc: '',
  subject: '',
  body: '',
  isHtml: false,
})

const showCc = ref(false)
const accountDropdownOpen = ref(false)
const accountDropdownRef = ref<HTMLElement | null>(null)

function close() {
  emit('update:modelValue', false)
}

function submit() {
  emit('send', {
    accountId: form.value.accountId,
    to: form.value.to
      .split(/[;,]/)
      .map((item) => item.trim())
      .filter(Boolean),
    cc: form.value.cc
      .split(/[;,]/)
      .map((item) => item.trim())
      .filter(Boolean),
    subject: form.value.subject.trim(),
    body: form.value.body,
    isHtml: form.value.isHtml,
  })
}

function toggleAccountDropdown() {
  accountDropdownOpen.value = !accountDropdownOpen.value
}

function selectAccount(accountId: string) {
  form.value.accountId = accountId
  accountDropdownOpen.value = false
}

function toggleCc() {
  showCc.value = !showCc.value
  if (showCc.value) {
    nextTick(() => {
      const ccInput = document.querySelector('.composer-field__cc input') as HTMLInputElement
      ccInput?.focus()
    })
  }
}

function closeAccountDropdown(event: MouseEvent) {
  if (accountDropdownRef.value && !accountDropdownRef.value.contains(event.target as Node)) {
    accountDropdownOpen.value = false
  }
}

function getImageExtension(file: File) {
  const typeMatch = file.type.match(/image\/([a-zA-Z0-9.+-]+)/)
  const nameMatch = file.name.match(/\.([a-zA-Z0-9]+)$/)
  const extension = (nameMatch?.[1] || typeMatch?.[1] || 'png').toLowerCase()

  if (extension === 'jpeg') {
    return 'jpg'
  }

  if (extension === 'svg+xml') {
    return 'svg'
  }

  return extension
}

function buildClipboardImageName(file: File) {
  return `mail-image-${Date.now()}-${crypto.randomUUID()}.${getImageExtension(file)}`
}

async function handleMailVditorUpload(files: File[]) {
  const images = files.filter((file) => file.type.startsWith('image/'))
  if (!images.length) {
    return null
  }

  const hosts = await listImageHosts()
  const host = hosts.find((item) => item.enabled)

  if (!host) {
    throw new Error('请先在设置中启用一个图床')
  }

  const markdownItems: string[] = []

  for (const image of images) {
    const uploadFile = image.name
      ? image
      : new File([image], buildClipboardImageName(image), { type: image.type || 'image/png' })
    const alt = uploadFile.name.replace(/\.[^.]+$/, '') || 'image'
    const uploaded = await uploadImageWithHost(host.id, uploadFile)

    markdownItems.push(`![${alt}](${uploaded.url})`)
  }

  return markdownItems.length ? `\n${markdownItems.join('\n')}\n` : null
}

onMounted(() => {
  document.addEventListener('click', closeAccountDropdown)
})

onUnmounted(() => {
  document.removeEventListener('click', closeAccountDropdown)
})

watch(
  () => props.modelValue,
  (visible) => {
    if (!visible) return

    form.value.accountId = props.initialDraft?.accountId || props.defaultAccountId
    form.value.to = props.initialDraft?.to ?? ''
    form.value.cc = props.initialDraft?.cc ?? ''
    form.value.subject = props.initialDraft?.subject ?? ''
    form.value.body = props.initialDraft?.body ?? ''
    form.value.isHtml = props.initialDraft?.isHtml ?? false

    if (props.initialDraft?.cc) {
      showCc.value = true
    }
  },
  { immediate: true },
)

const selectedAccountLabel = ref('')
watch(
  () => [form.value.accountId, props.accounts],
  () => {
    const account = props.accounts.find((a) => a.id === form.value.accountId)
    selectedAccountLabel.value = account ? `${account.address}` : '选择账号'
  },
  { immediate: true },
)
</script>

<template>
  <Teleport to="body">
    <Transition name="composer-fade">
      <div v-if="modelValue" class="composer-mask" @click.self="close">
        <section class="composer-modal" @keydown.esc.prevent="close">
          <header class="composer-header">
            <h3 class="composer-title">编辑邮件</h3>
            <button type="button" class="composer-close-btn" @click="close">
              <X :size="18" />
            </button>
          </header>

          <div class="composer-toolbar">
            <div ref="accountDropdownRef" class="composer-dropdown">
              <button type="button" class="composer-dropdown__trigger" @click="toggleAccountDropdown">
                <span class="composer-dropdown__value">{{ selectedAccountLabel }}</span>
                <ChevronDown :size="14" :class="{ 'composer-dropdown__arrow--open': accountDropdownOpen }" />
              </button>
              <Transition name="dropdown-slide">
                <div v-if="accountDropdownOpen" class="composer-dropdown__menu">
                  <button
                    v-for="account in accounts"
                    :key="account.id"
                    type="button"
                    class="composer-dropdown__item"
                    :class="{ 'composer-dropdown__item--active': account.id === form.accountId }"
                    @click="selectAccount(account.id)"
                  >
                    <span class="composer-dropdown__item-name">{{ account.name }}</span>
                    <span class="composer-dropdown__item-addr">{{ account.address }}</span>
                  </button>
                </div>
              </Transition>
            </div>

            <button type="button" class="composer-cc-trigger" @click="toggleCc">
              {{ showCc ? '隐藏抄送' : '抄送' }}
            </button>
          </div>

          <div class="composer-body">
            <div class="composer-field">
              <input
                v-model="form.to"
                type="text"
                class="composer-input composer-input--compact"
                placeholder="收件人"
              />
            </div>

            <Transition name="field-collapse">
              <div v-if="showCc" class="composer-field composer-field--cc">
                <input
                  v-model="form.cc"
                  type="text"
                  class="composer-input composer-input--compact composer-field__cc"
                  placeholder="抄送"
                />
              </div>
            </Transition>

            <div class="composer-field">
              <input
                v-model="form.subject"
                type="text"
                class="composer-input composer-input--compact"
                placeholder="邮件主题"
              />
            </div>

            <div class="composer-editor-wrap">
              <VditorEditor
                v-model="form.body"
                mode="ir"
                placeholder="请输入邮件正文"
                height="320px"
                :cache-id="`mail-composer-${form.accountId || 'draft'}`"
                :upload-handler="handleMailVditorUpload"
              />
            </div>
          </div>

          <footer class="composer-footer">
            <label class="composer-check-label">
              <input v-model="form.isHtml" type="checkbox" />
              <span>HTML 模式</span>
            </label>
            <div class="composer-actions">
              <button type="button" class="composer-btn composer-btn--ghost" @click="close">取消</button>
              <button type="button" class="composer-btn composer-btn--primary" :disabled="loading" @click="submit">
                {{ loading ? '发送中...' : '发送' }}
              </button>
            </div>
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
/* Overlay */
.composer-mask {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.32);
  backdrop-filter: blur(10px);
}

/* Modal */
.composer-modal {
  display: flex;
  width: min(860px, calc(100vw - 32px));
  height: min(80vh, 640px);
  max-height: 80vh;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid #dce3ec;
  border-radius: 14px;
  background: #fff;
  box-shadow: 0 28px 80px rgba(15, 23, 42, 0.22);
}

/* Header - sticky on scroll */
.composer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  height: 52px;
  min-height: 52px;
  border-bottom: 1px solid #eef2f7;
  background: #fff;
  flex-shrink: 0;
}

.composer-title {
  margin: 0;
  color: #101828;
  font-size: 15px;
  font-weight: 680;
}

.composer-close-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s ease;
}

.composer-close-btn:hover {
  background: #f1f5f9;
  color: #101828;
}

/* Toolbar */
.composer-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 20px;
  background: #fbfcfe;
  border-bottom: 1px solid #eef2f7;
  flex-shrink: 0;
}

/* Custom Dropdown */
.composer-dropdown {
  position: relative;
  display: inline-block;
}

.composer-dropdown__trigger {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border: 1px solid #e4e7ec;
  border-radius: 8px;
  background: #fff;
  color: #101828;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.composer-dropdown__trigger:hover {
  border-color: #2563eb;
  background: #f4f7ff;
}

.composer-dropdown__value {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.composer-dropdown__arrow--open {
  transform: rotate(180deg);
  transition: transform 0.2s ease;
}

.composer-dropdown__menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  min-width: 220px;
  background: #fff;
  border: 1px solid #e4e7ec;
  border-radius: 10px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.1);
  padding: 6px 0;
  z-index: 100;
}

.composer-dropdown__item {
  display: flex;
  flex-direction: column;
  width: 100%;
  padding: 8px 14px;
  border: none;
  background: transparent;
  text-align: left;
  cursor: pointer;
  transition: all 0.15s ease;
}

.composer-dropdown__item:hover {
  background: #f4f7ff;
}

.composer-dropdown__item--active {
  background: #eff6ff;
}

.composer-dropdown__item--active:hover {
  background: #dbeafe;
}

.composer-dropdown__item-name {
  font-size: 13px;
  font-weight: 600;
  color: #101828;
}

.composer-dropdown__item-addr {
  margin-top: 2px;
  font-size: 11px;
  color: #94a3b8;
}

/* CC toggle button */
.composer-cc-trigger {
  border: none;
  background: transparent;
  padding: 5px 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  color: #2563eb;
  cursor: pointer;
  transition: all 0.2s ease;
}

.composer-cc-trigger:hover {
  background: #eff6ff;
}

/* Body */
.composer-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0;
  padding: 12px 20px;
  overflow-y: auto;
  min-height: 0;
}

.composer-field {
  display: block;
  margin-bottom: 8px;
}

.composer-input {
  width: 100%;
  border: none;
  border-bottom: 1px solid #eef2f7;
  background: transparent;
  padding: 8px 0;
  color: #101828;
  font-size: 13px;
  font-family: inherit;
  transition: border-color 0.2s ease;
}

.composer-input:focus {
  outline: none;
  border-bottom-color: #2563eb;
}

.composer-input::placeholder {
  color: #cbd5e1;
}

.composer-field--cc {
  margin-bottom: 8px;
}

/* Editor wrap */
.composer-editor-wrap {
  margin-top: 12px;
  height: 320px;
  min-height: 320px;
}

.composer-editor-wrap :deep(.vditor) {
  border-radius: 8px;
}

/* Footer */
.composer-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 20px 14px;
  border-top: 1px solid #eef2f7;
  background: #fbfcfe;
  flex-shrink: 0;
}

.composer-check-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #64748b;
  cursor: pointer;
  user-select: none;
}

.composer-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* Unified Buttons */
.composer-btn {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 7px 14px;
  font-size: 13px;
  font-weight: 650;
  cursor: pointer;
  transition: all 0.2s ease;
}

.composer-btn--ghost {
  border-color: #e4e7ec;
  background: #fff;
  color: #475569;
}

.composer-btn--ghost:hover {
  border-color: #2563eb;
  background: #f4f7ff;
  color: #2563eb;
}

.composer-btn--primary {
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #fff;
  box-shadow: 0 8px 20px rgba(37, 99, 235, 0.18);
}

.composer-btn--primary:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 12px 26px rgba(37, 99, 235, 0.26);
}

.composer-btn--primary:disabled {
  cursor: not-allowed;
  opacity: 0.6;
  box-shadow: none;
}

/* Transitions */
.composer-fade-enter-active,
.composer-fade-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}

.composer-fade-enter-from,
.composer-fade-leave-to {
  opacity: 0;
  transform: scale(0.98) translateY(-8px);
}

.dropdown-slide-enter-active,
.dropdown-slide-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.dropdown-slide-enter-from,
.dropdown-slide-leave-to {
  opacity: 0;
  transform: translateY(-4px);
  transform-origin: top center;
}

.field-collapse-enter-active,
.field-collapse-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}

.field-collapse-enter-from,
.field-collapse-leave-to {
  max-height: 0;
  opacity: 0;
  margin-bottom: 0;
}
</style>
