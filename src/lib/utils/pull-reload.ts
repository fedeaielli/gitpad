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
}

export interface PullReloadPlan {
  reload: string[]
  close: string[]
  stale: string[]
}

/// Decides what a pull may touch: clean tabs reload, deleted files close, and
/// a write in flight stays put so the buffer is never overwritten. Dirty tabs
/// still reload: reloadTab compares disk to savedContent and decides there.
export function planPullReload(
  changes: UpstreamChange[],
  tabs: ReloadTab[],
  busy: (path: string) => boolean,
): PullReloadPlan {
  const plan: PullReloadPlan = { reload: [], close: [], stale: [] }
  for (const change of changes) {
    const tab = tabs.find((candidate) => candidate.path === change.path)
    if (!tab) continue
    if (busy(tab.path)) {
      plan.stale.push(change.path)
      continue
    }
    if (change.status === 'D') {
      if (tab.modified) plan.stale.push(change.path)
      else plan.close.push(change.path)
      continue
    }
    plan.reload.push(change.path)
  }
  return plan
}
