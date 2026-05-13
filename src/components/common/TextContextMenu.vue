<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

export interface TextContextMenuItem {
  key: string
  label: string
  danger?: boolean
  disabled?: boolean
}

const props = defineProps<{
  modelValue: boolean
  x: number
  y: number
  items: TextContextMenuItem[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  select: [key: string]
}>()

const menuRef = ref<HTMLElement | null>(null)

function close() {
  emit('update:modelValue', false)
}

function handleGlobalPointer(event: MouseEvent) {
  if (!props.modelValue) {
    return
  }

  const target = event.target
  if (target instanceof Node && menuRef.value?.contains(target)) {
    return
  }

  close()
}

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    close()
  }
}

function handleSelect(item: TextContextMenuItem) {
  if (item.disabled) {
    return
  }

  emit('select', item.key)
  close()
}

onMounted(() => {
  window.addEventListener('mousedown', handleGlobalPointer)
  window.addEventListener('keydown', handleEscape)
  window.addEventListener('blur', close)
})

onBeforeUnmount(() => {
  window.removeEventListener('mousedown', handleGlobalPointer)
  window.removeEventListener('keydown', handleEscape)
  window.removeEventListener('blur', close)
})

watch(
  () => props.modelValue,
  (visible) => {
    if (!visible) {
      return
    }

    requestAnimationFrame(() => {
      const menu = menuRef.value
      if (!menu) {
        return
      }

      const padding = 12
      const rect = menu.getBoundingClientRect()
      const nextLeft = Math.min(props.x, window.innerWidth - rect.width - padding)
      const nextTop = Math.min(props.y, window.innerHeight - rect.height - padding)

      menu.style.left = `${Math.max(padding, nextLeft)}px`
      menu.style.top = `${Math.max(padding, nextTop)}px`
    })
  },
)
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" ref="menuRef" class="text-context-menu" :style="{ left: `${x}px`, top: `${y}px` }">
      <button
        v-for="item in items"
        :key="item.key"
        type="button"
        class="text-context-menu__item"
        :class="{
          'text-context-menu__item--danger': item.danger,
          'text-context-menu__item--disabled': item.disabled,
        }"
        :disabled="item.disabled"
        @click="handleSelect(item)"
      >
        {{ item.label }}
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.text-context-menu {
  position: fixed;
  z-index: 120;
  display: flex;
  min-width: 168px;
  flex-direction: column;
  gap: 4px;
  border: 1px solid var(--line-strong);
  border-radius: 16px;
  background: rgba(255, 251, 245, 0.98);
  padding: 8px;
  box-shadow: 0 18px 40px rgba(24, 22, 19, 0.18);
  backdrop-filter: blur(14px);
}

.text-context-menu__item {
  border: none;
  border-radius: 12px;
  background: transparent;
  padding: 10px 12px;
  color: var(--ink);
  text-align: left;
  font-size: 13px;
  font-weight: 600;
}

.text-context-menu__item:hover {
  background: rgba(24, 22, 19, 0.06);
}

.text-context-menu__item--danger {
  color: var(--accent-deep);
}

.text-context-menu__item--disabled {
  opacity: 0.45;
}
</style>
