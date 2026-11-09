import { describe, expect, it } from 'vitest'
import { decideReload, planPullReload, type ReloadTab } from './pull-reload'

const tab = (path: string, modified = false, savedContent?: string, content = savedContent): ReloadTab => ({
  id: path,
  path,
  modified,
  savedContent,
  content,
})
const notBusy = () => false
const busyOn = (busyPath: string) => (path: string) => path === busyPath

describe('planPullReload', () => {
  it('reloads clean tabs for modified and added files', () => {
    const plan = planPullReload(
      [
        { path: 'a.md', status: 'M' },
        { path: 'b.md', status: 'A' },
      ],
      [tab('a.md'), tab('b.md')],
      notBusy,
    )
    expect(plan).toEqual({ reload: ['a.md', 'b.md'], close: [], stale: [] })
  })

  it('closes clean tabs whose file was deleted upstream', () => {
    const plan = planPullReload([{ path: 'a.md', status: 'D' }], [tab('a.md')], notBusy)
    expect(plan).toEqual({ reload: [], close: ['a.md'], stale: [] })
  })

  it('sends a dirty tab to reload so reloadTab can compare disk to savedContent', () => {
    const plan = planPullReload([{ path: 'a.md', status: 'M' }], [tab('a.md', true)], notBusy)
    expect(plan).toEqual({ reload: ['a.md'], close: [], stale: [] })
  })

  it('marks a dirty tab stale instead of reloading a deleted file', () => {
    const plan = planPullReload([{ path: 'a.md', status: 'D' }], [tab('a.md', true)], notBusy)
    expect(plan).toEqual({ reload: [], close: [], stale: ['a.md'] })
  })

  it('marks tabs with a save in flight stale, clean or dirty', () => {
    const change = [{ path: 'a.md', status: 'M' }]
    expect(planPullReload(change, [tab('a.md')], busyOn('a.md'))).toEqual({
      reload: [],
      close: [],
      stale: ['a.md'],
    })
    expect(planPullReload(change, [tab('a.md', true)], busyOn('a.md'))).toEqual({
      reload: [],
      close: [],
      stale: ['a.md'],
    })
  })

  it('ignores paths with no open tab', () => {
    const plan = planPullReload([{ path: 'gone.md', status: 'M' }], [tab('a.md')], notBusy)
    expect(plan).toEqual({ reload: [], close: [], stale: [] })
  })

  it('returns empty lists for an empty change set', () => {
    const plan = planPullReload([], [tab('a.md')], notBusy)
    expect(plan).toEqual({ reload: [], close: [], stale: [] })
  })
})

describe('decideReload', () => {
  it('keeps a dirty tab when disk equals savedContent: our own autosave echo', () => {
    const dirty = tab('a.md', true, 'saved')
    expect(decideReload(dirty, 'saved', false)).toBe('keep')
  })

  it('marks a dirty tab stale when disk differs from savedContent', () => {
    const dirty = tab('a.md', true, 'saved')
    expect(decideReload(dirty, 'someone else', false)).toBe('stale')
  })

  it('marks a tab stale when a save or keystroke raced the read', () => {
    expect(decideReload(tab('a.md', true, 'saved'), 'saved', true)).toBe('stale')
    expect(decideReload(tab('a.md'), 'other', true)).toBe('stale')
  })

  it('applies new content to a clean tab', () => {
    expect(decideReload(tab('a.md', false, 'old'), 'new', false)).toBe('apply')
  })

  it('keeps a clean tab whose content already matches disk', () => {
    expect(decideReload(tab('a.md', false, 'same'), 'same', false)).toBe('keep')
  })
})
