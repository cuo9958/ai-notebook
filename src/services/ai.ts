import axios from 'axios'
import { fetch as httpFetch } from '@tauri-apps/plugin-http'
import { invoke } from '@tauri-apps/api/core'
import { loggedInvoke, logFrontendAction } from '@/services/debug'
import type {
  AIChatPayload,
  AIChatResult,
  AIProviderConfig,
  AIProviderInput,
  AIProviderVendor,
} from '@/types/ai'

const LEGACY_AI_PROVIDER_STORAGE_KEY = 'ai-markdown.ai.providers'

const PROVIDER_PRESETS: Record<
  AIProviderVendor,
  { name: string; baseUrl: string; model: string; path: string }
> = {
  qwen: {
    name: '通义千问',
    baseUrl: 'https://dashscope.aliyuncs.com',
    model: 'qwen3.6-flash',
    path: '/compatible-mode/v1/chat/completions',
  },
  deepseek: {
    name: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com',
    model: 'deepseek-chat',
    path: '/chat/completions',
  },
  zhipu: {
    name: '智谱 AI',
    baseUrl: 'https://open.bigmodel.cn',
    model: 'glm-4-plus',
    path: '/api/paas/v4/chat/completions',
  },
  doubao: {
    name: '豆包',
    baseUrl: 'https://ark.cn-beijing.volces.com',
    model: 'doubao-1-5-pro-32k-250115',
    path: '/api/v3/chat/completions',
  },
  'openai-compatible': {
    name: 'OpenAI Compatible',
    baseUrl: 'https://api.openai.com',
    model: 'gpt-4o-mini',
    path: '/v1/chat/completions',
  },
}

type AIStreamHandlers = {
  onDelta?: (delta: string, fullText: string) => void
  onDone?: (fullText: string) => void
}

function trimTrailingSlash(value: string) {
  return value.replace(/\/+$/, '')
}

function resolvePreset(vendor: AIProviderVendor) {
  return PROVIDER_PRESETS[vendor]
}

function resolveRequestPath(provider: AIProviderConfig) {
  return resolvePreset(provider.vendor).path
}

function resolveEndpoint(provider: AIProviderConfig) {
  return `${trimTrailingSlash(provider.baseUrl)}${resolveRequestPath(provider)}`
}

function extractMessageContent(content: unknown): string {
  if (typeof content === 'string') {
    return content
  }

  if (!Array.isArray(content)) {
    return ''
  }

  return content
    .map((item) => {
      if (typeof item === 'string') {
        return item
      }

      if (item && typeof item === 'object' && 'text' in item) {
        return String((item as { text?: unknown }).text ?? '')
      }

      return ''
    })
    .join('\n')
    .trim()
}

function extractContent(raw: any): string {
  return extractMessageContent(raw?.choices?.[0]?.message?.content)
}

function extractStreamDelta(raw: any): string {
  const choice = raw?.choices?.[0]
  if (!choice) {
    return ''
  }

  const directDelta = choice.delta?.content
  if (typeof directDelta === 'string') {
    return directDelta
  }

  const mixedDelta = extractMessageContent(directDelta)
  if (mixedDelta) {
    return mixedDelta
  }

  return ''
}

function buildRequestBody(provider: AIProviderConfig, payload: AIChatPayload, stream = false) {
  const body: Record<string, unknown> = {
    model: provider.model,
    messages: payload.messages,
    stream,
  }

  if (typeof payload.temperature === 'number') {
    body.temperature = payload.temperature
  }

  if (typeof payload.maxTokens === 'number') {
    body.max_tokens = payload.maxTokens
  }

  if (provider.vendor === 'qwen') {
    body.result_format = 'message'
  }

  return body
}

