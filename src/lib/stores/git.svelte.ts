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

/// Expected states (no token yet, no clone yet) must not surface as sync errors.
export function isIgnorableSyncError(message: string) {
  return message.includes('not authenticated') || message.includes('no repository cloned')
}
