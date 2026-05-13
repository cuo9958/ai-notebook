<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { listAIProviders, invokeAIChat } from '@/services/ai'
import type { AIProviderConfig } from '@/types/ai'

const props = defineProps<{
  modelValue: boolean
  sourceText: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  apply: [value: string]
}>()

const providers = ref<AIProviderConfig[]>([])
const running = ref(false)
const error = ref('')
const result = ref('')

const form = reactive({
  providerId: '',
  action: 'polish' as 'polish' | 'expand' | 'shorten' | 'custom',
  customPrompt: '',
  improvementAdvice: '',
})

const actionLabel = computed(() => {
  switch (form.action) {
    case 'polish':
      return '润色'
    case 'expand':
      return '扩写'
    case 'shorten':
      return '精简'
    case 'custom':
      return '自定义处理'
  }
})

const adviceText = computed(() => form.improvementAdvice.trim() || '无')

function close() {
  emit('update:modelValue', false)
}

function buildActionText() {
  switch (form.action) {
    case 'polish':
      return '润色'
    case 'expand':
      return '扩写'
    case 'shorten':
      return '精简'
    case 'custom':
      return form.customPrompt.trim() || '自定义处理'
  }
}

function buildUserPrompt() {
  const actionText = buildActionText()
  return [
    `根据以下内容结合优化建议进行${actionText}。`,
    '',
    '原文：',
    props.sourceText.trim(),
    '',
    '意见：',
    adviceText.value,
  ].join('\n')
}

async function runAI() {
  error.value = ''
  result.value = ''

  if (!form.providerId) {
    error.value = '请选择一个 AI 供应商'
    return
  }

  if (!props.sourceText.trim()) {
    error.value = '当前没有可编辑的文本'
    return
  }

  if (form.action === 'custom' && !form.customPrompt.trim()) {
    error.value = '请输入自定义处理方式'
    return
  }

  running.value = true

  try {
    const response = await invokeAIChat({
      providerId: form.providerId,
      messages: [
        {
          role: 'system',
          content: '你是一名中文写作助手。只返回最终改写后的正文内容，不要附加解释、标题、列表说明或代码块。',
        },
        {
          role: 'user',
          content: buildUserPrompt(),
        },
      ],
      temperature: 0.7,
    })

    result.value = response.content.trim()
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'AI 处理失败'
  } finally {
    running.value = false
  }
}

async function initializeProviders() {
  providers.value = (await listAIProviders()).filter((item) => item.enabled)
  form.providerId = providers.value[0]?.id ?? ''
  form.action = 'polish'
  form.customPrompt = ''
  form.improvementAdvice = ''
  error.value = ''
  result.value = ''
}

function applyResult() {
  if (!result.value.trim()) {
    return
  }

  emit('apply', result.value.trim())
  close()
}

watch(
  () => props.modelValue,
  (visible) => {
    if (!visible) {
      return
    }

    void initializeProviders()
  },
)
</script>

<template>
  <div v-if="modelValue" class="ai-note-mask" @click.self="close">
    <section class="ai-note-modal">
      <header class="ai-note-modal__header">
        <div>
          <p class="ai-note-modal__eyebrow">AI Edit</p>
          <h3>段落 AI 编辑</h3>
        </div>
        <button type="button" class="ai-note-btn ai-note-btn--ghost" @click="close">关闭</button>
      </header>

      <div class="ai-note-grid">
        <label class="ai-note-field">
          <span>供应商</span>
          <select v-model="form.providerId">
            <option value="" disabled>请选择供应商</option>
            <option v-for="provider in providers" :key="provider.id" :value="provider.id">
              {{ provider.name }}
            </option>
          </select>
        </label>

        <label class="ai-note-field">
          <span>处理方式</span>
          <select v-model="form.action">
            <option value="polish">润色</option>
            <option value="expand">扩写</option>
            <option value="shorten">精简</option>
            <option value="custom">自定义</option>
          </select>
        </label>

        <label v-if="form.action === 'custom'" class="ai-note-field ai-note-field--wide">
          <span>自定义处理方式</span>
          <input v-model="form.customPrompt" type="text" placeholder="例如：改成更正式的汇报语气" />
        </label>

        <label class="ai-note-field ai-note-field--wide">
          <span>优化建议</span>
          <textarea
            v-model="form.improvementAdvice"
            rows="3"
            placeholder="例如：保留核心观点，语气更专业一些，结尾更简洁"
          />
        </label>

        <label class="ai-note-field ai-note-field--wide">
          <span>原文</span>
          <textarea :value="sourceText" rows="6" readonly />
        </label>

        <label class="ai-note-field ai-note-field--wide">
          <span>结果（{{ actionLabel }}）</span>
          <textarea :value="result" rows="8" readonly placeholder="点击“开始处理”后显示 AI 结果" />
        </label>
      </div>

      <p v-if="error" class="ai-note-message ai-note-message--error">{{ error }}</p>

      <footer class="ai-note-modal__footer">
        <button type="button" class="ai-note-btn ai-note-btn--ghost" :disabled="running" @click="runAI">
          {{ running ? '处理中...' : '开始处理' }}
        </button>
        <button type="button" class="ai-note-btn ai-note-btn--primary" :disabled="!result.trim()" @click="applyResult">
          替换原文
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.ai-note-mask {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(28, 21, 16, 0.34);
  backdrop-filter: blur(10px);
  padding: 20px;
}

.ai-note-modal {
  width: min(860px, 100%);
  border: 1px solid var(--line);
  border-radius: 24px;
  background: rgba(255, 251, 245, 0.96);
  padding: 18px;
  box-shadow: var(--shadow-lg);
}

.ai-note-modal__header,
.ai-note-modal__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.ai-note-modal__eyebrow {
  margin: 0 0 6px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.ai-note-modal h3 {
  margin: 0;
  color: var(--ink);
  font-family: var(--font-display);
  font-size: 26px;
}

.ai-note-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 16px;
}

.ai-note-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ai-note-field--wide {
  grid-column: 1 / -1;
}

.ai-note-field span {
  color: var(--ink);
  font-size: 13px;
  font-weight: 600;
}

.ai-note-field input,
.ai-note-field select,
.ai-note-field textarea {
  border: 1px solid var(--line);
  border-radius: 14px;
  background: rgba(255, 251, 245, 0.88);
  padding: 12px 14px;
  color: var(--ink);
  outline: none;
}

.ai-note-field textarea {
  resize: vertical;
}

.ai-note-btn {
  border: 1px solid transparent;
  border-radius: 999px;
  padding: 10px 14px;
  font-size: 12px;
  font-weight: 600;
}

.ai-note-btn--ghost {
  border-color: var(--line);
  background: rgba(255, 251, 245, 0.82);
  color: var(--ink);
}

.ai-note-btn--primary {
  background: rgba(24, 22, 19, 0.92);
  color: #fff7ef;
}

.ai-note-message {
  margin: 12px 0 0;
  border-radius: 14px;
  padding: 10px 12px;
  font-size: 13px;
}

.ai-note-message--error {
  border: 1px solid rgba(180, 62, 36, 0.16);
  background: rgba(180, 62, 36, 0.08);
  color: var(--accent-deep);
}

.ai-note-modal__footer {
  margin-top: 18px;
}

@media (max-width: 900px) {
  .ai-note-grid {
    grid-template-columns: 1fr;
  }

  .ai-note-modal__header,
  .ai-note-modal__footer {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
