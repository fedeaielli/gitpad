import { saveAllNow } from '../editor/bridge'
import { gitStatus, syncNow } from '../stores/git.svelte'
import { pushToast } from '../stores/toasts.svelte'
import { syncOutcome } from './sync-feedback'

let savingNow = false

/// Ctrl+S: flush every dirty tab now, then push. Autosave stays untouched.
export async function saveAndSync() {
  if (savingNow) return
  if (gitStatus.syncing) {
    pushToast('Sync already in progress', 'info')
    return
  }
  savingNow = true
  try {
    const hadChanges = gitStatus.changes.length > 0
    const hadAhead = gitStatus.ahead > 0
    const saved = await saveAllNow()
    if (!saved.ok) {
      const out = syncOutcome({
        saveOk: false,
        saveError: saved.error,
        wrote: saved.wrote,
        hadChanges,
        hadAhead,
        pushed: false,
        conflicts: false,
        error: null,
      })
      pushToast(out.message, out.kind)
      return
    }
    // Never `git_push` directly: the store guards conflicts, double syncs and timeouts.
    const pushed = await syncNow(null)
    const out = syncOutcome({
      saveOk: true,
      saveError: null,
      wrote: saved.wrote,
      hadChanges,
      hadAhead,
      pushed,
      conflicts: gitStatus.conflicts.length > 0,
      error: gitStatus.error,
    })
    pushToast(out.message, out.kind)
  } finally {
    savingNow = false
  }
}
