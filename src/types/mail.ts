export interface MailAccount {
  id: string
  name: string
  address: string
  enabled: boolean
  imapHost: string
  imapPort: number
  smtpHost: string
  smtpPort: number
  username: string
  password: string
  useTls: boolean
  defaultSender: boolean
}

export interface MailAccountInput {
  id?: string
  name: string
  address: string
  enabled: boolean
  imapHost: string
  imapPort: number
  smtpHost: string
  smtpPort: number
  username: string
  password: string
  useTls: boolean
  defaultSender: boolean
}

export interface MailMessageSummary {
  uid: number
  accountId: string
  accountName: string
  subject: string
  from: string
  to: string[]
  date: string
  unread: boolean
  preview: string
}

export interface MailMessageDetail {
  uid: number
  accountId: string
  accountName: string
  subject: string
  from: string
  to: string[]
  cc: string[]
  date: string
  unread: boolean
  textBody: string
  htmlBody?: string | null
}

export interface SendMailInput {
  accountId: string
  to: string[]
  cc: string[]
  subject: string
  body: string
  isHtml: boolean
}
