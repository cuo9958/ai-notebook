import { loggedInvoke } from '@/services/debug'
import type {
  BackupDocument,
  BackupHistoryItem,
  BackupSettings,
  NoteDocument,
  MoveEntryResult,
  NoteSaveResult,
  NoteImageAsset,
  NoteSyncResult,
  NoteSyncSettings,
  NoteWorkspaceSettings,
  NoteWorkspace,
} from '@/types/note'

export function listNotes(): Promise<NoteWorkspace> {
  return loggedInvoke('list_notes')
}

export function syncNotes(): Promise<NoteSyncResult> {
  return loggedInvoke('sync_notes')
}

export function readNote(path: string): Promise<NoteDocument> {
  return loggedInvoke('read_note', { path })
}

export function createDirectory(parentPath: string | null, name: string): Promise<void> {
  return loggedInvoke('create_directory', { parentPath, name })
}

export function createNote(parentPath: string | null, title: string): Promise<NoteDocument> {
  return loggedInvoke('create_note', { parentPath, title })
}

export function saveNote(path: string, content: string): Promise<NoteSaveResult> {
  return loggedInvoke('save_note', { path, content }, { path, contentLength: content.length })
}

export function saveNoteImage(path: string, fileName: string, data: number[]): Promise<NoteImageAsset> {
  return loggedInvoke('save_note_image', { notePath: path, fileName, data }, { notePath: path, fileName, dataLength: data.length })
}

export function renameEntry(path: string, newName: string): Promise<void> {
  return loggedInvoke('rename_entry', { path, newName })
}

export function moveEntry(path: string, targetParentPath: string | null): Promise<MoveEntryResult> {
  return loggedInvoke('move_entry', { path, targetParentPath })
}

export function deleteEntry(path: string): Promise<void> {
  return loggedInvoke('delete_entry', { path })
}

export function getBackupSettings(): Promise<BackupSettings> {
  return loggedInvoke('get_backup_settings')
}

export function getNoteWorkspaceSettings(): Promise<NoteWorkspaceSettings> {
  return loggedInvoke('get_note_workspace_settings')
}

export function getNoteSyncSettings(): Promise<NoteSyncSettings> {
  return loggedInvoke('get_note_sync_settings')
}

export function updateNoteSyncSettings(serverUrl: string, apiKey: string): Promise<NoteSyncSettings> {
  return loggedInvoke('update_note_sync_settings', { serverUrl, apiKey }, { serverUrl, apiKey: apiKey ? '***' : '' })
}

export function updateNoteWorkspaceSettings(
  notesRootPath: string,
  mailRootPath?: string,
): Promise<NoteWorkspaceSettings> {
  return loggedInvoke('update_note_workspace_settings', { notesRootPath, mailRootPath })
}

export function updateBackupSettings(
  backupRootPath: string,
  backupRetentionDays: number,
): Promise<BackupSettings> {
  return loggedInvoke('update_backup_settings', { backupRootPath, backupRetentionDays })
}

export function listBackups(notePath?: string): Promise<BackupHistoryItem[]> {
  return loggedInvoke('list_backups', { notePath })
}

export function readBackup(backupPath: string): Promise<BackupDocument> {
  return loggedInvoke('read_backup', { backupPath })
}

export function restoreBackup(backupPath: string): Promise<NoteDocument> {
  return loggedInvoke('restore_backup', { backupPath })
}
