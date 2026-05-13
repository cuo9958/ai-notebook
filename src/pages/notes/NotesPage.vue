<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { marked } from 'marked'
import hljs from 'highlight.js/lib/core'
import bash from 'highlight.js/lib/languages/bash'
import javascript from 'highlight.js/lib/languages/javascript'
import json from 'highlight.js/lib/languages/json'
import markdown from 'highlight.js/lib/languages/markdown'
import typescript from 'highlight.js/lib/languages/typescript'
import xml from 'highlight.js/lib/languages/xml'
import {
  Brain,
  ChevronDown,
  Download,
  FolderPlus,
  History,
  NotebookPen,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Save,
} from '@lucide/vue'
import mermaid from 'mermaid'
import { storeToRefs } from 'pinia'
import AINoteEditModal from '@/components/ai/AINoteEditModal.vue'
import TextContextMenu from '@/components/common/TextContextMenu.vue'
import VditorEditor from '@/components/common/VditorEditor.vue'
import type { VditorEditMode, VditorToolbarAction } from '@/components/common/VditorEditor.vue'
import BackupHistoryModal from '@/components/notes/BackupHistoryModal.vue'
import NoteAlertDialog from '@/components/notes/NoteAlertDialog.vue'
import NoteTreeItem from '@/components/notes/NoteTreeItem.vue'
import { listImageHosts, uploadImageWithHost } from '@/services/image-host'
import { listBackups, readBackup, restoreBackup, saveNoteImage, syncNotes } from '@/services/note'
import { useNotesStore } from '@/stores/notes'
import type { BackupDocument, BackupHistoryItem, NoteSyncProgress, NoteSyncResult } from '@/types/note'
import type { EChartsOption } from 'echarts'

type EditorMode = 'write' | 'preview'
type VditorEditorExpose = InstanceType<typeof VditorEditor>
type AIEditContext = 'editor-selection' | 'preview-block' | null
type TreeContextMenuItem = {
  key: string
  label: string
  danger?: boolean
  disabled?: boolean
}

hljs.registerLanguage('bash', bash)
hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('json', json)
hljs.registerLanguage('markdown', markdown)
hljs.registerLanguage('typescript', typescript)
hljs.registerLanguage('html', xml)

mermaid.initialize({
  startOnLoad: false,
  securityLevel: 'loose',
  theme: 'neutral',
})

const renderer = new marked.Renderer()
renderer.code = ({ text, lang }) => {
  if (lang === 'mermaid') {
    return `<div class="mermaid-block" data-mermaid>${text}</div>`
  }

  if (lang === 'echarts') {
    return `<div class="echarts-block" data-echarts="${encodeURIComponent(text)}"></div>`
  }

  return `<pre><code class="language-${lang ?? 'plaintext'}">${text}</code></pre>`
}
renderer.image = ({ href, title, text }) => {
  const source = resolvePreviewImageSource(href ?? '')
  const safeAlt = (text ?? '').replace(/"/g, '&quot;')
  const originSource = (href ?? '').replace(/"/g, '&quot;')
  const titleAttr = title ? ` title="${title.replace(/"/g, '&quot;')}"` : ''
  return `<img src="${source}" alt="${safeAlt}" data-origin-src="${originSource}"${titleAttr}>`
}

marked.setOptions({
  breaks: true,
  gfm: true,
  renderer,
})

const store = useNotesStore()
const {
  activeNote,
  activePath,
  draft,
  error,
  isDirty,
  loading,
  saving,
  tree,
} = storeToRefs(store)

const NOTE_VDITOR_MODE_KEY = 'ai-markdown.notes.vditor-mode'
const batchUploadIcon = '<svg viewBox="0 0 24 24"><path d="M12 3v10m0-10 4 4m-4-4-4 4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><path d="M4 14v4a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><path d="M7 10h10" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>'
const chartIcon = '<svg viewBox="0 0 24 24"><path d="M4 19V5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><path d="M4 19h16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><path d="M8 16v-5m4 5V8m4 8v-7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>'
const editorMode = ref<EditorMode>('write')
const vditorMode = ref<VditorEditMode>((localStorage.getItem(NOTE_VDITOR_MODE_KEY) as VditorEditMode) || 'wysiwyg')
const editorRef = ref<VditorEditorExpose | null>(null)
const previewRef = ref<HTMLElement | null>(null)
const sidebarCollapsed = ref(false)
const noteTitleDraft = ref('')
const renamingTitle = ref(false)
const backupModalVisible = ref(false)
const backupLoading = ref(false)
const backupPreviewLoading = ref(false)
const restoringBackup = ref(false)
const backups = ref<BackupHistoryItem[]>([])
const activeBackupPath = ref('')
const backupPreview = ref<BackupDocument | null>(null)
const aiModalVisible = ref(false)
const aiEditContext = ref<AIEditContext>(null)
const aiSourceText = ref('')
const previewAiOriginalText = ref('')
const previewParagraphTarget = ref<HTMLElement | null>(null)
const editorSelectionTimer = ref<ReturnType<typeof setTimeout> | null>(null)
const editorAITrigger = reactive({
  visible: false,
  top: 88,
  left: 20,
})
const previewAITrigger = reactive({
  visible: false,
  top: 24,
  left: 10,
})
const imageUploadingCount = ref(0)
const chartDialogVisible = ref(false)
const chartOptionDraft = ref('')
const chartDialogError = ref('')
const chartInsertRange = reactive({
  start: 0,
  end: 0,
})
const noteEditorActions: VditorToolbarAction[] = [
  {
    name: 'batch-upload-images',
    tip: '批量上传本地图片',
    icon: batchUploadIcon,
    click: uploadAllLocalImages,
  },
  {
    name: 'insert-echarts',
    tip: '插入图表',
    icon: chartIcon,
    click: openChartDialog,
  },
]
const previewImageMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  source: '',
})
const treeContextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  path: '',
  name: '',
  nodeType: 'file' as 'directory' | 'file',
})
const dialogState = reactive({
  visible: false,
  mode: 'prompt' as 'prompt' | 'confirm',
  title: '',
  message: '',
  placeholder: '',
  confirmText: '确认',
  cancelText: '取消',
  initialValue: '',
  danger: false,
  action: '' as 'create-folder' | 'create-note' | 'rename' | 'delete' | '',
  path: null as string | null,
})
const noteSyncing = ref(false)
const noteSyncProgress = ref<NoteSyncProgress | null>(null)
const noteSyncResult = ref<NoteSyncResult | null>(null)
let noteSyncTimer: ReturnType<typeof setInterval> | null = null
let unlistenNoteSyncProgress: (() => void) | null = null

// 导出功能相关状态
const exportDropdownVisible = ref(false)

const previewHtml = computed(() =>
  marked.parse(draft.value, {
    async: false,
    renderer,
  }),
)

const treeContextMenuItems = computed<TreeContextMenuItem[]>(() => {
  if (!treeContextMenu.path) {
    return []
  }

  if (treeContextMenu.nodeType === 'directory') {
    return [
      { key: 'create-note', label: '新建笔记' },
      { key: 'create-folder', label: '新建目录' },
      { key: 'rename', label: '重命名目录' },
      { key: 'delete', label: '删除目录', danger: true },
    ]
  }

  return [
    { key: 'open', label: '打开笔记' },
    { key: 'rename', label: '重命名笔记' },
    { key: 'delete', label: '删除笔记', danger: true },
  ]
})

const noteSyncPercent = computed(() => {
  const progress = noteSyncProgress.value

  if (!noteSyncing.value || !progress?.total) {
    return 0
  }

  return Math.min(100, Math.round((progress.current / progress.total) * 100))
})

const noteSyncTimeText = computed(() => {
  if (noteSyncing.value) {
    return '同步中'
  }

  if (!noteSyncResult.value) {
    return '未同步'
  }

  return formatSyncTime(noteSyncResult.value.lastSyncedAt)
})

