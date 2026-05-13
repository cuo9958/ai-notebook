<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import type { WritingMaterial, WritingOutline } from '@/types/writing'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    outline?: WritingOutline | null
    previousOutline?: WritingOutline | null
    materials?: WritingMaterial[]
  }>(),
  {
    outline: null,
    previousOutline: null,
    materials: () => [],
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  change: [payload: { content: string }]
  save: [payload: { content: string }]
}>()

const textareaRef = ref<HTMLTextAreaElement | null>(null)
const hydrating = ref(false)
const mentionVisible = ref(false)
const mentionQuery = ref('')
const mentionStart = ref(0)

const form = reactive({
  content: '',
})

const canSubmit = computed(() => form.content.trim().length > 0)
const filteredMaterials = computed(() => {
  const query = mentionQuery.value.trim().toLowerCase()
  const options = props.materials.filter((item) => item.title.trim())

  if (!query) {
    return options.slice(0, 8)
  }

  return options
    .filter((item) => item.title.toLowerCase().includes(query))
    .slice(0, 8)
})

watch(
  [() => props.modelValue, () => props.outline],
  async ([visible, outline]) => {
    if (!visible) {
      mentionVisible.value = false
      return
    }

    hydrating.value = true
    form.content = outline?.content || ''
    await nextTick()
    hydrating.value = false
  },
)

watch(
  () => form.content,
  (value) => {
    if (!props.modelValue || hydrating.value) {
      return
    }

    emit('change', {
      content: value,
    })
  },
)

function refreshMentionState() {
  const textarea = textareaRef.value
  if (!textarea) {
    mentionVisible.value = false
    return
  }

  const cursor = textarea.selectionStart ?? 0
  const beforeCursor = form.content.slice(0, cursor)
  const match = beforeCursor.match(/(^|\s)@([^\s@]*)$/)

  if (!match) {
    mentionVisible.value = false
    return
  }

  mentionQuery.value = match[2] ?? ''
  mentionStart.value = cursor - mentionQuery.value.length - 1
  mentionVisible.value = true
}

async function insertMaterialReference(material: WritingMaterial) {
  const textarea = textareaRef.value
  const reference = `@${material.title.trim()} `
  const cursor = textarea?.selectionStart ?? form.content.length
  const before = form.content.slice(0, mentionStart.value)
  const after = form.content.slice(cursor)

  form.content = `${before}${reference}${after}`
  mentionVisible.value = false

  await nextTick()
  textareaRef.value?.focus()
  const nextCursor = before.length + reference.length
  textareaRef.value?.setSelectionRange(nextCursor, nextCursor)
}

function close() {
  mentionVisible.value = false
  emit('update:modelValue', false)
}

