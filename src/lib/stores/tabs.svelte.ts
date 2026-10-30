import { languageFor } from '../utils/langmap'

export interface Tab {
  id: string
  path: string
  content: string
  modified: boolean
  language: string
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
  tabs.tabs.push(tab)
  tabs.activeTabId = tab.id
}

export function activateTab(id: string) {
  tabs.activeTabId = id
}

export function closeTab(id: string) {
  const index = tabs.tabs.findIndex((t) => t.id === id)
  if (index === -1) return
  tabs.tabs.splice(index, 1)
  if (tabs.activeTabId === id) {
    tabs.activeTabId = (tabs.tabs[index] ?? tabs.tabs[index - 1])?.id ?? ''
  }
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