const noteSyncResultText = computed(() => {
  if (noteSyncing.value) {
    return noteSyncProgress.value?.message || '正在准备同步'
  }

  const result = noteSyncResult.value

  if (!result) {
    return '等待同步'
  }

  if (error.value && result.message) {
    return result.message
  }

  const parts: string[] = []

  if (result.uploaded > 0) {
    parts.push(`上传 ${result.uploaded} 个`)
  }

  if (result.downloaded > 0) {
    parts.push(`下载 ${result.downloaded} 个`)
  }

  return parts.length ? parts.join('，') : '无内容变更'
})

const previewImageMenuItems = computed<TreeContextMenuItem[]>(() => [
  { key: 'upload', label: '上传' },
])

watch(
  activeNote,
  (note) => {
    noteTitleDraft.value = note?.title ?? ''
    hideEditorAITrigger()
    hidePreviewAITrigger()
    closeAIModal()
  },
  { immediate: true },
)

function highlightPreview(container: Element | null) {
  if (!container) {
    return
  }

  container.querySelectorAll('pre code').forEach((block) => {
    hljs.highlightElement(block as HTMLElement)
  })
}

async function renderMermaidBlocks(container: HTMLElement | null) {
  if (!container) {
    return
  }

  const blocks = Array.from(container.querySelectorAll<HTMLElement>('[data-mermaid]'))

  await Promise.all(
    blocks.map(async (block, index) => {
      const source = block.textContent ?? ''
      const id = `mermaid-${index}-${Date.now()}`

      try {
        const { svg } = await mermaid.render(id, source)
        block.innerHTML = svg
      } catch {
        block.innerHTML = `<pre class="mermaid-error">${source}</pre>`
      }
    }),
  )
}

async function renderEChartsBlocks(container: HTMLElement | null) {
  if (!container) {
    return
  }

  const blocks = Array.from(container.querySelectorAll<HTMLElement>('[data-echarts]'))
  if (!blocks.length) {
    return
  }

  const echarts = await import('echarts')

  for (const block of blocks) {
    const encoded = block.dataset.echarts ?? ''
    let option: EChartsOption

    try {
      option = JSON.parse(decodeURIComponent(encoded)) as EChartsOption
    } catch {
      block.innerHTML = '<pre class="echarts-error">图表配置不是有效的 JSON</pre>'
      continue
    }

    block.innerHTML = ''
    const chart = echarts.init(block)
    chart.setOption(option)
  }
}

watch(
  [draft, editorMode],
  async () => {
    await nextTick()

    if (editorMode.value === 'preview') {
      highlightPreview(previewRef.value)
      await renderMermaidBlocks(previewRef.value)
      await renderEChartsBlocks(previewRef.value)
    }

    store.queueAutoSave()
  },
  { immediate: true },
)

watch(editorMode, () => {
  hideEditorAITrigger()
  hidePreviewAITrigger()
})

watch(vditorMode, (mode) => {
  localStorage.setItem(NOTE_VDITOR_MODE_KEY, mode)
})

function setEditorMode(mode: EditorMode) {
  editorMode.value = mode
}

function setVditorMode(mode: VditorEditMode) {
  vditorMode.value = mode
}

function getFileExtension(file: File) {
  const match = file.type.match(/image\/([a-zA-Z0-9.+-]+)/)
  const rawExtension = match?.[1]?.toLowerCase() ?? 'png'

  if (rawExtension === 'jpeg') {
    return 'jpg'
  }

  if (rawExtension === 'svg+xml') {
    return 'svg'
  }

  return rawExtension
}

function buildClipboardImageName(file: File) {
  const extension = getFileExtension(file)
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
  return `clipboard-${timestamp}.${extension}`
}

function normalizeWindowsPath(value: string) {
  return value.replace(/\//g, '\\')
}

function isLocalImagePath(value: string) {
  return /^[a-zA-Z]:[\\/]/.test(value) || value.startsWith('\\\\')
}

function fileUriToLocalPath(value: string) {
  const withoutPrefix = decodeURIComponent(value.replace(/^file:\/\//, ''))

  if (/^\/[a-zA-Z]:\//.test(withoutPrefix)) {
    return withoutPrefix.slice(1)
  }

  return withoutPrefix
}

function resolvePreviewImageSource(value: string) {
  const trimmed = value.trim().replace(/^<|>$/g, '')

  if (!trimmed) {
    return trimmed
  }

  if (trimmed.startsWith('http://') || trimmed.startsWith('https://') || trimmed.startsWith('data:')) {
    return trimmed
  }

  if (trimmed.startsWith('file://')) {
    return convertFileSrc(normalizeWindowsPath(fileUriToLocalPath(trimmed)))
  }

  if (isLocalImagePath(trimmed)) {
    return convertFileSrc(normalizeWindowsPath(trimmed))
  }

  return trimmed
}

function isLocalMarkdownImageSource(value: string) {
  const trimmed = value.trim().replace(/^<|>$/g, '')

  if (!trimmed) {
    return false
  }

  if (trimmed.startsWith('http://') || trimmed.startsWith('https://') || trimmed.startsWith('data:')) {
    return false
  }

  return trimmed.startsWith('file://') || isLocalImagePath(trimmed)
}

function replaceImageUrl(sourceUrl: string, targetUrl: string) {
  if (!sourceUrl || !targetUrl || sourceUrl === targetUrl) {
    return
  }

  const wrappedSource = `<${sourceUrl}>`
  const withWrappedReplacement = draft.value.replace(wrappedSource, targetUrl)
  const nextDraft = withWrappedReplacement.replace(sourceUrl, targetUrl)

  if (nextDraft !== draft.value) {
    store.updateDraft(nextDraft)
  }
}

async function uploadClipboardImage(file: File) {
  const hosts = await listImageHosts()
  const host = hosts.find((item) => item.enabled)

  if (!host) {
    return null
  }

  const uploadFile = file.name
    ? file
    : new File([file], buildClipboardImageName(file), { type: file.type || 'image/png' })

  return uploadImageWithHost(host.id, uploadFile)
}

async function saveClipboardImageLocally(file: File) {
  if (!activeNote.value) {
    throw new Error('当前没有打开的笔记')
  }

  const fileName = file.name || buildClipboardImageName(file)
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()))
  return saveNoteImage(activeNote.value.path, fileName, bytes)
}

async function fileFromPreviewImageSource(source: string) {
  const resolvedSource = resolvePreviewImageSource(source)
  const response = await fetch(resolvedSource)

  if (!response.ok) {
    throw new Error(`读取图片失败：${response.status}`)
  }

  const blob = await response.blob()
  const normalizedSource = source.trim().replace(/^<|>$/g, '')
  const sourceName = normalizedSource.split('/').pop()?.split('?')[0] || 'image'
  const extensionFromName = sourceName.includes('.') ? sourceName : `${sourceName}.png`
  return new File([blob], extensionFromName, { type: blob.type || 'image/png' })
}

function extractLocalMarkdownImageSources(content: string) {
  const matches = content.matchAll(/!\[[^\]]*]\(([^)\r\n]+)\)/g)
  const sources: string[] = []
  const seen = new Set<string>()

  for (const match of matches) {
    const source = match[1]?.trim()
    if (!source || seen.has(source) || !isLocalMarkdownImageSource(source)) {
      continue
    }

    seen.add(source)
    sources.push(source)
  }

  return sources
}

