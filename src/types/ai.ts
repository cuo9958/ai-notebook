export type AIProviderVendor =
  | 'qwen'
  | 'deepseek'
  | 'zhipu'
  | 'doubao'
  | 'openai-compatible'

export interface AIProviderConfig {
  id: string
  name: string
  vendor: AIProviderVendor
  apiKey: string
  baseUrl: string
  model: string
  timeoutMs: number
  enabled: boolean
  updatedAt: string
}

export interface AIProviderInput {
  id?: string
  name: string
  vendor: AIProviderVendor
  apiKey: string
  baseUrl: string
  model: string
  timeoutMs: number
  enabled: boolean
}

export interface AIChatMessage {
  role: 'system' | 'user' | 'assistant'
  content: string
}

export interface AIChatPayload {
  providerId: string
  messages: AIChatMessage[]
  temperature?: number
  maxTokens?: number
}

export interface AIChatResult {
  providerId: string
  providerName: string
  vendor: AIProviderVendor
  model: string
  content: string
  raw: unknown
}
