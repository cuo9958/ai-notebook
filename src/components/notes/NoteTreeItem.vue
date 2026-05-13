<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronDown, ChevronRight, FileText, Folder } from '@lucide/vue'
import type { NoteTreeNode } from '@/types/note'

const props = defineProps<{
  activePath: string
  depth?: number
  node: NoteTreeNode
}>()

const emit = defineEmits<{
  open: [path: string]
  createFolder: [parentPath: string]
  createNote: [parentPath: string]
  rename: [path: string, currentName: string]
  remove: [path: string, nodeType: NoteTreeNode['nodeType']]
  openMenu: [
    payload: {
      path: string
      name: string
      nodeType: NoteTreeNode['nodeType']
      x: number
      y: number
    },
  ]
}>()

const expanded = ref(true)

const currentDepth = computed(() => props.depth ?? 0)
const isDirectory = computed(() => props.node.nodeType === 'directory')
const isActive = computed(() => props.activePath === props.node.path)

function forwardRename(path: string, currentName: string) {
  emit('rename', path, currentName)
}

function forwardRemove(path: string, nodeType: NoteTreeNode['nodeType']) {
  emit('remove', path, nodeType)
}

function handleContextMenu(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()

  emit('openMenu', {
    path: props.node.path,
    name: props.node.name,
    nodeType: props.node.nodeType,
    x: event.clientX,
    y: event.clientY,
  })
}
</script>

<template>
  <div
    class="tree-item"
    :class="{
      'tree-item--directory': isDirectory,
      'tree-item--file': !isDirectory,
    }"
    :style="{ '--depth': currentDepth }"
  >
    <div
      class="tree-item__row"
      :class="{
        'tree-item__row--active': isActive,
        'tree-item__row--directory': isDirectory,
      }"
      @click="isDirectory ? (expanded = !expanded) : emit('open', node.path)"
      @contextmenu="handleContextMenu"
    >
      <div class="tree-item__label">
        <span v-if="isDirectory" class="tree-item__caret">
          <ChevronDown v-if="expanded" :size="14" :stroke-width="2.2" />
          <ChevronRight v-else :size="14" :stroke-width="2.2" />
        </span>
        <span v-else class="tree-item__caret tree-item__caret--spacer"></span>

        <span class="tree-item__icon" :class="{ 'tree-item__icon--directory': isDirectory }">
          <Folder v-if="isDirectory" :size="17" :stroke-width="2" />
          <FileText v-else :size="17" :stroke-width="2" />
        </span>

        <span class="tree-item__text">{{ node.name }}</span>
      </div>
    </div>

    <div v-if="isDirectory && expanded" class="tree-item__children">
      <div v-if="!node.children?.length" class="tree-item__empty">空目录</div>

      <NoteTreeItem
        v-for="child in node.children"
        :key="child.path"
        :active-path="activePath"
        :depth="currentDepth + 1"
        :node="child"
        @open="emit('open', $event)"
        @create-folder="emit('createFolder', $event)"
        @create-note="emit('createNote', $event)"
        @rename="forwardRename"
        @remove="forwardRemove"
        @open-menu="emit('openMenu', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.tree-item {
  display: flex;
  flex-direction: column;
  gap: 3px;
  cursor: pointer;
}

.tree-item--directory {
  border: 0;
  border-radius: 6px;
  background: transparent;
}

.tree-item--file {
  margin-left: calc(var(--depth, 0) * 12px);
}

.tree-item__row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 9px;
  border-radius: 6px;
}

.tree-item__row:hover {
  background: #f4f7ff;
}

.tree-item__row--directory {
  cursor: pointer;
}

.tree-item__row--active {
  background: #eef4ff;
  color: #1d2939;
}

.tree-item__label {
  display: inline-flex;
  min-width: 0;
  align-items: center;
  gap: 8px;
}

.tree-item__caret {
  display: inline-flex;
  width: 14px;
  flex: 0 0 14px;
  align-items: center;
  justify-content: center;
  color: #98a2b3;
}

.tree-item__row--active .tree-item__caret {
  color: var(--accent);
}

.tree-item__caret--spacer {
  opacity: 0;
}

.tree-item__icon {
  display: inline-flex;
  width: 18px;
  height: 18px;
  align-items: center;
  justify-content: center;
  color: #667085;
}

.tree-item__icon--directory {
  color: var(--accent);
}

.tree-item__row--active .tree-item__icon {
  color: var(--accent);
}

.tree-item__text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 650;
}

.tree-item__row--active .tree-item__text {
  color: #1d4ed8;
}

.tree-item__children {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 0 0 4px;
  border-top: 0;
}

.tree-item__empty {
  margin-left: 30px;
  padding: 6px 9px 0;
  color: #98a2b3;
  font-size: 12px;
}
</style>
