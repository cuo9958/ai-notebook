import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  createDirectory,
  createNote,
  deleteEntry,
  listNotes,
  readNote,
  renameEntry,
  saveNote,
  updateNoteWorkspaceSettings,
} from '@/services/note'
import type { NoteDocument, NoteTreeNode } from '@/types/note'

function getErrorMessage(error: unknown, fallback: string): string {
  if (typeof error === 'string') {
    return error
  }

  if (error instanceof Error) {
    return error.message
  }

  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message?: unknown }).message
    if (typeof message === 'string') {
      return message
    }
  }

  return fallback
}

function findFirstFile(nodes: NoteTreeNode[]): NoteTreeNode | null {
  for (const node of nodes) {
    if (node.nodeType === 'file') {
      return node
    }

    if (node.children?.length) {
      const child = findFirstFile(node.children)
      if (child) {
        return child
      }
    }
  }

  return null
}

export const useNotesStore = defineStore('notes', () => {
  const loading = ref(false)
  const saving = ref(false)
  const autoSaving = ref(false)
  const autoSaveEnabled = ref(true)
  const tree = ref<NoteTreeNode[]>([])
  const rootPath = ref('')
  const backupRootPath = ref('')
  const backupRetentionDays = ref(7)
  const activeNote = ref<NoteDocument | null>(null)
  const draft = ref('')
  const error = ref('')
  const lastSavedAt = ref<string | null>(null)
  const lastBackupAt = ref<string | null>(null)
  const autoSaveTimer = ref<ReturnType<typeof setTimeout> | null>(null)

  const activePath = computed(() => activeNote.value?.path ?? '')
  const isDirty = computed(() => draft.value !== (activeNote.value?.content ?? ''))

  function syncNoteState(note: NoteDocument | null) {
    activeNote.value = note
    draft.value = note?.content ?? ''
    lastSavedAt.value = note?.updatedAt ?? null
    lastBackupAt.value = note?.lastBackupAt ?? null
  }

  async function refreshTree() {
    const workspace = await listNotes()
    tree.value = workspace.tree
    rootPath.value = workspace.rootPath
    backupRootPath.value = workspace.backupRootPath
    backupRetentionDays.value = workspace.backupRetentionDays
  }

  async function updateWorkspaceRoot(path: string) {
    error.value = ''
    cancelAutoSave()
    const currentActivePath = activeNote.value?.path ?? ''
    const settings = await updateNoteWorkspaceSettings(path)
    rootPath.value = settings.notesRootPath
    await refreshTree()

    const activeExists = currentActivePath
      ? await readNote(currentActivePath)
          .then((note) => {
            syncNoteState(note)
            return true
          })
          .catch(() => false)
      : false

    if (!activeExists) {
      syncNoteState(null)
      const first = findFirstFile(tree.value)
      if (first) {
        await openNote(first.path)
      }
    }
  }

  async function initialize() {
    loading.value = true
    error.value = ''

    try {
      await refreshTree()

      if (!activeNote.value) {
        const first = findFirstFile(tree.value)
        if (first) {
          await openNote(first.path)
        }
      }
    } catch (err) {
      error.value = getErrorMessage(err, '笔记工作区初始化失败')
    } finally {
      loading.value = false
    }
  }

  async function openNote(path: string) {
    cancelAutoSave()
    loading.value = true
    error.value = ''

    try {
      const note = await readNote(path)
      syncNoteState(note)
    } catch (err) {
      error.value = getErrorMessage(err, '打开笔记失败')
    } finally {
      loading.value = false
    }
  }

  async function createFolder(parentPath: string | null, name: string) {
    await createDirectory(parentPath, name)
    await refreshTree()
  }

  async function createNewNote(parentPath: string | null, title: string) {
    const note = await createNote(parentPath, title)
    await refreshTree()
    syncNoteState(note)
  }

  async function saveActiveNote({ silent = false }: { silent?: boolean } = {}) {
    if (!activeNote.value) {
      return
    }

    if (silent && !isDirty.value) {
      return
    }

    cancelAutoSave()
    error.value = ''

    if (silent) {
      autoSaving.value = true
    } else {
      saving.value = true
    }

    try {
      const result = await saveNote(activeNote.value.path, draft.value)
      syncNoteState({
        ...activeNote.value,
        content: draft.value,
        updatedAt: result.updatedAt,
        lastBackupAt: result.lastBackupAt,
      })
      await refreshTree()
    } catch (err) {
      error.value = getErrorMessage(err, '保存笔记失败')
    } finally {
      saving.value = false
      autoSaving.value = false
    }
  }

  async function renameNode(path: string, newName: string) {
    await renameEntry(path, newName)
    const shouldRefreshActive = activeNote.value?.path === path

    await refreshTree()

    if (shouldRefreshActive) {
      await openNote(path).catch(async () => {
        await openNote(path.replace(/[^\\/]+$/, newName.endsWith('.md') ? newName : `${newName}.md`))
      })
    }
  }

  async function removeNode(path: string) {
    cancelAutoSave()
    await deleteEntry(path)

    if (activeNote.value?.path === path) {
      syncNoteState(null)
    }

    await refreshTree()

    if (!activeNote.value) {
      const first = findFirstFile(tree.value)
      if (first) {
        await openNote(first.path)
      }
    }
  }

  function updateDraft(content: string) {
    draft.value = content
  }

  function queueAutoSave(delay = 900) {
    if (!autoSaveEnabled.value || !activeNote.value || !isDirty.value) {
      return
    }

    cancelAutoSave()
    autoSaveTimer.value = setTimeout(() => {
      void saveActiveNote({ silent: true })
    }, delay)
  }

  function cancelAutoSave() {
    if (autoSaveTimer.value) {
      clearTimeout(autoSaveTimer.value)
      autoSaveTimer.value = null
    }
  }

  return {
    activeNote,
    activePath,
    autoSaveEnabled,
    autoSaving,
    backupRetentionDays,
    backupRootPath,
    cancelAutoSave,
    createFolder,
    createNewNote,
    draft,
    error,
    initialize,
    isDirty,
    lastBackupAt,
    lastSavedAt,
    loading,
    openNote,
    queueAutoSave,
    refreshTree,
    removeNode,
    renameNode,
    rootPath,
    saveActiveNote,
    saving,
    syncNoteState,
    tree,
    updateWorkspaceRoot,
    updateDraft,
  }
})
