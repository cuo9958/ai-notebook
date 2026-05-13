<script setup lang="ts">
import { computed, ref, watch } from 'vue'

type DialogMode = 'prompt' | 'confirm'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    mode?: DialogMode
    title: string
    message: string
    confirmText?: string
    cancelText?: string
    placeholder?: string
    initialValue?: string
    danger?: boolean
  }>(),
  {
    mode: 'confirm',
    confirmText: '确认',
    cancelText: '取消',
    placeholder: '',
    initialValue: '',
    danger: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  confirm: [value: string]
  cancel: []
}>()

const inputValue = ref(props.initialValue)
const isPrompt = computed(() => props.mode === 'prompt')

watch(
  () => props.modelValue,
  (visible) => {
    if (visible) {
      inputValue.value = props.initialValue
    }
  },
)

function close() {
  emit('update:modelValue', false)
}

function cancel() {
  emit('cancel')
  close()
}

function confirm() {
  emit('confirm', inputValue.value.trim())
  close()
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="dialog-mask" @click.self="cancel">
      <section class="dialog-card" :class="{ 'dialog-card--danger': danger }">
        <p class="dialog-card__eyebrow">{{ danger ? '谨慎操作' : '继续操作' }}</p>
        <h3>{{ title }}</h3>
        <p class="dialog-card__message">{{ message }}</p>

        <input
          v-if="isPrompt"
          v-model="inputValue"
          class="dialog-card__input"
          :placeholder="placeholder"
          @keydown.enter="confirm"
        />

        <div class="dialog-card__actions">
          <button type="button" class="dialog-card__button dialog-card__button--ghost" @click="cancel">
            {{ cancelText }}
          </button>
          <button
            type="button"
            class="dialog-card__button"
            :class="{ 'dialog-card__button--danger': danger }"
            :disabled="isPrompt && !inputValue.trim()"
            @click="confirm"
          >
            {{ confirmText }}
          </button>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-mask {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.28);
  backdrop-filter: blur(10px);
}

.dialog-card {
  width: min(480px, calc(100vw - 32px));
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: #ffffff;
  padding: 20px;
  box-shadow: var(--shadow-lg);
}

.dialog-card__eyebrow {
  margin: 0 0 8px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.dialog-card h3 {
  margin: 0 0 10px;
  font-family: var(--font-display);
  font-size: 22px;
  line-height: 1.2;
}

.dialog-card__message {
  margin: 0;
  color: var(--ink-soft);
  line-height: 1.75;
}

.dialog-card__input {
  width: 100%;
  margin-top: 16px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: rgba(248, 251, 255, 0.84);
  padding: 12px 14px;
  color: var(--ink);
  outline: none;
}

.dialog-card__input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 4px rgba(37, 99, 235, 0.1);
}

.dialog-card__actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 20px;
}

.dialog-card__button {
  border: 1px solid transparent;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--accent), var(--accent-deep));
  padding: 8px 14px;
  color: #ffffff;
  font-size: 12px;
  font-weight: 600;
}

.dialog-card__button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.dialog-card__button--ghost {
  border-color: var(--line-strong);
  background: #ffffff;
  color: var(--ink);
}

.dialog-card__button--danger {
  background: linear-gradient(135deg, #ef4444, #b42318);
}
</style>
