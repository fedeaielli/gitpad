export interface SyncOutcomeInput {
  saveOk: boolean
  saveError: string | null
  wrote: boolean
  hadChanges: boolean
  hadAhead: boolean
  pushed: boolean
  conflicts: boolean
  error: string | null
}

/// One message for Ctrl+S, in the order the failures actually matter.
export function syncOutcome(i: SyncOutcomeInput): {
  kind: 'success' | 'info' | 'error'
  message: string
} {
  if (!i.saveOk) {
    return {
      kind: 'error',
      message: `Not saved: ${i.saveError ?? 'unknown error'}. Other tabs are committed locally and will be pushed automatically.`,
    }
  }
  if (i.conflicts) return { kind: 'error', message: 'Resolve the conflicts first' }
  if (!i.pushed) {
    return {
      kind: 'error',
      message: `Saved, push failed: ${i.error ?? 'unknown error'}. Your commit is safe locally.`,
    }
  }
  if (!i.wrote && !i.hadChanges && !i.hadAhead) {
    return { kind: 'info', message: 'Saved, nothing to push' }
  }
  return { kind: 'success', message: 'Saved and pushed' }
}
