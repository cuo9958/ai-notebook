<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import Vditor from 'vditor'
import 'vditor/dist/index.css'
import 'vditor/dist/js/i18n/zh_CN.js'

export type VditorEditMode = 'wysiwyg' | 'ir' | 'sv'
export type VditorToolbarAction = {
  name: string
  tip: string
  icon: string
  click: () => void
}

type VditorToolbarItem =
  | string
  | {
      name: string
      tip?: string
      tipPosition?: string
      icon?: string
      click?: (event: Event, vditor: unknown) => void
    }

const props = withDefaults(
  defineProps<{
    modelValue: string
    mode?: VditorEditMode
    height?: string | number
    placeholder?: string
    cacheId?: string
    uploadHandler?: (files: File[]) => Promise<string | null> | string | null
    customActions?: VditorToolbarAction[]
  }>(),
  {
    mode: 'wysiwyg',
    height: '100%',
    placeholder: '',
    cacheId: '',
    uploadHandler: undefined,
    customActions: () => [],
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  ready: []
}>()

const rootRef = ref<HTMLDivElement | null>(null)
const editor = shallowRef<Vditor | null>(null)
const internalValue = ref(props.modelValue)
let syncingFromEditor = false

function destroyEditor() {
  editor.value?.destroy()
  editor.value = null
}

async function mountEditor() {
  await nextTick()
  if (!rootRef.value) {
    return
  }

  destroyEditor()
  internalValue.value = props.modelValue

  const toolbar: VditorToolbarItem[] = [
    'emoji',
    'headings',
    'bold',
    'italic',
    'strike',
    'link',
    '|',
    'list',
    'ordered-list',
    'check',
    'outdent',
    'indent',
    '|',
    'quote',
    'line',
    'code',
    'inline-code',
    'insert-before',
    'insert-after',
    '|',
    ...(props.uploadHandler ? ['upload'] : []),
    'table',
    ...props.customActions.map((action) => ({
      name: action.name,
      tip: action.tip,
      tipPosition: 'n',
      icon: action.icon,
      click: () => action.click(),
    })),
    '|',
    'undo',
    'redo',
    '|',
    'fullscreen',
  ]

  editor.value = new Vditor(rootRef.value, {
    cdn: '/vditor',
    lang: 'zh_CN',
    i18n: window.VditorI18n,
    value: props.modelValue,
    mode: props.mode,
    height: props.height,
    placeholder: props.placeholder,
    cache: {
      enable: false,
      id: props.cacheId || undefined,
    },
    toolbar,
    ...(props.uploadHandler
      ? {
          upload: {
            accept: 'image/*',
            fieldName: 'file[]',
            filename: (name: string) => name.replace(/\W/g, ''),
            max: 10 * 1024 * 1024,
            multiple: true,
            url: '',
            handler: async (files: File[]): Promise<null> => {
              try {
                const markdown = await props.uploadHandler?.(files)
                if (markdown) {
                  editor.value?.focus()
                  editor.value?.insertValue(markdown)
                  editor.value?.tip('图片已插入', 1600)
                }
              } catch (err) {
                editor.value?.tip(err instanceof Error ? err.message : '图片上传失败', 2600)
              }
              return null
            },
          },
        }
      : {}),
    input(value: string) {
      syncingFromEditor = true
      internalValue.value = value
      emit('update:modelValue', value)
      void nextTick(() => {
        syncingFromEditor = false
      })
    },
    after() {
      emit('ready')
    },
  })
}

watch(
  () => props.modelValue,
  (value) => {
    if (syncingFromEditor || value === internalValue.value) {
      return
    }

    internalValue.value = value
    editor.value?.setValue(value, true)
  },
)

watch(
  () => props.mode,
  () => {
    void mountEditor()
  },
)

function insertMarkdown(markdown: string) {
  editor.value?.insertValue(markdown)
}

function wrapSelection(prefix: string, suffix: string, placeholder: string) {
  const selected = editor.value?.getSelection() || placeholder
  editor.value?.deleteValue()
  editor.value?.insertValue(`${prefix}${selected}${suffix}`)
}

function focus() {
  editor.value?.focus()
}

function getValue() {
  return editor.value?.getValue() ?? internalValue.value
}

function getSelection() {
  return editor.value?.getSelection() ?? ''
}

onMounted(() => {
  void mountEditor()
})

onBeforeUnmount(() => {
  destroyEditor()
})

defineExpose({
  focus,
  getSelection,
  getValue,
  insertMarkdown,
  wrapSelection,
})
</script>

<template>
  <div ref="rootRef" class="vditor-editor"></div>
</template>

<style scoped>
.vditor-editor {
  width: 100%;
  height: 100%;
  min-height: 0;
}

.vditor-editor :deep(.vditor) {
  height: 100%;
  border-color: var(--line-strong);
  border-radius: 8px;
  overflow: hidden;
}

.vditor-editor :deep(.vditor-toolbar) {
  border-bottom-color: var(--line);
  background: #f8fbff;
}

.vditor-editor :deep(.vditor-reset),
.vditor-editor :deep(.vditor-ir),
.vditor-editor :deep(.vditor-wysiwyg),
.vditor-editor :deep(.vditor-sv) {
  color: var(--ink);
  font-family: var(--font-body);
}
</style>
