<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import VditorEditor from '@/components/common/VditorEditor.vue'
import type { WritingMaterial, WritingMaterialInput, WritingMaterialKind } from '@/types/writing'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    material?: WritingMaterial | null
    loading?: boolean
  }>(),
  {
    material: null,
    loading: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  save: [payload: WritingMaterialInput]
}>()

const form = reactive({
  kind: 'text' as WritingMaterialKind,
  title: '',
  content: '',
  sourceUrl: '',
})

const canSubmit = computed(() => {
  if (form.kind === 'text') {
    return form.content.trim().length > 0
  }

  return form.sourceUrl.trim().length > 0
})

watch(
  [() => props.modelValue, () => props.material],
  ([visible, material]) => {
    if (!visible) {
      return
    }

    form.kind = (material?.kind as WritingMaterialKind) || 'text'
    form.title = material?.title || ''
    form.content = material?.content || ''
    form.sourceUrl = material?.sourceUrl || ''
  },
)

function close() {
  emit('update:modelValue', false)
}

function submit() {
  if (!canSubmit.value || props.loading) {
    return
  }

  emit('save', {
    kind: form.kind,
    title: form.title.trim(),
    content: form.content.trim(),
    sourceUrl: form.sourceUrl.trim() || undefined,
  })
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="material-mask" @click.self="close">
      <section class="material-modal">
        <div class="material-modal__header">
          <div>
            <p class="material-modal__eyebrow">Material</p>
            <h3>{{ material ? '编辑素材' : '新增素材' }}</h3>
          </div>

          <button type="button" class="material-modal__close" @click="close">
            关闭
          </button>
        </div>

        <div class="material-switch">
          <button
            type="button"
            class="material-switch__item"
            :class="{ 'material-switch__item--active': form.kind === 'text' }"
            @click="form.kind = 'text'"
          >
            文字
          </button>
          <button
            type="button"
            class="material-switch__item"
            :class="{ 'material-switch__item--active': form.kind === 'link' }"
            @click="form.kind = 'link'"
          >
            链接
          </button>
        </div>

        <div class="material-form">
          <label class="material-field">
            <span>标题</span>
            <input
              v-model="form.title"
              type="text"
              :placeholder="form.kind === 'text' ? '可选，不填时自动截取正文前几个字' : '可选，不填时自动抓取网页标题'"
            />
          </label>

          <label v-if="form.kind === 'text'" class="material-field material-field--grow">
            <span>内容</span>
            <VditorEditor
              v-model="form.content"
              class="material-field__vditor"
              mode="ir"
              height="260px"
              placeholder="粘贴你想收集的片段、灵感、采访摘录或参考笔记"
              :cache-id="`writing-material-${material?.id || 'draft'}`"
            />
          </label>

          <label v-else class="material-field material-field--grow">
            <span>链接</span>
            <input
              v-model="form.sourceUrl"
              type="url"
              placeholder="https://example.com/article"
            />
            <small>保存时会尝试抓取网页标题和正文内容。</small>
          </label>
        </div>

        <div class="material-modal__actions">
          <button type="button" class="material-btn material-btn--ghost" @click="close">
            取消
          </button>
          <button
            type="button"
            class="material-btn"
            :disabled="!canSubmit || loading"
            @click="submit"
          >
            {{ loading ? '处理中...' : '保存素材' }}
          </button>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.material-mask {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.28);
  backdrop-filter: blur(10px);
}

.material-modal {
  display: flex;
  width: min(720px, calc(100vw - 40px));
  max-height: min(760px, calc(100vh - 48px));
  flex-direction: column;
  gap: 14px;
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: #ffffff;
  padding: 18px;
  box-shadow: var(--shadow-lg);
}

.material-modal__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.material-modal__eyebrow {
  margin: 0 0 6px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.material-modal h3 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 22px;
  line-height: 1.2;
}

.material-modal__close,
.material-btn,
.material-switch__item {
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: #ffffff;
  color: var(--ink);
}

.material-modal__close {
  padding: 8px 12px;
}

.material-switch {
  display: inline-flex;
  gap: 8px;
  align-self: flex-start;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: rgba(248, 251, 255, 0.9);
  padding: 4px;
}

.material-switch__item {
  border-color: transparent;
  padding: 7px 12px;
  font-size: 12px;
}

.material-switch__item--active {
  background: rgba(37, 99, 235, 0.1);
  color: var(--accent);
}

.material-form {
  display: flex;
  min-height: 0;
  flex-direction: column;
  gap: 14px;
  overflow: auto;
}

.material-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.material-field--grow {
  flex: 1;
}

.material-field span {
  color: var(--ink-soft);
  font-size: 12px;
}

.material-field input {
  width: 100%;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: rgba(248, 251, 255, 0.84);
  padding: 12px 14px;
  color: var(--ink);
}

.material-field__vditor {
  min-height: 260px;
}

.material-field__vditor :deep(.vditor) {
  border-radius: 8px;
}

.material-field small {
  color: var(--ink-soft);
}

.material-modal__actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.material-btn {
  padding: 8px 14px;
  font-size: 12px;
}

.material-btn--ghost {
  background: #ffffff;
}

.material-btn:not(.material-btn--ghost) {
  border-color: transparent;
  background: linear-gradient(135deg, var(--accent), var(--accent-deep));
  color: #ffffff;
}

.material-btn:disabled {
  opacity: 0.56;
  cursor: not-allowed;
}
</style>
