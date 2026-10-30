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
