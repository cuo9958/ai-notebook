<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import {
  getImageHostPreset,
  getQiniuRegionOption,
  getQiniuRegionOptions,
} from '@/services/image-host'
import type { ImageHostConfig, ImageHostInput, ImageHostVendor } from '@/types/image-host'

const props = defineProps<{
  modelValue: boolean
  host: ImageHostConfig | null
  loading?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  save: [payload: ImageHostInput]
}>()

const vendorOptions: Array<{ label: string; value: ImageHostVendor }> = [
  { label: '七牛云', value: 'qiniu' },
  { label: '阿里云 OSS', value: 'aliyun-oss' },
]

const qiniuRegionOptions = getQiniuRegionOptions()

const form = reactive<ImageHostInput>({
  id: '',
  name: '',
  vendor: 'qiniu',
  bucket: '',
  region: '',
  endpoint: '',
  accessKeyId: '',
  accessKeySecret: '',
  uploadToken: '',
  cdnUrl: '',
  pathPrefix: '',
  enabled: true,
})

const title = computed(() => (props.host ? '编辑图床' : '新增图床'))
const isQiniu = computed(() => form.vendor === 'qiniu')

function fillFromPreset(vendor: ImageHostVendor) {
  const preset = getImageHostPreset(vendor)

  if (!form.name.trim()) {
    form.name = preset.name
  }

  if (vendor === 'qiniu') {
    const selectedRegion = getQiniuRegionOption(form.region.trim()) ?? getQiniuRegionOption(preset.region)
    form.region = selectedRegion?.value ?? preset.region
    form.endpoint = selectedRegion?.endpoint ?? preset.endpoint
  } else {
    if (!form.region.trim()) {
      form.region = preset.region
    }
    form.endpoint = preset.endpoint
  }

  if (!form.cdnUrl.trim()) {
    form.cdnUrl = preset.cdnUrl
  }
}

function normalizeQiniuRegion() {
  if (form.vendor !== 'qiniu') {
    return
  }

  const selectedRegion = getQiniuRegionOption(form.region.trim())
  if (selectedRegion) {
    form.region = selectedRegion.value
    form.endpoint = selectedRegion.endpoint
    return
  }

  const fallbackRegion = getQiniuRegionOption(getImageHostPreset('qiniu').region)
  if (fallbackRegion) {
    form.region = fallbackRegion.value
    if (!form.endpoint.trim()) {
      form.endpoint = fallbackRegion.endpoint
    }
  }
}

function resetForm() {
  form.id = props.host?.id ?? ''
  form.name = props.host?.name ?? ''
  form.vendor = props.host?.vendor ?? 'qiniu'
  form.bucket = props.host?.bucket ?? ''
  form.region = props.host?.region ?? ''
  form.endpoint = props.host?.endpoint ?? ''
  form.accessKeyId = props.host?.accessKeyId ?? ''
  form.accessKeySecret = props.host?.accessKeySecret ?? ''
  form.uploadToken = props.host?.uploadToken ?? ''
  form.cdnUrl = props.host?.cdnUrl ?? ''
  form.pathPrefix = props.host?.pathPrefix ?? ''
  form.enabled = props.host?.enabled ?? true

  if (!props.host) {
    fillFromPreset(form.vendor)
  } else {
    normalizeQiniuRegion()
  }
}

function close() {
  emit('update:modelValue', false)
}

function handleVendorChange(value: ImageHostVendor) {
  form.vendor = value
  fillFromPreset(value)

  if (value === 'qiniu') {
    form.accessKeyId = ''
    form.accessKeySecret = ''
  } else {
    form.uploadToken = ''
  }
}

function handleQiniuRegionChange(value: string) {
  const selectedRegion = getQiniuRegionOption(value)
  form.region = value

  if (selectedRegion) {
    form.endpoint = selectedRegion.endpoint
  }
}

function submit() {
  emit('save', {
    id: form.id || undefined,
    name: form.name.trim(),
    vendor: form.vendor,
    bucket: form.bucket.trim(),
    region: form.region.trim(),
    endpoint: form.endpoint.trim(),
    accessKeyId: form.accessKeyId.trim(),
    accessKeySecret: form.accessKeySecret.trim(),
    uploadToken: form.uploadToken.trim(),
    cdnUrl: form.cdnUrl.trim(),
    pathPrefix: form.pathPrefix.trim(),
    enabled: form.enabled,
  })
}

