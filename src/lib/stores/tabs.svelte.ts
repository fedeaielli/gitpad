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
