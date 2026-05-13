<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import type { AIProviderConfig, AIProviderInput, AIProviderVendor } from '@/types/ai'
import { getAIProviderPreset } from '@/services/ai'

const props = defineProps<{
  modelValue: boolean
  provider: AIProviderConfig | null
  loading?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  save: [payload: AIProviderInput]
}>()

const vendorOptions: Array<{ label: string; value: AIProviderVendor }> = [
  { label: '通义千问', value: 'qwen' },
  { label: 'DeepSeek', value: 'deepseek' },
  { label: '智谱', value: 'zhipu' },
  { label: '豆包', value: 'doubao' },
  { label: 'OpenAI Compatible', value: 'openai-compatible' },
]

const form = reactive<AIProviderInput>({
  id: '',
  name: '',
  vendor: 'qwen',
  apiKey: '',
  baseUrl: '',
  model: '',
  timeoutMs: 30000,
  enabled: true,
})

const title = computed(() => (props.provider ? '编辑 AI 供应商' : '新增 AI 供应商'))

function fillFromPreset(vendor: AIProviderVendor) {
  const preset = getAIProviderPreset(vendor)
  if (!form.name.trim()) {
    form.name = preset.name
  }
  form.baseUrl = preset.baseUrl
  form.model = preset.model
}

function resetForm() {
  form.id = props.provider?.id ?? ''
  form.name = props.provider?.name ?? ''
  form.vendor = props.provider?.vendor ?? 'qwen'
  form.apiKey = props.provider?.apiKey ?? ''
  form.baseUrl = props.provider?.baseUrl ?? ''
  form.model = props.provider?.model ?? ''
  form.timeoutMs = props.provider?.timeoutMs ?? 30000
  form.enabled = props.provider?.enabled ?? true

  if (!props.provider) {
    fillFromPreset(form.vendor)
  }
}

function close() {
  emit('update:modelValue', false)
}

function handleVendorChange(value: AIProviderVendor) {
  form.vendor = value
  fillFromPreset(value)
}

function submit() {
  emit('save', {
    id: form.id || undefined,
    name: form.name.trim(),
    vendor: form.vendor,
    apiKey: form.apiKey.trim(),
    baseUrl: form.baseUrl.trim(),
    model: form.model.trim(),
    timeoutMs: Number(form.timeoutMs),
    enabled: form.enabled,
  })
}

watch(
  () => [props.modelValue, props.provider] as const,
  ([visible]) => {
    if (visible) {
      resetForm()
    }
  },
  { immediate: true },
)
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="provider-mask" @click.self="close">
      <section class="provider-modal">
        <header class="provider-modal__header">
          <div>
            <p class="provider-modal__eyebrow">AI Gateway</p>
            <h3>{{ title }}</h3>
          </div>
          <button type="button" class="provider-modal__close" @click="close">关闭</button>
        </header>

        <div class="provider-grid">
          <label class="provider-field">
            <span>供应商类型</span>
            <select :value="form.vendor" @change="handleVendorChange(($event.target as HTMLSelectElement).value as AIProviderVendor)">
              <option v-for="option in vendorOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="provider-field">
            <span>自定义名称</span>
            <input v-model="form.name" type="text" placeholder="例如：工作区千问" />
          </label>

          <label class="provider-field provider-field--wide">
            <span>API Key</span>
            <input v-model="form.apiKey" type="password" placeholder="请输入 API Key" />
          </label>

          <label class="provider-field provider-field--wide">
            <span>Base URL</span>
            <input v-model="form.baseUrl" type="text" placeholder="https://example.com" />
          </label>

          <label class="provider-field">
            <span>默认模型</span>
            <input v-model="form.model" type="text" placeholder="请输入模型名称" />
          </label>

          <label class="provider-field">
            <span>超时时间（毫秒）</span>
            <input v-model.number="form.timeoutMs" type="number" min="3000" step="1000" />
          </label>
        </div>

        <label class="provider-check">
          <input v-model="form.enabled" type="checkbox" />
          <span>启用该供应商</span>
        </label>

        <footer class="provider-modal__footer">
          <button type="button" class="provider-btn provider-btn--ghost" @click="close">取消</button>
          <button type="button" class="provider-btn provider-btn--primary" :disabled="loading" @click="submit">
            {{ loading ? '保存中...' : '保存' }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.provider-mask {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.28);
  backdrop-filter: blur(10px);
  padding: 20px;
}

.provider-modal {
  width: min(760px, 100%);
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: #ffffff;
  padding: 18px;
  box-shadow: 0 28px 80px rgba(15, 23, 42, 0.16);
}

.provider-modal__header,
.provider-modal__footer,
.provider-check {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.provider-modal__eyebrow {
  margin: 0 0 6px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.provider-modal h3 {
  margin: 0;
  color: #101828;
  font-size: 22px;
  font-weight: 760;
}

.provider-modal__close,
.provider-btn {
  border: 1px solid #e4e7ec;
  border-radius: 8px;
  background: #ffffff;
  padding: 8px 13px;
  color: #344054;
  font-size: 12px;
  font-weight: 700;
}

.provider-btn--primary {
  border-color: transparent;
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #ffffff;
  box-shadow: 0 12px 26px rgba(37, 99, 235, 0.2);
}

.provider-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 16px;
}

.provider-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.provider-field--wide {
  grid-column: 1 / -1;
}

.provider-field span,
.provider-check span {
  color: #344054;
  font-size: 13px;
  font-weight: 700;
}

.provider-field input,
.provider-field select {
  border: 1px solid #e4e7ec;
  border-radius: 8px;
  background: #ffffff;
  padding: 10px 12px;
  color: #101828;
  outline: none;
}

.provider-field input:focus,
.provider-field select:focus {
  border-color: rgba(37, 99, 235, 0.38);
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.08);
}

.provider-check {
  justify-content: flex-start;
  margin-top: 14px;
}

.provider-modal__footer {
  margin-top: 18px;
}

@media (max-width: 900px) {
  .provider-grid {
    grid-template-columns: 1fr;
  }

  .provider-modal__header,
  .provider-modal__footer {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