watch(
  () => [props.modelValue, props.host] as const,
  ([visible]) => {
    if (visible) {
      resetForm()
    }
  },
  { immediate: true },
)
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="host-mask" @click.self="close">
      <section class="host-modal">
        <header class="host-modal__header">
          <div>
            <p class="host-modal__eyebrow">Image Host</p>
            <h3>{{ title }}</h3>
          </div>
          <button type="button" class="host-modal__close" @click="close">关闭</button>
        </header>

        <div class="host-grid">
          <label class="host-field">
            <span>供应商</span>
            <select
              :value="form.vendor"
              @change="handleVendorChange(($event.target as HTMLSelectElement).value as ImageHostVendor)"
            >
              <option v-for="option in vendorOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </label>

          <label class="host-field">
            <span>名称</span>
            <input v-model="form.name" type="text" placeholder="例如：主图床" />
          </label>

          <label class="host-field">
            <span>空间 / Bucket</span>
            <input v-model="form.bucket" type="text" placeholder="请输入 Bucket 名称" />
          </label>

          <label class="host-field">
            <span>区域</span>
            <select
              v-if="isQiniu"
              :value="form.region"
              @change="handleQiniuRegionChange(($event.target as HTMLSelectElement).value)"
            >
              <option v-for="option in qiniuRegionOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
            <input
              v-else
              v-model="form.region"
              type="text"
              placeholder="例如：oss-cn-hangzhou"
            />
          </label>

          <label class="host-field host-field--wide">
            <span>上传地址 / Endpoint</span>
            <input
              v-model="form.endpoint"
              type="text"
              :placeholder="isQiniu ? '选择区域后自动带出' : '例如：oss-cn-hangzhou.aliyuncs.com'"
            />
          </label>

          <label v-if="isQiniu" class="host-field host-field--wide">
            <span>上传 Token</span>
            <input v-model="form.uploadToken" type="password" placeholder="请输入七牛云上传 Token" />
          </label>

          <template v-else>
            <label class="host-field">
              <span>AccessKey ID</span>
              <input v-model="form.accessKeyId" type="text" placeholder="请输入 AccessKey ID" />
            </label>

            <label class="host-field">
              <span>AccessKey Secret</span>
              <input v-model="form.accessKeySecret" type="password" placeholder="请输入 AccessKey Secret" />
            </label>
          </template>

          <label class="host-field host-field--wide">
            <span>访问域名</span>
            <input v-model="form.cdnUrl" type="text" placeholder="例如：https://cdn.example.com" />
          </label>

          <label class="host-field host-field--wide">
            <span>路径前缀</span>
            <input v-model="form.pathPrefix" type="text" placeholder="例如：notes/images" />
          </label>
        </div>

        <label class="host-check">
          <input v-model="form.enabled" type="checkbox" />
          <span>启用该图床</span>
        </label>

        <footer class="host-modal__footer">
          <button type="button" class="host-btn host-btn--ghost" @click="close">取消</button>
          <button type="button" class="host-btn host-btn--primary" :disabled="loading" @click="submit">
            {{ loading ? '保存中...' : '保存' }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.host-mask {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.28);
  backdrop-filter: blur(10px);
  padding: 20px;
}

.host-modal {
  width: min(760px, 100%);
  max-height: min(820px, calc(100vh - 40px));
  overflow: auto;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: #ffffff;
  padding: 18px;
  box-shadow: 0 28px 80px rgba(15, 23, 42, 0.16);
}

.host-modal__header,
.host-modal__footer,
.host-check {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.host-modal__eyebrow {
  margin: 0 0 6px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.host-modal h3 {
  margin: 0;
  color: #101828;
  font-size: 22px;
  font-weight: 760;
}

.host-modal__close,
.host-btn {
  border: 1px solid #e4e7ec;
  border-radius: 8px;
  background: #ffffff;
  padding: 8px 13px;
  color: #344054;
  font-size: 12px;
  font-weight: 700;
}

.host-btn--primary {
  border-color: transparent;
  background: linear-gradient(135deg, #2563eb, #1d4ed8);
  color: #ffffff;
  box-shadow: 0 12px 26px rgba(37, 99, 235, 0.2);
}

.host-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 16px;
}

.host-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.host-field--wide {
  grid-column: 1 / -1;
}

.host-field span,
.host-check span {
  color: #344054;
  font-size: 13px;
  font-weight: 700;
}

.host-field input,
.host-field select {
  border: 1px solid #e4e7ec;
  border-radius: 8px;
  background: #ffffff;
  padding: 10px 12px;
  color: #101828;
  outline: none;
}

.host-field input:focus,
.host-field select:focus {
  border-color: rgba(37, 99, 235, 0.38);
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.08);
}

.host-check {
  justify-content: flex-start;
  margin-top: 14px;
}

.host-modal__footer {
  margin-top: 18px;
}

@media (max-width: 900px) {
  .host-grid {
    grid-template-columns: 1fr;
  }

  .host-modal__header,
  .host-modal__footer {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
