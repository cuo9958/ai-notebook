import { loggedInvoke } from '@/services/debug'
import type {
  ImageHostConfig,
  ImageHostInput,
  ImageHostVendor,
  ImageUploadResult,
} from '@/types/image-host'

export interface QiniuRegionOption {
  label: string
  value: string
  endpoint: string
}

const HOST_PRESETS: Record<
  ImageHostVendor,
  { name: string; region: string; endpoint: string; cdnUrl: string }
> = {
  qiniu: {
    name: '七牛云',
    region: 'z0',
    endpoint: 'https://upload.qiniup.com',
    cdnUrl: '',
  },
  'aliyun-oss': {
    name: '阿里云 OSS',
    region: 'oss-cn-hangzhou',
    endpoint: 'oss-cn-hangzhou.aliyuncs.com',
    cdnUrl: '',
  },
}

const QINIU_REGION_OPTIONS: QiniuRegionOption[] = [
  { label: '华东-浙江', value: 'z0', endpoint: 'https://up-z0.qiniup.com' },
  { label: '华东-浙江2', value: 'cn-east-2', endpoint: 'https://up-cn-east-2.qiniup.com' },
  { label: '华北-河北', value: 'z1', endpoint: 'https://up-z1.qiniup.com' },
  { label: '华南-广东', value: 'z2', endpoint: 'https://up-z2.qiniup.com' },
  { label: '北美-洛杉矶', value: 'na0', endpoint: 'https://up-na0.qiniup.com' },
  { label: '亚太-新加坡（原东南亚）', value: 'as0', endpoint: 'https://up-as0.qiniup.com' },
  { label: '亚太-河内', value: 'ap-southeast-2', endpoint: 'https://up-ap-southeast-2.qiniup.com' },
  { label: '亚太-胡志明', value: 'ap-southeast-3', endpoint: 'https://up-ap-southeast-3.qiniup.com' },
]

function trimTrailingSlash(value: string) {
  return value.replace(/\/+$/, '')
}

function normalizePrefix(value: string) {
  return value
    .trim()
    .replace(/^\/+/, '')
    .replace(/\/+/g, '/')
    .replace(/\/$/, '')
}

function buildObjectKey(host: ImageHostConfig, fileName: string) {
  const prefix = normalizePrefix(host.pathPrefix)
  const trimmedName = fileName.trim()
  const extensionMatch = trimmedName.match(/\.([a-zA-Z0-9]+)$/)
  const extension = extensionMatch?.[1]?.toLowerCase()
  const uniqueName = `${crypto.randomUUID().replace(/-/g, '')}${extension ? `.${extension}` : ''}`
  return prefix ? `${prefix}/${uniqueName}` : uniqueName
}

function buildPublicUrl(host: ImageHostConfig, key: string) {
  const normalizedCdn = trimTrailingSlash(host.cdnUrl.trim())
  if (normalizedCdn) {
    return `${normalizedCdn}/${key}`
  }

  if (host.vendor === 'aliyun-oss') {
    return `https://${host.bucket}.${host.endpoint}/${key}`
  }

  return `${trimTrailingSlash(host.endpoint)}/${key}`
}

function base64Encode(input: string) {
  return btoa(unescape(encodeURIComponent(input)))
}

async function signWithHmacSha1(secret: string, content: string) {
  const encoder = new TextEncoder()
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-1' },
    false,
    ['sign'],
  )
  const signature = await crypto.subtle.sign('HMAC', key, encoder.encode(content))
  return btoa(String.fromCharCode(...new Uint8Array(signature)))
}

async function uploadToQiniu(host: ImageHostConfig, file: File): Promise<ImageUploadResult> {
  if (!host.uploadToken.trim()) {
    throw new Error('当前七牛云图床缺少上传 Token')
  }

  const key = buildObjectKey(host, file.name)
  const formData = new FormData()
  formData.append('token', host.uploadToken.trim())
  formData.append('key', key)
  formData.append('file', file)

  const response = await fetch(host.endpoint, {
    method: 'POST',
    body: formData,
  })

  if (!response.ok) {
    throw new Error(`七牛云上传失败: ${response.status}`)
  }

  const raw = await response.json()

  return {
    hostId: host.id,
    hostName: host.name,
    vendor: host.vendor,
    key,
    url: buildPublicUrl(host, raw.key ?? key),
    raw,
  }
}

async function uploadToAliyunOss(host: ImageHostConfig, file: File): Promise<ImageUploadResult> {
  if (!host.accessKeyId.trim() || !host.accessKeySecret.trim()) {
    throw new Error('当前阿里云 OSS 图床缺少 AccessKey 配置')
  }

  const key = buildObjectKey(host, file.name)
  const expireAt = new Date(Date.now() + 10 * 60 * 1000).toISOString()
  const policy = base64Encode(
    JSON.stringify({
      expiration: expireAt,
      conditions: [['content-length-range', 0, 20 * 1024 * 1024]],
    }),
  )
  const signature = await signWithHmacSha1(host.accessKeySecret, policy)
  const formData = new FormData()
  formData.append('key', key)
  formData.append('OSSAccessKeyId', host.accessKeyId.trim())
  formData.append('policy', policy)
  formData.append('Signature', signature)
  formData.append('success_action_status', '200')
  formData.append('file', file)

  const response = await fetch(`https://${host.bucket}.${host.endpoint}`, {
    method: 'POST',
    body: formData,
  })

  if (!response.ok) {
    throw new Error(`阿里云 OSS 上传失败: ${response.status}`)
  }

  return {
    hostId: host.id,
    hostName: host.name,
    vendor: host.vendor,
    key,
    url: buildPublicUrl(host, key),
    raw: {
      status: response.status,
    },
  }
}

export function getImageHostPreset(vendor: ImageHostVendor) {
  return HOST_PRESETS[vendor]
}

export function getQiniuRegionOptions() {
  return QINIU_REGION_OPTIONS
}

export function getQiniuRegionOption(regionId: string) {
  return QINIU_REGION_OPTIONS.find((item) => item.value === regionId)
}

export async function listImageHosts(): Promise<ImageHostConfig[]> {
  return loggedInvoke<ImageHostConfig[]>('get_image_hosts')
}

export async function saveImageHost(host: ImageHostInput): Promise<ImageHostConfig[]> {
  return loggedInvoke<ImageHostConfig[]>(
    'save_image_host',
    { host },
    {
      host: {
        ...host,
        accessKeyId: host.accessKeyId ? '***' : '',
        accessKeySecret: host.accessKeySecret ? '***' : '',
        uploadToken: host.uploadToken ? '***' : '',
      },
    },
  )
}

export async function deleteImageHost(hostId: string): Promise<ImageHostConfig[]> {
  return loggedInvoke<ImageHostConfig[]>('delete_image_host', { hostId })
}

export async function uploadImageWithHost(hostId: string, file: File): Promise<ImageUploadResult> {
  const hosts = await listImageHosts()
  const host = hosts.find((item) => item.id === hostId)

  if (!host) {
    throw new Error('未找到对应的图床配置')
  }

  if (!host.enabled) {
    throw new Error('当前图床已被禁用')
  }

  if (host.vendor === 'qiniu') {
    return uploadToQiniu(host, file)
  }

  return uploadToAliyunOss(host, file)
}
