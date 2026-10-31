import { beforeEach, describe, expect, it, vi } from 'vitest'

let store: typeof import('./tabs.svelte')

beforeEach(async () => {
  vi.resetModules()
  store = await import('./tabs.svelte')
})

function tab(id: string, path = id, language = 'plaintext') {
  return { id, path, content: '', modified: false, language }
}

describe('tabs store', () => {
  it('opens and activates tabs, deduplicating by id', () => {
    store.openTab(tab('/a.md'))
    store.openTab(tab('/b.md'))
    expect(store.tabs.tabs.map((t) => t.id)).toEqual(['/a.md', '/b.md'])
    expect(store.tabs.activeTabId).toBe('/b.md')

    store.openTab(tab('/a.md'))
    expect(store.tabs.tabs).toHaveLength(2)
    expect(store.tabs.activeTabId).toBe('/a.md')
  })

  it('closes the active tab and activates a neighbour', () => {
    store.openTab(tab('/a'))
    store.openTab(tab('/b'))
    store.openTab(tab('/c'))
    store.tabs.activeTabId = '/b'

    store.closeTab('/b')
    expect(store.tabs.tabs.map((t) => t.id)).toEqual(['/a', '/c'])
    expect(store.tabs.activeTabId).toBe('/c')

    store.closeTab('/c')
    expect(store.tabs.activeTabId).toBe('/a')
  })

  it('discards every tab under a deleted path', () => {
    store.openTab(tab('/notes/a.md'))
    store.openTab(tab('/notes/deep/b.md'))
    store.openTab(tab('/other/c.md'))

    store.discardUnder('/notes')
    expect(store.tabs.tabs.map((t) => t.id)).toEqual(['/other/c.md'])
    expect([...store.discarded].sort()).toEqual(['/notes/a.md', '/notes/deep/b.md'])
  })

  it('retargets renamed paths and remaps the language', () => {
    store.openTab(tab('/notes/a.txt'))

    store.retarget('/notes/a.txt', '/notes/a.md')
    const [only] = store.tabs.tabs
    expect(only.id).toBe('/notes/a.md')
    expect(only.path).toBe('/notes/a.md')
    expect(only.language).toBe('markdown')
    expect(store.discarded.has('/notes/a.txt')).toBe(true)
  })
})
