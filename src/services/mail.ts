import { loggedInvoke } from '@/services/debug'
import type {
  MailAccount,
  MailAccountInput,
  MailMessageDetail,
  MailMessageSummary,
  SendMailInput,
} from '@/types/mail'

export function getMailAccounts(): Promise<MailAccount[]> {
  return loggedInvoke('get_mail_accounts')
}

export function saveMailAccount(account: MailAccountInput): Promise<MailAccount[]> {
  return loggedInvoke('save_mail_account', { account }, { account: { ...account, password: account.password ? '***' : '' } })
}

export function deleteMailAccount(accountId: string): Promise<MailAccount[]> {
  return loggedInvoke('delete_mail_account', { accountId })
}

export function fetchMailMessages(limitPerAccount = 20): Promise<MailMessageSummary[]> {
  return loggedInvoke('fetch_mail_messages', { limitPerAccount })
}

export function syncMailMessages(limitPerAccount = 20): Promise<MailMessageSummary[]> {
  return loggedInvoke('sync_mail_messages', { limitPerAccount })
}

export function fetchMailDetail(accountId: string, uid: number): Promise<MailMessageDetail> {
  return loggedInvoke('fetch_mail_detail', { accountId, uid })
}

export function sendMail(input: SendMailInput): Promise<void> {
  return loggedInvoke('send_mail', { input }, { input: { ...input, body: `[${input.body.length} chars]` } })
}
