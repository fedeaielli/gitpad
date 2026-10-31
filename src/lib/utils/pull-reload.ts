export interface UpstreamChange {
  path: string
  status: string
}

export interface ReloadTab {
  id: string
  path: string
  modified: boolean
}

export interface PullReloadPlan {
  reload: string[]
  close: string[]
  stale: string[]
}

/// Decides what a pull may touch: clean tabs reload, deleted files close, and
/// anything with unsaved edits stays put so the buffer is never overwritten.
export function planPullReload(
  changes: UpstreamChange[],
  tabs: ReloadTab[],
  busy: (path: string) => boolean,
): PullReloadPlan {
  const plan: PullReloadPlan = { reload: [], close: [], stale: [] }
  for (const change of changes) {
    const tab = tabs.find((candidate) => candidate.path === change.path)
    if (!tab) continue
    if (tab.modified || busy(tab.path)) {
      plan.stale.push(change.path)
      continue
    }
    if (change.status === 'D') plan.close.push(change.path)
    else plan.reload.push(change.path)
  }
  return plan
}
