import { invoke } from '@tauri-apps/api/core'
import { withTimeout } from '../utils/timeout'

export interface ConflictFile {
  path: string
}

export interface FileChange {
  path: string
  /** "M", "A" (untracked/staged new) or "D". */
  status: string
}

export interface GitStatus {
  synced: boolean
  branch: string
  changes: FileChange[]
  modifiedFiles: string[]
  conflicts: ConflictFile[]
  ahead: number
  behind: number
  diverged: boolean
}

export const gitStatus = $state({
  synced: true,
  syncing: false,
  branch: '',
  changes: [] as FileChange[],
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
  gitStatus.branch = status.branch
  gitStatus.changes = status.changes
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

const SYNC_TIMEOUT_MS = 120_000
const SYNC_TIMEOUT_ERROR = 'sync timed out: the network may be down; local commits are safe'

/// Incremented per run: a late result from an older run must not touch the state.
let runId = 0

/// Pull, push or full sync share the same guard, spinner and error handling.
async function run(operation: () => Promise<GitStatus>, mark = false) {
  if (gitStatus.syncing || !gitStatus.configured) return false
  const id = ++runId
  gitStatus.syncing = true
  gitStatus.error = null
  try {
    // A timed-out run keeps going in Rust. The worktree gate serializes writes,
    // so a second sync may start while the first one is still in flight.
    const status = await withTimeout(operation(), SYNC_TIMEOUT_MS, 'sync')
    if (id !== runId) return false
    if (mark) markPushed(status)
    else applyGitStatus(status)
    return true
  } catch (e) {
    if (id !== runId) return false
    const text = errorMessage(e)
    if (text.startsWith('sync timed out')) gitStatus.error = SYNC_TIMEOUT_ERROR
    else if (!isIgnorableSyncError(e)) gitStatus.error = text
    return false
  } finally {
    if (id === runId) gitStatus.syncing = false
  }
}

/// Fetch + fast-forward/merge only.
export function pullOnly() {
  return run(() => invoke<GitStatus>('git_pull'))
}

/// Commit + push, skipping the pull (the push-only button).
export function pushOnly(message: string | null = null) {
  return run(() => invoke<GitStatus>('git_push', { message, pull: false }), true)
}

/// Single push path for the editor, git panel and conflict dialog.
export function syncNow(message: string | null) {
  return run(() => invoke<GitStatus>('git_push', { message }), true)
}
