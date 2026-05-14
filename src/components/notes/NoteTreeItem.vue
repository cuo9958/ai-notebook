<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronDown, ChevronRight, FileText, Folder } from '@lucide/vue'
import { VueDraggable } from 'vue-draggable-plus'
import type { MoveEvent, SortableEvent } from 'vue-draggable-plus'
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
  dragStart: [payload: { path: string; nodeType: NoteTreeNode['nodeType'] }]
  move: [payload: { path: string; nodeType: NoteTreeNode['nodeType']; targetParentPath: string | null }]
  refresh: []
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
const dropTargetParentPath = ref<string | null | undefined>(undefined)
const moveHandled = ref(false)

const currentDepth = computed(() => props.depth ?? 0)
const isDirectory = computed(() => props.node.nodeType === 'directory')
const isActive = computed(() => props.activePath === props.node.path)
const childNodes = computed<NoteTreeNode[]>({
  get() {
    if (!props.node.children) {
      props.node.children = []
    }

    return props.node.children
  },
  set(value) {
    props.node.children = value
  },
})
const draggableGroup = {
  name: 'notes-tree',
  pull: true,
  put: true,
}

function elementNode(element?: HTMLElement | null) {
  const path = element?.dataset.notePath
  const nodeType = element?.dataset.noteType as NoteTreeNode['nodeType'] | undefined

  if (!path) {
    return null
  }

  return {
    path,
    nodeType: nodeType ?? 'file',
  }
}

function eventNode(event: SortableEvent) {
  return elementNode(event.item as HTMLElement | undefined)
}

function targetParentPath(event: SortableEvent) {
  return targetParentPathFromElement(event.to as HTMLElement | undefined)
}

function targetParentPathFromElement(element?: HTMLElement | null) {
  return element?.dataset.parentPath || null
}

function rowTargetParentPath(originalEvent?: Event) {
  const target = originalEvent?.target
  if (!(target instanceof HTMLElement)) {
    return undefined
  }

  const row = target.closest<HTMLElement>('.tree-item__row')
  const item = row?.closest<HTMLElement>('.tree-item')
  const path = item?.dataset.notePath
  const nodeType = item?.dataset.noteType as NoteTreeNode['nodeType'] | undefined

  if (!path) {
    return undefined
  }

  if (nodeType === 'directory') {
    return path
  }

  return item?.parentElement?.dataset.parentPath || null
}

function isValidTargetParent(draggedPath: string, nextParentPath: string | null) {
  return !nextParentPath || (nextParentPath !== draggedPath && !nextParentPath.startsWith(`${draggedPath}/`))
}

function handleStart(event: SortableEvent) {
  const node = eventNode(event)
  if (node) {
    emit('dragStart', node)
  }
}

function handleAdd(event: SortableEvent) {
  const node = eventNode(event)
  if (!node) {
    return
  }

  emit('move', {
    path: node.path,
    nodeType: node.nodeType,
    targetParentPath: dropTargetParentPath.value ?? targetParentPath(event),
  })
  moveHandled.value = true
}

function canMove(event: MoveEvent, originalEvent?: Event) {
  const draggedPath = elementNode(event.dragged)?.path
  const rowParentPath = rowTargetParentPath(originalEvent)
  const nextParentPath = rowParentPath ?? targetParentPathFromElement(event.to)

  if (!draggedPath) {
    dropTargetParentPath.value = undefined
    return false
  }

  const allowed = isValidTargetParent(draggedPath, nextParentPath)
  dropTargetParentPath.value = allowed ? rowParentPath : undefined
  return allowed
}

function handleEnd(event: SortableEvent) {
  const node = eventNode(event)
  const sourceParentPath = targetParentPathFromElement(event.from as HTMLElement | undefined)
  const nextParentPath = dropTargetParentPath.value

  if (!moveHandled.value && node && nextParentPath !== undefined && nextParentPath !== sourceParentPath) {
    emit('move', {
      path: node.path,
      nodeType: node.nodeType,
      targetParentPath: nextParentPath,
    })
  } else if (!moveHandled.value && event.from === event.to) {
    emit('refresh')
  }

  dropTargetParentPath.value = undefined
  moveHandled.value = false
}

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
    :data-note-path="node.path"
    :data-note-type="node.nodeType"
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

    <VueDraggable
      v-if="isDirectory && expanded"
      v-model="childNodes"
      class="tree-item__children"
      :animation="150"
      :group="draggableGroup"
      :force-fallback="true"
      :fallback-on-body="true"
      :empty-insert-threshold="18"
      :data-parent-path="node.path"
      @start="handleStart"
      @add="handleAdd"
      @end="handleEnd"
      @move="canMove"
    >
      <div v-if="!childNodes.length" class="tree-item__empty">空目录</div>

      <NoteTreeItem
        v-for="child in childNodes"
        :key="child.path"
        :active-path="activePath"
        :depth="currentDepth + 1"
        :node="child"
        @open="emit('open', $event)"
        @create-folder="emit('createFolder', $event)"
        @create-note="emit('createNote', $event)"
        @drag-start="emit('dragStart', $event)"
        @rename="forwardRename"
        @remove="forwardRemove"
        @open-menu="emit('openMenu', $event)"
        @move="emit('move', $event)"
        @refresh="emit('refresh')"
      />
    </VueDraggable>
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
  min-height: 18px;
  padding: 0 0 4px;
  border-top: 0;
}

.tree-item__empty {
  margin-left: 30px;
  padding: 6px 9px 0;
  color: #98a2b3;
  font-size: 12px;
}

.sortable-ghost > .tree-item__row,
.sortable-chosen > .tree-item__row {
  background: #e0ecff;
  outline: 1px solid #93c5fd;
}
</style>