async function uploadAllLocalImages() {
  if (!activeNote.value) {
    return
  }

  const localSources = extractLocalMarkdownImageSources(draft.value)
  if (!localSources.length) {
    error.value = '当前笔记中没有可上传的本地图片'
    return
  }

  imageUploadingCount.value += localSources.length
  error.value = ''

  try {
    for (const source of localSources) {
      try {
        const file = await fileFromPreviewImageSource(source)
        const uploaded = await uploadClipboardImage(file)

        if (!uploaded?.url) {
          throw new Error('请先在设置中启用一个图床')
        }

        replaceImageUrl(source, uploaded.url)
      } finally {
        imageUploadingCount.value = Math.max(0, imageUploadingCount.value - 1)
      }
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : '批量上传图片失败'
  }
}

async function handleVditorUpload(files: File[]) {
  if (!activeNote.value) {
    return null
  }

  const images = files.filter((file) => file.type.startsWith('image/'))
  if (!images.length) {
    return null
  }

  imageUploadingCount.value += images.length
  error.value = ''

  const markdownItems: string[] = []

  try {
    for (const imageFile of images) {
      try {
        const alt = imageFile.name ? imageFile.name.replace(/\.[^.]+$/, '') : 'image'
        const localImage = await saveClipboardImageLocally(imageFile)
        let imageUrl = `<${localImage.markdownPath}>`

        try {
          const uploaded = await uploadClipboardImage(imageFile)
          if (uploaded?.url) {
            imageUrl = uploaded.url
          }
        } catch (err) {
          error.value = err instanceof Error ? err.message : '图片上传失败，已保留本地图片引用'
        }

        markdownItems.push(`![${alt}](${imageUrl})`)
      } finally {
        imageUploadingCount.value = Math.max(0, imageUploadingCount.value - 1)
      }
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : '图片处理失败'
  }

  return markdownItems.length ? `\n${markdownItems.join('\n')}\n` : null
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value
}

function normalizeSegmentText(text: string) {
  return text.replace(/\r\n/g, '\n').trim()
}

function findPreviewParagraph(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return null
  }

  const candidate = target.closest('p')
  return candidate instanceof HTMLElement ? candidate : null
}

function hideEditorAITrigger() {
  if (editorSelectionTimer.value) {
    clearTimeout(editorSelectionTimer.value)
    editorSelectionTimer.value = null
  }

  editorAITrigger.visible = false
}

function hidePreviewAITrigger() {
  previewParagraphTarget.value?.classList.remove('markdown-paragraph--ai-target')
  previewParagraphTarget.value = null
  previewAITrigger.visible = false
  previewAiOriginalText.value = ''
}

function closeAIModal() {
  aiModalVisible.value = false
  aiEditContext.value = null
  aiSourceText.value = ''
}

function closeTreeContextMenu() {
  treeContextMenu.visible = false
  treeContextMenu.path = ''
  treeContextMenu.name = ''
  treeContextMenu.nodeType = 'file'
}

function closePreviewImageMenu() {
  previewImageMenu.visible = false
  previewImageMenu.source = ''
}

function openTreeContextMenu(payload: {
  path: string
  name: string
  nodeType: 'directory' | 'file'
  x: number
  y: number
}) {
  treeContextMenu.visible = true
  treeContextMenu.x = payload.x
  treeContextMenu.y = payload.y
  treeContextMenu.path = payload.path
  treeContextMenu.name = payload.name
  treeContextMenu.nodeType = payload.nodeType
}

function handlePreviewContextMenu(event: MouseEvent) {
  if (editorMode.value !== 'preview') {
    return
  }

  event.preventDefault()

  const target = event.target
  if (!(target instanceof HTMLElement)) {
    closePreviewImageMenu()
    return
  }

  const image = target.closest('img')
  if (!(image instanceof HTMLImageElement)) {
    closePreviewImageMenu()
    return
  }

  const source = image.dataset.originSrc?.trim() || image.getAttribute('src')?.trim() || image.currentSrc?.trim() || ''
  if (!source) {
    closePreviewImageMenu()
    return
  }

  previewImageMenu.visible = true
  previewImageMenu.x = event.clientX
  previewImageMenu.y = event.clientY
  previewImageMenu.source = source
}

function handleTreeContextMenuSelect(action: string) {
  const path = treeContextMenu.path
  const name = treeContextMenu.name
  const nodeType = treeContextMenu.nodeType

  closeTreeContextMenu()

  if (!path) {
    return
  }

  if (action === 'open' && nodeType === 'file') {
    void store.openNote(path)
    return
  }

  if (action === 'create-note' && nodeType === 'directory') {
    promptCreateNote(path)
    return
  }

  if (action === 'create-folder' && nodeType === 'directory') {
    promptCreateFolder(path)
    return
  }

  if (action === 'rename') {
    promptRename(path, name)
    return
  }

  if (action === 'delete') {
    confirmRemove(path, nodeType)
  }
}

async function handlePreviewImageMenuSelect(action: string) {
  const source = previewImageMenu.source
  closePreviewImageMenu()

  if (action !== 'upload' || !source) {
    return
  }

  imageUploadingCount.value += 1
  error.value = ''

  try {
    const file = await fileFromPreviewImageSource(source)
    const uploaded = await uploadClipboardImage(file)

    if (!uploaded?.url) {
      throw new Error('请先在设置中启用一个图床')
    }

    replaceImageUrl(source, uploaded.url)
  } catch (err) {
    error.value = err instanceof Error ? err.message : '上传图片失败'
  } finally {
    imageUploadingCount.value = Math.max(0, imageUploadingCount.value - 1)
  }
}

function openEditorAI() {
  if (!activeNote.value) {
    return
  }

  const selectedText = editorRef.value?.getSelection() ?? ''

  if (!normalizeSegmentText(selectedText)) {
    hideEditorAITrigger()
    return
  }

  aiSourceText.value = selectedText
  aiEditContext.value = 'editor-selection'
  aiModalVisible.value = true
}

function handlePreviewHover(event: MouseEvent) {
  if (editorMode.value !== 'preview' || aiModalVisible.value) {
    return
  }

  const container = previewRef.value
  if (!container) {
    return
  }

  if (event.target instanceof HTMLElement && event.target.closest('.floating-ai-trigger--preview')) {
    return
  }

  const paragraph = findPreviewParagraph(event.target)

  if (!paragraph || !container.contains(paragraph)) {
    return
  }

  if (previewParagraphTarget.value === paragraph) {
    return
  }

  previewParagraphTarget.value?.classList.remove('markdown-paragraph--ai-target')
  previewParagraphTarget.value = paragraph
  previewParagraphTarget.value.classList.add('markdown-paragraph--ai-target')

  const text = normalizeSegmentText(paragraph.textContent ?? '')

  if (!text) {
    hidePreviewAITrigger()
    return
  }

  previewAiOriginalText.value = text
  previewAITrigger.top = Math.max(paragraph.offsetTop + 2, 16)
  previewAITrigger.left = 10
  previewAITrigger.visible = true
}

function openPreviewAI() {
  if (!previewAiOriginalText.value) {
    return
  }

  aiSourceText.value = previewAiOriginalText.value
  aiEditContext.value = 'preview-block'
  aiModalVisible.value = true
}

function applyAIResult(nextText: string) {
  if (!activeNote.value) {
    return
  }

  if (aiEditContext.value === 'editor-selection') {
    const replacement = nextText.trim()
    editorRef.value?.wrapSelection('', '', replacement)
    hideEditorAITrigger()
    closeAIModal()
    return
  }

  if (aiEditContext.value === 'preview-block') {
    const originalText = previewAiOriginalText.value

    if (!originalText) {
      closeAIModal()
      return
    }

    const nextDraft = draft.value.replace(originalText, nextText.trim())

    if (nextDraft !== draft.value) {
      store.updateDraft(nextDraft)
    }

    hidePreviewAITrigger()
    closeAIModal()
  }
}

async function commitTitleRename() {
  if (!activeNote.value || renamingTitle.value) {
    return
  }

  const nextTitle = noteTitleDraft.value.trim()
  const currentTitle = activeNote.value.title.replace(/\.md$/i, '')

  if (!nextTitle) {
    noteTitleDraft.value = activeNote.value.title
    return
  }

  if (nextTitle === currentTitle) {
    noteTitleDraft.value = activeNote.value.title
    return
  }

  renamingTitle.value = true

  try {
    await store.renameNode(activeNote.value.path, nextTitle)
    noteTitleDraft.value = store.activeNote?.title ?? nextTitle
  } finally {
    renamingTitle.value = false
  }
}

async function openBackupHistory() {
  backupModalVisible.value = true
  backupLoading.value = true

  try {
    backups.value = await listBackups(activeNote.value?.path)
    if (backups.value.length) {
      await previewBackup(backups.value[0].path)
    } else {
      backupPreview.value = null
      activeBackupPath.value = ''
    }
  } finally {
    backupLoading.value = false
  }
}

async function previewBackup(path: string) {
  activeBackupPath.value = path
  backupPreviewLoading.value = true

  try {
    backupPreview.value = await readBackup(path)
  } finally {
    backupPreviewLoading.value = false
  }
}

async function handleRestoreBackup(path: string) {
  restoringBackup.value = true

  try {
    const restored = await restoreBackup(path)
    store.syncNoteState(restored)
    await store.refreshTree()
    editorMode.value = 'write'
    backupModalVisible.value = false
  } finally {
    restoringBackup.value = false
  }
}

function openPromptDialog(options: {
  action: 'create-folder' | 'create-note'
  title: string
  message: string
  placeholder: string
  path: string | null
}) {
  dialogState.visible = true
  dialogState.mode = 'prompt'
  dialogState.title = options.title
  dialogState.message = options.message
  dialogState.placeholder = options.placeholder
  dialogState.confirmText = '创建'
  dialogState.cancelText = '取消'
  dialogState.initialValue = ''
  dialogState.danger = false
  dialogState.action = options.action
  dialogState.path = options.path
}

function promptCreateFolder(parentPath: string | null = null) {
  openPromptDialog({
    action: 'create-folder',
    title: '新建目录',
    message: '在当前位置创建一个新目录，后续可以继续添加子目录和 Markdown 笔记。',
    placeholder: '例如：工作周报',
    path: parentPath,
  })
}

function promptCreateNote(parentPath: string | null = null) {
  openPromptDialog({
    action: 'create-note',
    title: '新建笔记',
    message: '输入名称后会立即创建 Markdown 文件，并自动切换到编辑状态。',
    placeholder: '例如：2026-04 项目复盘',
    path: parentPath,
  })
}

function promptRename(path: string, currentName: string) {
  dialogState.visible = true
  dialogState.mode = 'prompt'
  dialogState.title = '重命名'
  dialogState.message = '输入新的名称后，会立即更新当前目录或笔记在本地工作区中的名称。'
  dialogState.placeholder = '请输入新的名称'
  dialogState.confirmText = '保存名称'
  dialogState.cancelText = '取消'
  dialogState.initialValue = currentName.replace(/\.md$/, '')
  dialogState.danger = false
  dialogState.action = 'rename'
  dialogState.path = path
}

function confirmRemove(path: string, nodeType: 'directory' | 'file') {
  dialogState.visible = true
  dialogState.mode = 'confirm'
  dialogState.title = nodeType === 'directory' ? '删除目录' : '删除笔记'
  dialogState.message =
    nodeType === 'directory'
      ? '删除目录后，其中的子目录和笔记都会一起移除，这个操作不能撤销。'
      : '删除笔记后，当前内容会立即从本地工作区移除，这个操作不能撤销。'
  dialogState.placeholder = ''
  dialogState.confirmText = '确认删除'
  dialogState.cancelText = '取消'
  dialogState.initialValue = ''
  dialogState.danger = true
  dialogState.action = 'delete'
  dialogState.path = path
}

function resetDialog() {
  dialogState.visible = false
  dialogState.mode = 'prompt'
  dialogState.title = ''
  dialogState.message = ''
  dialogState.placeholder = ''
  dialogState.confirmText = '确认'
  dialogState.cancelText = '取消'
  dialogState.initialValue = ''
  dialogState.danger = false
  dialogState.action = ''
  dialogState.path = null
}

function handleDialogConfirm(value: string) {
  if (dialogState.action === 'create-folder' && value) {
    void store.createFolder(dialogState.path, value)
    resetDialog()
    return
  }

  if (dialogState.action === 'create-note' && value) {
    void store.createNewNote(dialogState.path, value)
    setEditorMode('write')
    resetDialog()
    return
  }

  if (dialogState.action === 'rename' && dialogState.path && value) {
    void store.renameNode(dialogState.path, value)
    resetDialog()
    return
  }

  if (dialogState.action === 'delete' && dialogState.path) {
    void store.removeNode(dialogState.path)
  }

  resetDialog()
}

function defaultChartOption() {
  return JSON.stringify(
    {
      title: {
        text: '示例图表',
      },
      tooltip: {},
      xAxis: {
        type: 'category',
        data: ['一月', '二月', '三月', '四月'],
      },
      yAxis: {
        type: 'value',
      },
      series: [
        {
          name: '数量',
          type: 'bar',
          data: [12, 20, 15, 28],
        },
      ],
    },
    null,
    2,
  )
}

function openChartDialog() {
  if (!activeNote.value) {
    return
  }

  const selectedText = editorRef.value?.getSelection()?.trim() ?? ''

  chartDialogError.value = ''

  if (selectedText) {
    chartInsertRange.start = 0
    chartInsertRange.end = 0
    chartOptionDraft.value = selectedText
  } else {
    chartInsertRange.start = 0
    chartInsertRange.end = 0
    chartOptionDraft.value = defaultChartOption()
  }

  chartDialogVisible.value = true
}

function closeChartDialog() {
  chartDialogVisible.value = false
  chartDialogError.value = ''
}

function saveChartDialog() {
  let normalized = ''

  try {
    normalized = JSON.stringify(JSON.parse(chartOptionDraft.value), null, 2)
  } catch {
    chartDialogError.value = '图表数据必须是有效的 ECharts option JSON'
    return
  }

  const snippet = `\n\`\`\`echarts\n${normalized}\n\`\`\`\n`
  editorRef.value?.insertMarkdown(snippet)
  closeChartDialog()
}

function formatSyncTime(value: string) {
  const date = new Date(value)

  if (Number.isNaN(date.getTime())) {
    return '刚刚'
  }

  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

async function runNoteSync() {
  if (noteSyncing.value) {
    return
  }

  noteSyncing.value = true
  noteSyncProgress.value = {
    stage: 'checking',
    current: 0,
    total: 0,
    message: '正在准备同步',
  }
  error.value = ''

  try {
    if (isDirty.value) {
      await store.saveActiveNote({ silent: true })
    }

    const currentPath = activePath.value
    const result = await syncNotes()
    noteSyncResult.value = result
    await store.refreshTree()

    if (currentPath && activePath.value === currentPath && !isDirty.value) {
      await store.openNote(currentPath).catch(() => undefined)
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : '笔记同步失败'
    error.value = message
    noteSyncResult.value = {
      checkedAt: new Date().toISOString(),
      lastSyncedAt: new Date().toISOString(),
      downloadTotal: 0,
      uploadTotal: 0,
      downloaded: 0,
      uploaded: 0,
      skipped: 0,
      message,
    }
  } finally {
    noteSyncing.value = false
  }
}

// 导出功能实现
function toggleExportDropdown() {
  exportDropdownVisible.value = !exportDropdownVisible.value
}

function closeExportDropdown() {
  exportDropdownVisible.value = false
}

async function exportNote(format: 'markdown' | 'html' | 'pdf') {
  if (!activeNote.value) {
    return
  }

  // 先保存当前笔记
  if (isDirty.value) {
    await store.saveActiveNote({ silent: true })
  }

  // 选择导出目录
  const selectedDir = await open({
    directory: true,
    multiple: false,
    title: '选择导出目录',
  })

  if (!selectedDir) {
    return
  }

  try {
    // 使用笔记标题作为文件名,并移除 Windows 文件名中的非法字符
    const noteTitle = (noteTitleDraft.value || activeNote.value.title || activeNote.value.path || '未命名笔记')
      .toString()
      .replace(/\.md$/, '')
      .replace(/[<>:"/\\|?*]/g, '_')  // 替换 Windows 文件名非法字符
    
    // 使用正确的路径分隔符
    let fileName = ''
    let fileContent = ''
    
    if (format === 'markdown') {
      fileName = `${noteTitle}.md`
      fileContent = draft.value
    } else if (format === 'html') {
      fileName = `${noteTitle}.html`
      fileContent = generateHtmlExport()
    } else if (format === 'pdf') {
      fileName = `${noteTitle}.pdf`
      // PDF 导出需要特殊处理
      await exportAsPDF(selectedDir, fileName)
      closeExportDropdown()
      return
    }

    // 使用操作系统正确的路径分隔符
    const separator = selectedDir.includes('\\') ? '\\' : '/'
    const filePath = `${selectedDir}${separator}${fileName}`
    console.log('准备导出文件:', filePath)
    console.log('文件名:', fileName)
    
    await writeTextFile(filePath, fileContent)
    console.log(`${format.toUpperCase()} 导出成功:`, filePath)
    
    // 显示成功提示
    alert(`${format.toUpperCase()} 导出成功!\n文件位置: ${filePath}`)

    closeExportDropdown()
  } catch (error) {
    console.error('导出失败:', error)
    alert(`导出失败: ${error}`)
  }
}

// 导出为真正的 PDF 文件
async function exportAsPDF(_directory: string, fileName: string) {
  let printWindow: Window | null = null
  
  try {
    console.log('开始 PDF 导出流程...')
    console.log('笔记内容长度:', draft.value.length)
    
    // 渲染 Markdown 为 HTML
    const htmlContent = marked.parse(draft.value, {
      async: false,
      renderer,
    })
    
    console.log('渲染后的 HTML 长度:', htmlContent.length)
    
    // 创建一个打印窗口
    printWindow = window.open('', '_blank')
    
    if (!printWindow) {
      throw new Error('无法打开打印窗口,请检查浏览器弹窗拦截设置')
    }
    
    // 写入完整的 HTML 文档
    printWindow.document.write(`
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="UTF-8">
        <title>${fileName}</title>
        <style>
          @page {
            size: A4;
            margin: 20mm;
          }
          
          * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
          }
          
          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, 'Microsoft YaHei', sans-serif;
            line-height: 1.6;
            color: #333;
            padding: 20px;
          }
          
          h1, h2, h3, h4, h5, h6 {
            margin-top: 24px;
            margin-bottom: 16px;
            font-weight: 600;
            line-height: 1.25;
            page-break-after: avoid;
          }
          
          h1 { font-size: 2em; border-bottom: 2px solid #eee; padding-bottom: 0.3em; }
          h2 { font-size: 1.5em; border-bottom: 1px solid #eee; padding-bottom: 0.3em; }
          h3 { font-size: 1.25em; }
          
          p {
            margin-bottom: 16px;
          }
          
          code {
            background-color: #f6f8fa;
            padding: 0.2em 0.4em;
            border-radius: 3px;
            font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
            font-size: 0.85em;
          }
          
          pre {
            background-color: #f6f8fa;
            padding: 16px;
            overflow: auto;
            border-radius: 6px;
            margin-bottom: 16px;
            page-break-inside: avoid;
          }
          
          pre code {
            background-color: transparent;
            padding: 0;
          }
          
          blockquote {
            margin: 0 0 16px 0;
            padding: 0 1em;
            color: #6a737d;
            border-left: 0.25em solid #dfe2e5;
          }
          
          table {
            border-collapse: collapse;
            width: 100%;
            margin: 16px 0;
            page-break-inside: avoid;
          }
          
          table th,
          table td {
            padding: 6px 13px;
            border: 1px solid #dfe2e5;
          }
          
          table tr:nth-child(2n) {
            background-color: #f6f8fa;
          }
          
          img {
            max-width: 100%;
            height: auto;
            page-break-inside: avoid;
          }
          
          ul, ol {
            margin-bottom: 16px;
            padding-left: 2em;
          }
          
          li {
            margin-bottom: 4px;
          }
          
          a {
            color: #0366d6;
            text-decoration: none;
          }
          
          a:hover {
            text-decoration: underline;
          }
          
          hr {
            height: 0.25em;
            padding: 0;
            margin: 24px 0;
            background-color: #e1e4e8;
            border: 0;
          }
          
          @media print {
            body {
              padding: 0;
            }
          }
        </style>
      </head>
      <body>
        ${htmlContent}
      </body>
      </html>
    `)
    
    printWindow.document.close()
    
    console.log('打印窗口已打开,等待加载...')
    
    // 等待内容加载完成
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    console.log('开始打印为 PDF...')
    
    // 调用打印功能
    printWindow.print()
    
    console.log('打印对话框已打开')
    
    // 显示提示信息
    alert('PDF 打印对话框已打开!\n\n请在打印对话框中:\n1. 选择"另存为 PDF"或"Microsoft Print to PDF"\n2. 点击"保存"按钮\n3. 选择保存位置并输入文件名: ' + fileName)
    
  } catch (error) {
    console.error('PDF 导出失败:', error)
    alert(`PDF 导出失败: ${error}`)
    throw error
  } finally {
    // 不立即关闭打印窗口,让用户完成打印操作
    // 如果需要自动关闭,可以延迟关闭
    // if (printWindow) {
    //   setTimeout(() => printWindow.close(), 3000)
    // }
  }
}

function generateHtmlExport(): string {
  const htmlContent = marked.parse(draft.value, {
    async: false,
    renderer,
  })

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${activeNote.value?.title || '导出笔记'}</title>
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
      line-height: 1.6;
      max-width: 900px;
      margin: 0 auto;
      padding: 40px 20px;
      color: #333;
    }
    h1, h2, h3, h4, h5, h6 {
      margin-top: 24px;
      margin-bottom: 16px;
      font-weight: 600;
      line-height: 1.25;
    }
    code {
      background-color: #f6f8fa;
      padding: 0.2em 0.4em;
      border-radius: 3px;
      font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
      font-size: 85%;
    }
    pre {
      background-color: #f6f8fa;
      padding: 16px;
      overflow: auto;
      border-radius: 6px;
    }
    pre code {
      background-color: transparent;
      padding: 0;
    }
    blockquote {
      margin: 0;
      padding: 0 1em;
      color: #6a737d;
      border-left: 0.25em solid #dfe2e5;
    }
    table {
      border-collapse: collapse;
      width: 100%;
      margin: 16px 0;
    }
    table th, table td {
      padding: 6px 13px;
      border: 1px solid #dfe2e5;
    }
    table tr:nth-child(2n) {
      background-color: #f6f8fa;
    }
    img {
      max-width: 100%;
      height: auto;
    }
  </style>
</head>
<body>
${htmlContent}
</body>
</html>`
}

onMounted(() => {
  void listen<NoteSyncProgress>('note-sync-progress', (event) => {
    noteSyncProgress.value = event.payload
  }).then((unlisten) => {
    unlistenNoteSyncProgress = unlisten
  })

  void store.initialize().then(() => {
    void runNoteSync()
  })

  noteSyncTimer = setInterval(() => {
    void runNoteSync()
  }, 10 * 60 * 1000)

  // 添加全局点击监听器，点击外部关闭导出下拉菜单
  document.addEventListener('click', handleGlobalClick)
})

function handleGlobalClick(event: MouseEvent) {
  const target = event.target as HTMLElement
  const exportDropdown = target.closest('.export-dropdown')
  
  if (!exportDropdown && exportDropdownVisible.value) {
    closeExportDropdown()
  }
}

onBeforeUnmount(() => {
  store.cancelAutoSave()
  if (editorSelectionTimer.value) {
    clearTimeout(editorSelectionTimer.value)
  }
  if (noteSyncTimer) {
    clearInterval(noteSyncTimer)
    noteSyncTimer = null
  }
  if (unlistenNoteSyncProgress) {
    unlistenNoteSyncProgress()
    unlistenNoteSyncProgress = null
  }
  // 移除全局点击监听器
  document.removeEventListener('click', handleGlobalClick)
})
</script>

<template>
  <div class="notes-shell" :class="{ 'notes-shell--collapsed': sidebarCollapsed }">
    <TextContextMenu
      v-model="treeContextMenu.visible"
      :x="treeContextMenu.x"
      :y="treeContextMenu.y"
      :items="treeContextMenuItems"
      @select="handleTreeContextMenuSelect"
    />
    <TextContextMenu
      v-model="previewImageMenu.visible"
      :x="previewImageMenu.x"
      :y="previewImageMenu.y"
      :items="previewImageMenuItems"
      @select="handlePreviewImageMenuSelect"
    />

    <NoteAlertDialog
      v-model="dialogState.visible"
      :mode="dialogState.mode"
      :title="dialogState.title"
      :message="dialogState.message"
      :placeholder="dialogState.placeholder"
      :confirm-text="dialogState.confirmText"
      :cancel-text="dialogState.cancelText"
      :initial-value="dialogState.initialValue"
      :danger="dialogState.danger"
      @confirm="handleDialogConfirm"
      @cancel="resetDialog"
    />

    <BackupHistoryModal
      v-model="backupModalVisible"
      :backups="backups"
      :loading="backupLoading"
      :preview="backupPreview"
      :preview-loading="backupPreviewLoading"
      :active-backup-path="activeBackupPath"
      :restoring="restoringBackup"
      @preview="previewBackup"
      @restore="handleRestoreBackup"
    />

    <AINoteEditModal v-model="aiModalVisible" :source-text="aiSourceText" @apply="applyAIResult" />

    <Teleport to="body">
      <div v-if="chartDialogVisible" class="chart-modal-mask" @click.self="closeChartDialog">
        <section class="chart-modal">
          <header class="chart-modal__header">
            <div>
              <p>ECHARTS</p>
              <h3>编辑图表数据</h3>
            </div>
            <button type="button" class="chart-modal__close" @click="closeChartDialog">关闭</button>
          </header>

          <p class="chart-modal__hint">
            使用 ECharts option JSON。保存后会插入为 Markdown 图表代码块，预览模式中自动渲染。
          </p>

          <textarea
            v-model="chartOptionDraft"
            class="chart-modal__textarea"
            spellcheck="false"
            placeholder='{"xAxis":{},"yAxis":{},"series":[]}'
          />

          <p v-if="chartDialogError" class="chart-modal__error">{{ chartDialogError }}</p>

          <footer class="chart-modal__actions">
            <button type="button" class="chart-modal__button chart-modal__button--ghost" @click="closeChartDialog">
              取消
            </button>
            <button type="button" class="chart-modal__button" @click="saveChartDialog">插入图表</button>
          </footer>
        </section>
      </div>
    </Teleport>

    <aside class="notes-sidebar">
      <div class="notes-sidebar__topbar">
        <div v-if="!sidebarCollapsed" class="notes-sidebar__toolbar">
          <button
            type="button"
            class="toolbar-icon-button"
            title="创建目录"
            aria-label="创建目录"
            @click="promptCreateFolder()"
          >
            <FolderPlus :size="16" :stroke-width="2" />
          </button>
          <button
            type="button"
            class="toolbar-icon-button"
            title="新建笔记"
            aria-label="新建笔记"
            @click="promptCreateNote()"
          >
            <NotebookPen :size="16" :stroke-width="2" />
          </button>
          <button
            type="button"
            class="toolbar-icon-button"
            title="历史文档"
            aria-label="历史文档"
            :disabled="!activeNote"
            @click="openBackupHistory"
          >
            <History :size="16" />
          </button>
        </div>

        <button
          type="button"
          class="collapse-button"
          :title="sidebarCollapsed ? '展开笔记列表' : '收起笔记列表'"
          :aria-label="sidebarCollapsed ? '展开笔记列表' : '收起笔记列表'"
          @click="toggleSidebar"
        >
          <PanelLeftOpen v-if="sidebarCollapsed" :size="16" :stroke-width="2" />
          <PanelLeftClose v-else :size="16" :stroke-width="2" />
        </button>
      </div>

      <template v-if="!sidebarCollapsed">
        <div class="notes-tree">
          <p v-if="!tree.length && !loading" class="empty-tip">当前工作区还没有笔记，先创建一篇开始吧。</p>

          <NoteTreeItem
            v-for="node in tree"
            :key="node.path"
            :active-path="activePath"
            :node="node"
            @open="store.openNote"
            @create-folder="promptCreateFolder"
            @create-note="promptCreateNote"
            @rename="promptRename"
            @remove="confirmRemove"
            @open-menu="openTreeContextMenu"
          />
        </div>

        <div class="notes-sync-panel">
          <button
            type="button"
            class="notes-sync-button"
            :class="{ 'notes-sync-button--spinning': noteSyncing }"
            :disabled="noteSyncing"
            title="同步笔记"
            aria-label="同步笔记"
            @click="runNoteSync"
          >
            <RefreshCw :size="16" :stroke-width="2.2" />
          </button>
          <div class="notes-sync-panel__text">
            <strong class="notes-sync-panel__time">{{ noteSyncTimeText }}</strong>
            <span class="notes-sync-panel__result">{{ noteSyncResultText }}</span>
            <div v-if="noteSyncing" class="notes-sync-progress" aria-hidden="true">
              <i :style="{ width: `${noteSyncPercent}%` }" />
            </div>
          </div>
        </div>
      </template>
    </aside>

    <main class="notes-main">
      <header class="notes-header">
        <div class="notes-header__title">
          <input
            v-if="activeNote"
            v-model="noteTitleDraft"
            type="text"
            class="note-title-input"
            :disabled="saving || renamingTitle"
            placeholder="输入笔记标题"
            @blur="commitTitleRename"
            @keydown.enter.prevent="commitTitleRename"
          />
          <span v-else class="note-title-placeholder">未选择笔记</span>
        </div>

        <div class="notes-header__actions">
          <div class="mode-switch" role="tablist" aria-label="Vditor 编辑模式">
            <button
              type="button"
              class="mode-switch__button"
              :class="{ 'mode-switch__button--active': vditorMode === 'wysiwyg' }"
              @click="setVditorMode('wysiwyg')"
            >
              所见即所得
            </button>
            <button
              type="button"
              class="mode-switch__button"
              :class="{ 'mode-switch__button--active': vditorMode === 'ir' }"
              @click="setVditorMode('ir')"
            >
              即时渲染
            </button>
            <button
              type="button"
              class="mode-switch__button"
              :class="{ 'mode-switch__button--active': vditorMode === 'sv' }"
              @click="setVditorMode('sv')"
            >
              分屏预览
            </button>
          </div>

          <span v-if="isDirty" class="dirty-flag">未保存</span>
          
          <div class="export-dropdown" v-if="activeNote">
            <button
              type="button"
              class="export-dropdown__trigger"
              :class="{ 'export-dropdown__trigger--active': exportDropdownVisible }"
              @click="toggleExportDropdown"
            >
              <Download :size="16" :stroke-width="2" />
              <span>导出</span>
              <ChevronDown :size="14" :stroke-width="2" />
            </button>
            
            <div v-if="exportDropdownVisible" class="export-dropdown__menu">
              <button
                type="button"
                class="export-dropdown__item"
                @click="exportNote('markdown')"
              >
                导出为 Markdown
              </button>
              <button
                type="button"
                class="export-dropdown__item"
                @click="exportNote('html')"
              >
                导出为 HTML
              </button>
              <button
                type="button"
                class="export-dropdown__item"
                @click="exportNote('pdf')"
              >
                导出为 PDF
              </button>
            </div>
          </div>
          
          <button
            type="button"
            class="primary-btn"
            :disabled="!activeNote || saving"
            @click="() => store.saveActiveNote()"
          >
            <Save :size="16" :stroke-width="2.2" />
            <span>{{ saving ? '保存中...' : '保存' }}</span>
          </button>
        </div>
      </header>

      <p v-if="error" class="error-banner">{{ error }}</p>

      <section v-if="activeNote" class="workspace-stage">
        <article v-if="editorMode === 'write'" class="panel editor-panel">
          <div class="editor-panel__body">
            <small v-if="imageUploadingCount > 0" class="editor-panel__uploading">图片上传中...</small>
            <button
              v-if="editorAITrigger.visible"
              type="button"
              class="floating-ai-trigger floating-ai-trigger--editor"
              :style="{ top: `${editorAITrigger.top}px`, left: `${editorAITrigger.left}px` }"
              title="AI 编辑当前选中文本"
              aria-label="AI 编辑当前选中文本"
              @mousedown.prevent
              @click="openEditorAI"
            >
              <Brain :size="16" :stroke-width="2.1" />
            </button>

            <VditorEditor
              ref="editorRef"
              v-model="draft"
              class="editor-panel__vditor"
              :mode="vditorMode"
              height="100%"
              placeholder="# 从这里开始记录&#10;&#10;支持 Markdown 表格与 Mermaid 图表"
              :cache-id="activeNote?.path || activePath || 'note-editor'"
              :upload-handler="handleVditorUpload"
              :custom-actions="noteEditorActions"
              @ready="setEditorMode('write')"
            />
          </div>
        </article>

        <article v-else class="panel preview-panel">
          <div
            ref="previewRef"
            class="preview-panel__content"
            @mousemove="handlePreviewHover"
            @contextmenu.capture="handlePreviewContextMenu"
          >
            <button
              v-if="previewAITrigger.visible"
              type="button"
              class="floating-ai-trigger floating-ai-trigger--preview"
              :style="{ top: `${previewAITrigger.top}px`, left: `${previewAITrigger.left}px` }"
              title="AI 编辑当前段落"
              aria-label="AI 编辑当前段落"
              @mousedown.prevent
              @click="openPreviewAI"
            >
              <Brain :size="16" :stroke-width="2.1" />
            </button>

            <div class="preview-panel__rendered markdown-body" v-html="previewHtml" />
          </div>
        </article>
      </section>

      <section v-else class="empty-state">
        <p class="empty-state__badge">Workspace Ready</p>
        <h3>左侧保留笔记树，右侧专注当前笔记的编辑或预览。</h3>
        <p>顶部可以直接修改标题，正文区继续保持简洁的写作与预览切换。</p>
      </section>
    </main>
  </div>
</template>

<style scoped>
/* =========================================
   NOTES PAGE UI - Modern Minimal Blue & White
   ========================================= */

/* 全局变量定义 */
:root {
  --color-primary: #2563eb;
  --color-primary-hover: #1d4ed8;
  --color-primary-light: #eff6ff;
  --color-text-main: #0f172a;
  --color-text-sub: #64748b;
  --color-bg-app: #ffffff;
  --color-bg-sidebar: #ffffff;
  --color-bg-content: #f8fafc;
  --color-bg-hover: #f1f5f9;
  --color-border: #e2e8f0;
  --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --font-primary: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
  --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
}

/* 布局容器：去除多余的间隙，全屏网格 */
.notes-shell {
  display: grid;
  grid-template-columns: 280px 1fr;
  height: 100vh;
  background: var(--color-bg-app);
  transition: grid-template-columns 0.25s ease;
  overflow: hidden;
}

.notes-shell--collapsed {
  grid-template-columns: 60px 1fr;
}

/* 侧边栏：纯白背景，右侧细线分割 */
.notes-sidebar {
  background: var(--color-bg-sidebar);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  transition: all 0.25s ease;
  z-index: 10;
}

/* 顶部工具栏：极简风格 */
.notes-sidebar__topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--color-border);
}

.notes-sidebar__toolbar {
  display: flex;
  gap: 8px;
  flex: 1;
}

/* 通用图标按钮 */
.toolbar-icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--color-text-sub);
  cursor: pointer;
  transition: all 0.2s ease;
}

.toolbar-icon-button:hover {
  background: var(--color-bg-hover);
  color: var(--color-text-main);
  transform: translateY(-1px);
  box-shadow: 0 2px 6px rgba(37, 99, 235, 0.2);
}

/* 特殊强调按钮（新建笔记） */
.toolbar-icon-button:first-child {
  background: var(--color-primary);
  /* color: #fff; */
}

.toolbar-icon-button:first-child:hover {
  background: var(--color-primary-hover);
  /* color: #fff; */
  transform: translateY(-1px);
}

/* 侧边栏折叠按钮 */
.collapse-button {
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: var(--radius-sm);
}

/* 笔记树列表 */
.notes-tree {
  flex: 1;
  overflow-y: auto;
  padding: 12px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

/* 树节点优化（通过样式穿透或直接在组件中调整，这里假设使用 NoteTreeItem） */
.notes-tree::-webkit-scrollbar {
  width: 4px;
}
.notes-tree::-webkit-scrollbar-thumb {
  background: transparent;
  border-radius: 4px;
}
.notes-tree:hover::-webkit-scrollbar-thumb {
  background: #cbd5e1;
}

/* 同步面板：底部吸顶 */
.notes-sync-panel {
  /* 确保横向排列，按钮在左，文字在右 */
  display: flex; 
  flex-direction: row; 
  align-items: center;
  justify-content: flex-start;
  
  /* 调整间距和内边距 */
  gap: 12px;
  padding: 12px 14px;
  margin-top: 8px;
  
  /* 去除多余的背景干扰 */
  background: transparent;
  border-top: 1px solid #e2e8f0; 
  
  /* 让面板紧贴底部 */
  width: 100%;
  box-sizing: border-box;
}
/* 右侧文字区域 */
.notes-sync-panel__text {
  display: flex;
  flex-direction: column; /* 上下两层 */
  justify-content: center;
  flex: 1; /* 占据剩余宽度 */
  overflow: hidden;
}
/* 上方：时间文字 */
.notes-sync-panel__time {
  color: #334155; /* 中等蓝色，作为主标题，不要太深 */
  font-size: 13px;
  font-weight: 600;
  line-height: 1.3;
}

/* 下方：状态描述文字 */
.notes-sync-panel__result {
  color: #94a3b8; /* 浅灰色，作为次要信息，视觉上“隐身” */
  font-size: 11px;
  font-weight: 400;
  /* 防止过长文本溢出 */
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}
/* 主内容区 */
.notes-main {
  background: var(--color-bg-content);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* 顶部标题栏 */
.notes-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  background: #fff;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
  height: 64px;
}

.notes-header__title {
  flex: 1;
}

/* 标题输入框：无边框，大字体，聚焦微动效 */
.note-title-input {
  width: 100%;
  font-size: 20px;
  font-weight: 600;
  color: var(--color-text-main);
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  background: transparent;
  transition: all 0.2s ease;
  font-family: var(--font-primary);
}

.note-title-input::placeholder {
  color: #cbd5e1;
  font-weight: 400;
  font-style: italic;
}

.note-title-input:focus {
  background: var(--color-primary-light);
  border-color: rgba(37, 99, 235, 0.2);
  outline: none;
}

.note-title-placeholder {
  color: #cbd5e1;
  font-size: 20px;
  padding: 0 10px;
}

.notes-header__actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

/* 模式切换开关 */
.mode-switch {
  display: inline-flex;
  background: var(--color-bg-hover);
  padding: 4px;
  border-radius: var(--radius-md);
}

.mode-switch__button {
  border: none;
  background: transparent;
  color: var(--color-text-sub);
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  font-family: var(--font-primary);
}

.mode-switch__button:hover {
  color: var(--color-text-main);
}

.mode-switch__button--active {
  background: #fff;
  color: var(--color-primary);
  font-weight: 600;
  box-shadow: var(--shadow-sm);
}

/* 导出下拉菜单 */
.export-dropdown {
  position: relative;
}

.export-dropdown__trigger {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 8px 12px;
  background: #fff;
  color: var(--color-text-main);
  font-size: 13px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: all 0.2s ease;
}

.export-dropdown__trigger:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
  background: var(--color-primary-light);
}

.export-dropdown__trigger--active {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.export-dropdown__menu {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  width: 180px;
  background: #fff;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md);
  z-index: 1000;
  padding: 6px 0;
}

.export-dropdown__item {
  width: 100%;
  text-align: left;
  padding: 10px 16px;
  border: none;
  background: transparent;
  color: var(--color-text-main);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s;
}

.export-dropdown__item:hover {
  background: var(--color-primary-light);
  color: var(--color-primary);
}

/* 主保存按钮：渐变、阴影 */
.primary-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 18px;
  border: none;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #fff;
  font-weight: 600;
  font-size: 13px;
  box-shadow: 0 4px 12px rgba(37, 99, 235, 0.3);
  cursor: pointer;
  transition: all 0.2s ease;
}

.primary-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(37, 99, 235, 0.4);
}

.primary-btn:disabled {
  background: #94a3b8;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}

/* 工作区域 */
.workspace-stage {
  min-height: 0;
  height: 100%;
  overflow: hidden; /* 关键：限制外部高度，迫使内容在内部滚动 */
}

.panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #fff;
}

/* Vditor 编辑器容器 */
.editor-panel__body {
  flex: 1;
  min-height: 0;
  overflow: hidden; /* 限制高度，不向外溢出 */
}

/* 移除 Vditor 默认的边框，使其无缝嵌入 */
.editor-panel__vditor {
  flex: 1;
  height: 100%;
  
  /* 覆盖原样式的 min-height: 640px，这会导致容器无限撑大 */
  min-height: unset !important; 
  min-height: 0;
}
.editor-panel__vditor :deep(.vditor) {
  height: 100% !important;
  border: none;
  display: flex;
  flex-direction: column;
}

.editor-panel__vditor :deep(.vditor-content) {
  border: none;
  border-radius: 0;
}
.editor-panel__vditor :deep(.vditor-reset),
.editor-panel__vditor :deep(.vditor-ir),
.editor-panel__vditor :deep(.vditor-sv) {
  min-height: 100%;
}

/* 预览区 */
.preview-panel__content {
  height: 100%;
  overflow-y: auto;
  padding: 20px 10%; /* 左右留白增加沉浸感 */
  background: #fff;
  color: var(--color-text-main);
  font-family: var(--font-primary);
}

/* Markdown 内容优化 */
.preview-panel__rendered {
  max-width: 800px;
  margin: 0 auto;
  padding: 24px 0;
}

.preview-panel__rendered :deep(h1),
.preview-panel__rendered :deep(h2),
.preview-panel__rendered :deep(h3) {
  margin-top: 1.5em;
  margin-bottom: 0.75em;
  font-weight: 700;
  text-rendering: optimizeLegibility;
}

.preview-panel__rendered :deep(h1) {
  border-bottom: 1px solid var(--color-border);
  padding-bottom: 0.3em;
  font-size: 2em;
}

.preview-panel__rendered :deep(p) {
  line-height: 1.8;
  margin-bottom: 1.2em;
  color: var(--color-text-main);
}

.preview-panel__rendered :deep(img) {
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-sm);
  margin: 20px auto;
  display: block;
  max-width: 100%;
}

.preview-panel__rendered :deep(blockquote) {
  margin: 1.6em 0;
  padding: 10px 20px;
  border-left: 4px solid var(--color-primary);
  background: var(--color-primary-light);
  border-radius: 0 var(--radius-md) var(--radius-md) 0;
  color: var(--color-text-sub);
}

/* 代码块样式：浅灰色背景 */
.preview-panel__rendered :deep(code) {
  font-family: var(--font-mono);
  background: var(--color-bg-hover);
  color: #d63384;
  padding: 3px 6px;
  border-radius: 4px;
  font-size: 0.9em;
}

.preview-panel__rendered :deep(pre) {
  background: #1e1e2e !important; /* 深色背景代码块，高对比度 */
  padding: 16px;
  border-radius: var(--radius-lg);
  overflow: auto;
}

.preview-panel__rendered :deep(pre code) {
  background: transparent;
  color: #cdd6f4;
  padding: 0;
}

/* 空状态提示 */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--color-text-sub);
}

.empty-state__badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  background: var(--color-bg-hover);
  padding: 6px 14px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 16px;
}

/* 移动端适配 */
@media (max-width: 800px) {
  .notes-shell,
  .notes-shell--collapsed {
    grid-template-columns: 100px 1fr;
  }
  .notes-header {
    padding: 10px 12px;
  }
}
/* =========================================
   1. 统一操作按钮样式 (解决难看、白色、巨大线框问题)
   ========================================= */

/* 针对：同步按钮、收起按钮 */
.action-btn-common,
.notes-sync-button,
.collapse-button {
  /* 统一尺寸与圆角 */
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;

  /* 统一配色：白底、灰框、中性灰文字 */
  background-color: #ffffff;
  border: 1px solid #e2e8f0;
  color: #64748b;

  /* 交互反馈 */
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 悬停状态：淡灰背景 + 边框加深 */
.action-btn-common:hover,
.notes-sync-button:hover,
.collapse-button:hover {
  background-color: #f8fafc;
  border-color: #cbd5e1;
  color: #334155;
  transform: translateY(-1px);
}

/* 同步按钮旋转时的特殊处理 */
.notes-sync-button--spinning svg {
  animation: spin 1s linear infinite;
}

/* 关键修复：强制控制 SVG 图标大小和线宽，解决“巨大线框”问题 */
.notes-sync-button svg,
.collapse-button svg,
.action-btn-common svg {
  width: 18px !important;
  height: 18px !important;
  stroke-width: 2px !important;
  stroke: currentColor !important;
}

/* =========================================
   2. 编辑器渲染模式切换器 (解决无边界感问题)
   ========================================= */

/* 容器：增加深色背景框，形成凹槽感 */
.mode-switch {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background-color: #f1f5f9;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 3px;
}

/* 按钮默认态：透明，融入背景 */
.mode-switch__button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 60px; /* 保证宽度一致 */
  height: 28px;
  border: none;
  padding: 0 10px;

  background: transparent;
  color: #64748b;

  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 悬停未选中态：微白遮罩 */
.mode-switch__button:hover:not(.mode-switch__button--active) {
  color: #475569;
  background: rgba(255, 255, 255, 0.5);
}

/* 选中态：纯白凸起卡片 + 投影 + 蓝色文字 */
.mode-switch__button--active {
  background-color: #ffffff;
  color: #2563eb; /* 蓝色高亮 */
  font-weight: 600;

  /* 核心：阴影带来边界感和厚度感 */
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08), 0 1px 2px rgba(0, 0, 0, 0.04);
  transform: scale(1.01);
}
</style>
