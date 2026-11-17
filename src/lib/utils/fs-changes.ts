import { debounce } from './debounce'

export interface FsChange {
  path: string
  status: string
}

/// A big import or delete arrives as hundreds of debounced batches. Merging
/// them and refreshing once, after the burst goes quiet, scans the worktree once.
export const FS_FLUSH_MS = 1500

export function coalesceChanges(
  apply: (changes: FsChange[]) => Promise<void>,
  ms = FS_FLUSH_MS,
) {
  // Map keeps the first position and the latest status for a repeated path.
  const pending = new Map<string, string>()
  let running = false

  async function flush() {
    if (running || pending.size === 0) return
    running = true
    try {
      // Changes that arrive while `apply` runs are drained in one more pass,
      // so a slow status scan never gets a second one on top of it.
      while (pending.size > 0) {
        const batch = [...pending].map(([path, status]) => ({ path, status }))
        pending.clear()
        await apply(batch)
      }
    } finally {
      running = false
    }
  }

  const debounced = debounce(() => void flush(), ms)
  return {
    add(changes: FsChange[]) {
      for (const change of changes) pending.set(change.path, change.status)
      debounced.run()
    },
  }
}
