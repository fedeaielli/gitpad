import { invoke } from '@tauri-apps/api/core'

export interface ConflictFile {
  path: string
}

export interface GitStatus {
  synced: boolean
  modifiedFiles: string[]
  conflicts: ConflictFile[]
  ahead: number
  behind: number
  diverged: boolean
}

export const gitStatus = $state({
  synced: true,
  syncing: false,
  modifiedFiles: [] as string[],
  ahead: 0,
  behind: 0,
  diverged: false,
  lastPush: null as Date | null,
  conflicts: [] as ConflictFile[],
  error: null as string | null,
  /** False until a repository answers git_status. */
  configured: false,
})

export function applyGitStatus(status: GitStatus) {
  gitStatus.synced = status.synced
  gitStatus.modifiedFiles = status.modifiedFiles
  gitStatus.ahead = status.ahead
  gitStatus.behind = status.behind
  gitStatus.diverged = status.diverged
  gitStatus.conflicts = status.conflicts
  gitStatus.error = null
  gitStatus.configured = true
}

export function markPushed(status: GitStatus) {
  applyGitStatus(status)
  gitStatus.lastPush = new Date()
}

/// Tauri errors are either strings or `{ code, message }` from the Rust GitError enum.
export function errorMessage(error: unknown): string {
  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message?: unknown }).message
    if (typeof message === 'string') return message
  }
  return String(error)
}

/// A missing repository is the only state that means "not configured yet".
export function isNoRepoError(error: unknown) {
  return !!error && typeof error === 'object' && (error as { code?: unknown }).code === 'NOT_FOUND'
}

/// Expected states (no token yet, no clone yet) must not surface as sync errors.
export function isIgnorableSyncError(error: unknown) {
  return errorMessage(error).includes('not authenticated') || isNoRepoError(error)
}

/// Single push path for the editor, git panel and conflict dialog.
export async function syncNow(message: string | null) {
  if (gitStatus.syncing || !gitStatus.configured) return false
  gitStatus.syncing = true
  gitStatus.error = null
  try {
    markPushed(await invoke<GitStatus>('git_push', { message }))
    return true
  } catch (e) {
    const text = errorMessage(e)
    if (!isIgnorableSyncError(e)) gitStatus.error = text
    return false
  } finally {
    gitStatus.syncing = false
  }
}
