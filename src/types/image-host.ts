export type ImageHostVendor = 'qiniu' | 'aliyun-oss'

export interface ImageHostConfig {
  id: string
  name: string
  vendor: ImageHostVendor
  bucket: string
  region: string
  endpoint: string
  accessKeyId: string
  accessKeySecret: string
  uploadToken: string
  cdnUrl: string
  pathPrefix: string
  enabled: boolean
  updatedAt: string
}

export interface ImageHostInput {
  id?: string
  name: string
  vendor: ImageHostVendor
  bucket: string
  region: string
  endpoint: string
  accessKeyId: string
  accessKeySecret: string
  uploadToken: string
  cdnUrl: string
  pathPrefix: string
  enabled: boolean
}

export interface ImageUploadResult {
  hostId: string
  hostName: string
  vendor: ImageHostVendor
  key: string
  url: string
  raw: unknown
}
