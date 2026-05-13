import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  deleteMailAccount,
  fetchMailDetail,
  fetchMailMessages,
  getMailAccounts,
  saveMailAccount,
  sendMail,
  syncMailMessages,
} from '@/services/mail'
import type {
  MailAccount,
  MailAccountInput,
  MailMessageDetail,
  MailMessageSummary,
  SendMailInput,
} from '@/types/mail'

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

export const useMailStore = defineStore('mail', () => {
  const accounts = ref<MailAccount[]>([])
  const messages = ref<MailMessageSummary[]>([])
  const activeMessage = ref<MailMessageDetail | null>(null)
  const activeMessageKey = ref('')
  const loadingAccounts = ref(false)
  const loadingMessages = ref(false)
  const loadingDetail = ref(false)
  const syncingMessages = ref(false)
  const sending = ref(false)
  const error = ref('')

  const defaultAccount = computed(
    () =>
      accounts.value.find((account) => account.enabled && account.defaultSender) ??
      accounts.value.find((account) => account.enabled) ??
      null,
  )

  async function loadAccounts() {
    loadingAccounts.value = true
    error.value = ''

    try {
      accounts.value = await getMailAccounts()
    } catch (err) {
      error.value = getErrorMessage(err, '加载邮箱账号失败')
    } finally {
      loadingAccounts.value = false
    }
  }

  async function loadMessages(limitPerAccount = 20) {
    loadingMessages.value = true
    error.value = ''

    try {
      messages.value = await fetchMailMessages(limitPerAccount)

      if (!messages.value.length) {
        activeMessage.value = null
        activeMessageKey.value = ''
        return
      }

      const stillExists = messages.value.some(
        (message) => `${message.accountId}:${message.uid}` === activeMessageKey.value,
      )

      if (!stillExists) {
        const first = messages.value[0]
        await openMessage(first.accountId, first.uid)
      }
    } catch (err) {
      error.value = getErrorMessage(err, '加载本地邮件列表失败')
    } finally {
      loadingMessages.value = false
    }
  }

  async function pullMessages(limitPerAccount = 20) {
    syncingMessages.value = true
    error.value = ''

    try {
      messages.value = await syncMailMessages(limitPerAccount)
      
      if (!messages.value.length) {
        activeMessage.value = null
        activeMessageKey.value = ''
        return
      }

      const stillExists = messages.value.some(
        (message) => `${message.accountId}:${message.uid}` === activeMessageKey.value,
      )

      if (activeMessageKey.value && stillExists) {
        const [accountId, uid] = activeMessageKey.value.split(':')
        await openMessage(accountId, Number(uid))
      } else {
        const first = messages.value[0]
        await openMessage(first.accountId, first.uid)
      }
    } catch (err) {
      error.value = getErrorMessage(err, '拉取邮件并更新本地缓存失败')
    } finally {
      syncingMessages.value = false
    }
  }

  async function initialize() {
    await loadAccounts()
    await loadMessages()
  }

  async function upsertAccount(input: MailAccountInput) {
    error.value = ''

    try {
      accounts.value = await saveMailAccount(input)
      await loadMessages()
    } catch (err) {
      error.value = getErrorMessage(err, '保存邮箱账号失败')
      throw err
    }
  }

  async function removeAccount(accountId: string) {
    error.value = ''

    try {
      accounts.value = await deleteMailAccount(accountId)
      if (activeMessage.value?.accountId === accountId) {
        activeMessage.value = null
        activeMessageKey.value = ''
      }
      await loadMessages()
    } catch (err) {
      error.value = getErrorMessage(err, '删除邮箱账号失败')
      throw err
    }
  }

  async function openMessage(accountId: string, uid: number) {
    loadingDetail.value = true
    error.value = ''
    activeMessageKey.value = `${accountId}:${uid}`

    try {
      activeMessage.value = await fetchMailDetail(accountId, uid)
      messages.value = messages.value.map((message) =>
        message.accountId === accountId && message.uid === uid
          ? { ...message, unread: false }
          : message,
      )
    } catch (err) {
      error.value = getErrorMessage(err, '加载本地邮件详情失败')
    } finally {
      loadingDetail.value = false
    }
  }

  async function sendMessage(input: SendMailInput) {
    sending.value = true
    error.value = ''

    try {
      await sendMail(input)
      await loadMessages()
    } catch (err) {
      error.value = getErrorMessage(err, '发送邮件失败')
      throw err
    } finally {
      sending.value = false
    }
  }

  return {
    accounts,
    activeMessage,
    activeMessageKey,
    defaultAccount,
    error,
    initialize,
    loadAccounts,
    loadMessages,
    loadingAccounts,
    loadingDetail,
    loadingMessages,
    messages,
    openMessage,
    pullMessages,
    removeAccount,
    sending,
    sendMessage,
    syncingMessages,
    upsertAccount,
  }
})
