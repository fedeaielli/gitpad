import { describe, expect, it } from 'vitest'
import { isCurrentVersion } from './file-history'

const change = (path: string) => ({ path, status: 'M' })

describe('isCurrentVersion', () => {
  it('marks the newest commit when the file is clean', () => {
    expect(isCurrentVersion(0, 'note.md', [], false)).toBe(true)
  })

  it('never marks an older commit', () => {
    expect(isCurrentVersion(1, 'note.md', [], false)).toBe(false)
  })

  it('does not mark a match while the open tab has unsaved edits', () => {
    expect(isCurrentVersion(0, 'note.md', [], true)).toBe(false)
  })

  it('does not mark a match while the working tree differs', () => {
    expect(isCurrentVersion(0, 'note.md', [change('note.md')], false)).toBe(false)
  })

  it('ignores changes to other files', () => {
    expect(isCurrentVersion(0, 'note.md', [change('other.md')], false)).toBe(true)
  })
})
