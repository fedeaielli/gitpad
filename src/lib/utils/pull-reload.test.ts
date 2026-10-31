import { describe, expect, it } from 'vitest'
import { planPullReload, type ReloadTab } from './pull-reload'

const tab = (path: string, modified = false): ReloadTab => ({ id: path, path, modified })
const notBusy = () => false

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

  it('marks dirty tabs stale instead of reloading them', () => {
    const plan = planPullReload([{ path: 'a.md', status: 'M' }], [tab('a.md', true)], notBusy)
    expect(plan).toEqual({ reload: [], close: [], stale: ['a.md'] })
  })

  it('marks a dirty tab stale when the file was deleted upstream', () => {
    const plan = planPullReload([{ path: 'a.md', status: 'D' }], [tab('a.md', true)], notBusy)
    expect(plan).toEqual({ reload: [], close: [], stale: ['a.md'] })
  })

  it('reloads a resolved path unless the tab has edits', () => {
    const changes = [{ path: 'note.md', status: 'M' }]
    expect(planPullReload(changes, [tab('note.md')], notBusy)).toEqual({
      reload: ['note.md'],
      close: [],
      stale: [],
    })
    expect(planPullReload(changes, [tab('note.md', true)], notBusy)).toEqual({
      reload: [],
      close: [],
      stale: ['note.md'],
    })
  })

  it('marks tabs with a save in flight stale', () => {
    const plan = planPullReload([{ path: 'a.md', status: 'M' }], [tab('a.md')], (path) => path === 'a.md')
    expect(plan).toEqual({ reload: [], close: [], stale: ['a.md'] })
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
