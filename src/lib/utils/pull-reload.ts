export interface UpstreamChange {
  path: string
  status: string
}

export interface ReloadTab {
  id: string
  path: string
  modified: boolean
  /// Last content written to disk: lets reloadTab tell our own write from an edit.
  savedContent?: string
  /// Current buffer; only read to detect a no-op reload of a clean tab.
  content?: string
}

export interface PullReloadPlan {
  reload: string[]
  close: string[]
  stale: string[]
}

/// Decides what a pull may touch: clean tabs reload, deleted files close. A
/// modified file still reloads even while a write is in flight: reloadTab reads
/// the disk and tells our own atomic write from an external edit.
export function planPullReload(
  changes: UpstreamChange[],
  tabs: ReloadTab[],
  busy: (path: string) => boolean,
): PullReloadPlan {
  const plan: PullReloadPlan = { reload: [], close: [], stale: [] }
  for (const change of changes) {
    const tab = tabs.find((candidate) => candidate.path === change.path)
    if (!tab) continue
    // A delete under a write in flight would throw away the buffer: stay put.
    if (change.status === 'D') {
      if (tab.modified || busy(tab.path)) plan.stale.push(change.path)
      else plan.close.push(change.path)
      continue
    }
    plan.reload.push(change.path)
  }
  return plan
}

export type ReloadAction = 'stale' | 'keep' | 'apply'

/// Decides what reloadTab does with the content it just read. `writing` means
/// our own atomic write overlapped the read: the disk holds either the old or
/// the new bytes, both ours, so the buffer wins and nothing is stale.
export function decideReload(tab: ReloadTab, content: string, writing: boolean): ReloadAction {
  if (writing) return 'keep'
  // Dirty and disk is exactly our last write: the watcher echoing our autosave,
  // not a conflict. Keep the unsaved keystrokes.
  if (tab.modified) return content === tab.savedContent ? 'keep' : 'stale'
  return content === tab.content ? 'keep' : 'apply'
}
