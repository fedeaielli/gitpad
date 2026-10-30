export interface ConflictFile {
  path: string
}

export const gitStatus = $state({
  synced: true,
  syncing: false,
  ahead: 0,
  behind: 0,
  diverged: false,
  lastPush: null as Date | null,
  conflicts: [] as ConflictFile[],
  error: null as string | null,
})
