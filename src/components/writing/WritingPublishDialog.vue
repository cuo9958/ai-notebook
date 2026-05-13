<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean
  options: Array<{ label: string; value: string }>
  selectedPath: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  select: [value: string]
}>()

function close() {
  emit('update:modelValue', false)
}

function choose(value: string) {
  emit('select', value)
  close()
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="publish-mask" @click.self="close">
      <section class="publish-dialog">
        <div class="publish-dialog__header">
          <div>
            <p class="publish-dialog__eyebrow">Publish</p>
            <h3>选择发布目录</h3>
          </div>

          <button type="button" class="publish-dialog__close" @click="close">
            关闭
          </button>
        </div>

        <div v-if="options.length" class="publish-dialog__list">
          <button
            v-for="option in options"
            :key="option.value"
            type="button"
            class="publish-option"
            :class="{ 'publish-option--active': selectedPath === option.value }"
            @click="choose(option.value)"
          >
            <strong>{{ option.label }}</strong>
            <span>{{ option.value }}</span>
          </button>
        </div>

        <div v-else class="publish-dialog__empty">
          <p>当前还没有可用的笔记目录，请先到笔记页或设置中准备工作区目录。</p>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.publish-mask {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(24, 22, 19, 0.34);
  backdrop-filter: blur(10px);
}

.publish-dialog {
  display: flex;
  width: min(640px, calc(100vw - 40px));
  max-height: min(680px, calc(100vh - 48px));
  flex-direction: column;
  gap: 18px;
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 28px;
  background:
    linear-gradient(180deg, rgba(255, 252, 247, 0.97), rgba(245, 236, 224, 0.95));
  padding: 24px;
  box-shadow: var(--shadow-lg);
}

.publish-dialog__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.publish-dialog__eyebrow {
  margin: 0 0 8px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.publish-dialog h3 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 30px;
  line-height: 1.04;
}

.publish-dialog__close {
  border: 1px solid var(--line-strong);
  border-radius: 999px;
  background: rgba(255, 251, 246, 0.88);
  padding: 10px 14px;
  color: var(--ink);
}

.publish-dialog__list {
  display: flex;
  min-height: 0;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
}

.publish-option {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  border: 1px solid var(--line-strong);
  border-radius: 18px;
  background: rgba(255, 251, 245, 0.9);
  padding: 14px 16px;
  color: var(--ink);
  text-align: left;
}

.publish-option strong {
  font-size: 14px;
}

.publish-option span {
  color: var(--ink-soft);
  font-size: 12px;
  word-break: break-all;
}

.publish-option--active {
  border-color: rgba(180, 62, 36, 0.42);
  background: rgba(180, 62, 36, 0.12);
}

.publish-dialog__empty {
  border: 1px dashed var(--line-strong);
  border-radius: 20px;
  padding: 24px;
  color: var(--ink-soft);
}
</style>