function readLegacyProviders(): AIProviderConfig[] {
  const raw = localStorage.getItem(LEGACY_AI_PROVIDER_STORAGE_KEY)
  if (!raw) {
    return []
  }

  try {
    const parsed = JSON.parse(raw) as AIProviderConfig[]
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}

async function migrateLegacyProvidersIfNeeded() {
  const legacyProviders = readLegacyProviders()
  if (!legacyProviders.length) {
    return
  }

  const currentProviders = await invoke<AIProviderConfig[]>('get_ai_providers')
  if (currentProviders.length) {
    localStorage.removeItem(LEGACY_AI_PROVIDER_STORAGE_KEY)
    return
  }

  for (const provider of legacyProviders) {
    await invoke<AIProviderConfig[]>('save_ai_provider', {
      provider: {
        id: provider.id,
        name: provider.name,
        vendor: provider.vendor,
        apiKey: provider.apiKey,
        baseUrl: provider.baseUrl,
        model: provider.model,
        timeoutMs: provider.timeoutMs,
        enabled: provider.enabled,
      },
    })
  }

  localStorage.removeItem(LEGACY_AI_PROVIDER_STORAGE_KEY)
}

async function resolveProvider(payload: AIChatPayload) {
  const providers = await listAIProviders()
  const provider = providers.find((item) => item.id === payload.providerId)

  if (!provider) {
    throw new Error('未找到对应的 AI 供应商配置')
  }

  if (!provider.enabled) {
    throw new Error('当前 AI 供应商已被禁用')
  }

  if (!provider.apiKey.trim()) {
    throw new Error('当前 AI 供应商缺少 API Key')
  }

  return provider
}

async function streamTextFallback(
  payload: AIChatPayload,
  handlers: AIStreamHandlers,
): Promise<AIChatResult> {
  const result = await invokeAIChat(payload)
  let fullText = ''

  for (const char of result.content) {
    fullText += char
    handlers.onDelta?.(char, fullText)
    await new Promise((resolve) => window.setTimeout(resolve, 8))
  }

  handlers.onDone?.(result.content)
  return result
}

export function getAIProviderPreset(vendor: AIProviderVendor) {
  return resolvePreset(vendor)
}

export async function listAIProviders(): Promise<AIProviderConfig[]> {
  await migrateLegacyProvidersIfNeeded()
  return loggedInvoke<AIProviderConfig[]>('get_ai_providers')
}

export async function saveAIProvider(input: AIProviderInput): Promise<AIProviderConfig[]> {
  await migrateLegacyProvidersIfNeeded()
  return loggedInvoke<AIProviderConfig[]>(
    'save_ai_provider',
    { provider: input },
    { provider: { ...input, apiKey: input.apiKey ? '***' : '' } },
  )
}

export async function deleteAIProvider(id: string): Promise<AIProviderConfig[]> {
  await migrateLegacyProvidersIfNeeded()
  return loggedInvoke<AIProviderConfig[]>('delete_ai_provider', { providerId: id })
}

export async function invokeAIChat(payload: AIChatPayload): Promise<AIChatResult> {
  const provider = await resolveProvider(payload)
  logFrontendAction('ai.chat.request', {
    provider: provider.name,
    model: provider.model,
    messages: payload.messages.length,
  })

  const response = await axios.post(resolveEndpoint(provider), buildRequestBody(provider, payload), {
    timeout: provider.timeoutMs,
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${provider.apiKey}`,
    },
  })

  const content = extractContent(response.data)
  if (!content) {
    throw new Error('AI 返回结果为空，无法解析内容')
  }

  logFrontendAction('ai.chat.success', { provider: provider.name, length: content.length }, 'success')

  return {
    providerId: provider.id,
    providerName: provider.name,
    vendor: provider.vendor,
    model: provider.model,
    content,
    raw: response.data,
  }
}

export async function invokeAIChatStream(
  payload: AIChatPayload,
  handlers: AIStreamHandlers = {},
): Promise<AIChatResult> {
  const provider = await resolveProvider(payload)
  logFrontendAction('ai.stream.request', {
    provider: provider.name,
    model: provider.model,
    messages: payload.messages.length,
  })

  try {
    const response = await httpFetch(resolveEndpoint(provider), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${provider.apiKey}`,
      },
      connectTimeout: Math.max(5, Math.ceil(provider.timeoutMs / 1000)),
      body: JSON.stringify(buildRequestBody(provider, payload, true)),
    })

    if (!response.ok) {
      const errorText = await response.text()
      throw new Error(errorText || `AI 请求失败（${response.status}）`)
    }

    const reader = response.body?.getReader?.()
    if (!reader) {
      return await streamTextFallback(payload, handlers)
    }

    const decoder = new TextDecoder()
    let buffer = ''
    let content = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) {
        break
      }

      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split(/\r?\n/)
      buffer = lines.pop() ?? ''

      for (const rawLine of lines) {
        const line = rawLine.trim()
        if (!line.startsWith('data:')) {
          continue
        }

        const payloadText = line.slice(5).trim()
        if (!payloadText || payloadText === '[DONE]') {
          continue
        }

        try {
          const parsed = JSON.parse(payloadText)
          const delta = extractStreamDelta(parsed)
          if (!delta) {
            continue
          }

          content += delta
          handlers.onDelta?.(delta, content)
        } catch {
          continue
        }
      }
    }

    if (!content.trim()) {
      return await streamTextFallback(payload, handlers)
    }

    const finalContent = content.trim()
    handlers.onDone?.(finalContent)
    logFrontendAction('ai.stream.success', { provider: provider.name, length: finalContent.length }, 'success')

    return {
      providerId: provider.id,
      providerName: provider.name,
      vendor: provider.vendor,
      model: provider.model,
      content: finalContent,
      raw: null,
    }
  } catch (error) {
    logFrontendAction('ai.stream.fallback', error instanceof Error ? error.message : String(error), 'error')
    return await streamTextFallback(payload, handlers)
  }
}
