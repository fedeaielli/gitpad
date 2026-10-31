import { languageFor } from '../utils/langmap'

export interface Tab {
  id: string
  path: string
  content: string
  modified: boolean
  language: string
  /// Last content known to be on disk; guards against writing an empty buffer.
  savedContent?: string
  /// A pull rewrote this file while the tab had unsaved edits: never autosave.
  staleUpstream?: boolean
}

export const tabs = $state({
  tabs: [] as Tab[],
  activeTabId: '',
})

/// Paths removed by FileTree delete/rename: EditorPane must not write them back.
export const discarded = new Set<string>()

export function openTab(tab: Tab) {
  const existing = tabs.tabs.find((t) => t.id === tab.id)
  if (existing) {
    tabs.activeTabId = existing.id
    return
  }
  if (tab.savedContent === undefined) tab.savedContent = tab.content
  discarded.delete(tab.path)
  tabs.tabs.push(tab)
  tabs.activeTabId = tab.id
}

export function activateTab(id: string) {
  tabs.activeTabId = id
}

export function closeTab(id: string) {
  const index = tabs.tabs.findIndex((t) => t.id === id)
  if (index === -1) return
  // A stale buffer is older than the file on disk: never write it back on close.
  if (tabs.tabs[index].staleUpstream) discarded.add(tabs.tabs[index].path)
  tabs.tabs.splice(index, 1)
  if (tabs.activeTabId === id) {
    tabs.activeTabId = (tabs.tabs[index] ?? tabs.tabs[index - 1])?.id ?? ''
  }
}

/// Close a tab only when it has no unsaved edits; false means it blocked.
export function closeIfClean(path: string) {
  const tab = tabs.tabs.find((t) => t.path === path)
  if (!tab) return true
  if (tab.modified) return false
  discarded.add(tab.id)
  closeTab(tab.id)
  return true
}

/// Close every tab at or under `path` without writing anything back to disk.
export function discardUnder(path: string) {
  for (const tab of [...tabs.tabs]) {
    if (tab.path === path || tab.path.startsWith(`${path}/`)) {
      discarded.add(tab.path)
      closeTab(tab.id)
    }
  }
}

/// Follow a file or directory rename in the open tabs.
export function retarget(oldPath: string, newPath: string) {
  for (const tab of tabs.tabs) {
    if (tab.path !== oldPath && !tab.path.startsWith(`${oldPath}/`)) continue
    const next = newPath + tab.path.slice(oldPath.length)
    if (tabs.activeTabId === tab.id) tabs.activeTabId = next
    discarded.add(tab.path)
    tab.id = next
    tab.path = next
    tab.language = languageFor(next)
  }
}
