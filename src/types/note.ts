export type NoteNodeType = 'directory' | 'file'

export interface NoteTreeNode {
  name: string
  path: string
  nodeType: NoteNodeType
  children?: NoteTreeNode[]
}

export interface NoteDocument {
  path: string
  title: string
  content: string
  updatedAt?: string | null
  lastBackupAt?: string | null
}

export interface NoteSaveResult {
  updatedAt: string | null
  lastBackupAt: string | null
}

export interface MoveEntryResult {
  path: string
}

export interface NoteImageAsset {
  filePath: string
  markdownPath: string
}

export interface NoteWorkspace {
  rootPath: string
  tree: NoteTreeNode[]
  backupRootPath: string
  backupRetentionDays: number
}

export interface NoteSyncResult {
  checkedAt: string
  lastSyncedAt: string
  downloadTotal: number
  uploadTotal: number
  downloaded: number
  uploaded: number
  skipped: number
  message: string
}

export interface NoteSyncProgress {
  stage: string
  current: number
  total: number
  message: string
}

export interface NoteSyncSettings {
  serverUrl: string
  apiKey: string
  lastSyncedAt?: string | null
}

export interface NoteWorkspaceSettings {
  notesRootPath: string
  backupRootPath: string
  mailRootPath: string
  backupRetentionDays: number
}

export interface BackupSettings {
  backupRootPath: string
  backupRetentionDays: number
}

export interface BackupHistoryItem {
  path: string
  name: string
  noteTitle: string
  notePath: string
  createdAt: string | null
}

export interface BackupDocument {
  path: string
  name: string
  noteTitle: string
  notePath: string
  content: string
  createdAt: string | null
}