function submit() {
  if (!canSubmit.value) {
    return
  }

  mentionVisible.value = false
  emit('save', {
    content: form.content.trim(),
  })
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="outline-mask" @click.self="close">
      <section class="outline-modal">
        <div class="outline-modal__header">
          <div>
            <p class="outline-modal__eyebrow">Outline</p>
            <h3>编辑大纲内容</h3>
          </div>

          <button type="button" class="outline-modal__close" @click="close">关闭</button>
        </div>

        <div v-if="previousOutline?.content?.trim()" class="outline-reference">
          <div class="outline-reference__header">
            <span>上一项大纲参考</span>
          </div>
          <p>{{ previousOutline.content }}</p>
        </div>

        <div class="outline-form">
          <label class="outline-field outline-field--grow">
            <span>内容</span>
            <textarea
              ref="textareaRef"
              v-model="form.content"
              rows="12"
              maxlength="1000"
              placeholder="在这里整理这一项大纲。输入 @ 可以引用顶部素材，正文里只保留素材名称。"
              @input="refreshMentionState"
              @click="refreshMentionState"
              @keyup="refreshMentionState"
            />
          </label>

          <div v-if="mentionVisible" class="mention-list">
            <button
              v-for="material in filteredMaterials"
              :key="material.id"
              type="button"
              class="mention-list__item"
              @mousedown.prevent="insertMaterialReference(material)"
            >
              <strong>{{ material.title }}</strong>
              <span>{{ material.kind === 'link' ? '链接素材' : '文字素材' }}</span>
            </button>
            <p v-if="!filteredMaterials.length" class="mention-list__empty">没有匹配的素材</p>
          </div>
        </div>

        <div class="outline-modal__actions">
          <button type="button" class="outline-btn outline-btn--ghost" @click="close">取消</button>
          <button type="button" class="outline-btn" :disabled="!canSubmit" @click="submit">保存</button>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
:global(.outline-mask) {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background:
    radial-gradient(circle at 50% 20%, rgba(37, 99, 235, 0.16), transparent 34%),
    rgba(15, 23, 42, 0.24) !important;
  backdrop-filter: blur(10px);
}

:global(.outline-modal) {
  display: flex;
  width: min(760px, calc(100vw - 40px));
  max-height: min(780px, calc(100vh - 48px));
  flex-direction: column;
  gap: 14px;
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: linear-gradient(180deg, #ffffff 0%, #f7faff 100%) !important;
  padding: 18px;
  box-shadow: 0 24px 70px rgba(15, 23, 42, 0.16);
}

:global(.outline-modal__header) {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

:global(.outline-modal__eyebrow) {
  margin: 0 0 6px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

:global(.outline-modal h3) {
  margin: 0;
  font-family: var(--font-display);
  font-size: 22px;
  line-height: 1.2;
}

:global(.outline-modal__close),
:global(.outline-btn) {
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: #ffffff !important;
  color: var(--ink);
}

:global(.outline-modal__close) {
  padding: 8px 12px;
}

:global(.outline-reference) {
  display: flex;
  flex-direction: column;
  gap: 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #f6f9ff !important;
  padding: 12px 14px;
}

:global(.outline-reference__header) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

:global(.outline-reference__header span) {
  color: var(--ink-soft);
  font-size: 12px;
}

:global(.outline-reference p) {
  margin: 0;
  max-height: 140px;
  overflow: auto;
  color: var(--ink-soft);
  line-height: 1.75;
  white-space: pre-wrap;
}

:global(.outline-form) {
  position: relative;
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  overflow: visible;
}

:global(.outline-field) {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

:global(.outline-field--grow) {
  flex: 1;
}

:global(.outline-field span) {
  color: var(--ink-soft);
  font-size: 12px;
}

:global(.outline-field textarea) {
  width: 100%;
  min-height: 320px;
  resize: vertical;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: #f8fbff !important;
  padding: 14px 16px;
  color: var(--ink);
  line-height: 1.8;
}

:global(.mention-list) {
  position: absolute;
  left: 0;
  bottom: 12px;
  z-index: 2;
  display: flex;
  width: min(360px, 100%);
  max-height: 220px;
  flex-direction: column;
  gap: 4px;
  overflow: auto;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: #ffffff !important;
  padding: 6px;
  box-shadow: var(--shadow-md);
}

:global(.mention-list__item) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  padding: 8px 10px;
  color: var(--ink);
  text-align: left;
}

:global(.mention-list__item:hover) {
  background: rgba(37, 99, 235, 0.08);
}

:global(.mention-list__item strong) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}

:global(.mention-list__item span),
:global(.mention-list__empty) {
  color: var(--ink-soft);
  font-size: 12px;
}

:global(.mention-list__empty) {
  margin: 0;
  padding: 8px 10px;
}

:global(.outline-modal__actions) {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

:global(.outline-btn) {
  padding: 8px 14px;
  font-size: 12px;
}

:global(.outline-btn--ghost) {
  background: #ffffff !important;
}

:global(.outline-btn:not(.outline-btn--ghost)) {
  border-color: transparent;
  background: linear-gradient(135deg, var(--accent), var(--accent-deep)) !important;
  color: #ffffff;
}

:global(.outline-btn:disabled) {
  opacity: 0.56;
  cursor: not-allowed;
}
</style>
