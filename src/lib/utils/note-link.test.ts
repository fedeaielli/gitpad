import { describe, expect, it } from 'vitest'
import { noteLink, noteOrder, noteRelativePath, resolveNoteLink } from './note-link'

describe('noteRelativePath', () => {
  it('stays in the same folder', () => {
    expect(noteRelativePath('notes/a.md', 'notes/b.md')).toBe('b.md')
  })

  it('walks to a parent and into another folder', () => {
    expect(noteRelativePath('notes/a.md', 'b.md')).toBe('../b.md')
    expect(noteRelativePath('notes/a.md', 'other/b.md')).toBe('../other/b.md')
  })

  it('descends from the repo root', () => {
    expect(noteRelativePath('a.md', 'notes/b.md')).toBe('notes/b.md')
  })
})

describe('noteLink', () => {
  it('uses the note name as the label and a relative destination', () => {
    expect(noteLink('notes/a.md', 'notes/b.md')).toBe('[b](b.md)')
    expect(noteLink('a.md', 'notes/b.md')).toBe('[b](notes/b.md)')
  })

  it('encodes spaces and parentheses in the destination', () => {
    expect(noteLink('a.md', 'my note (draft).md')).toBe('[my note (draft)](my%20note%20%28draft%29.md)')
  })
})

describe('noteOrder', () => {
  it('puts the current folder first, then the rest by path', () => {
    expect(
      noteOrder(['other/z.md', 'notes/c.md', 'notes/b.md', 'a.md'], 'notes/a.md'),
    ).toEqual(['notes/b.md', 'notes/c.md', 'a.md', 'other/z.md'])
  })

  it('treats root notes as the current folder', () => {
    expect(noteOrder(['notes/b.md', 'b.md', 'a.md'], 'a.md')).toEqual(['b.md', 'notes/b.md'])
  })

  it('drops the source note', () => {
    expect(noteOrder(['a.md', 'b.md'], 'a.md')).toEqual(['b.md'])
  })
})

describe('resolveNoteLink', () => {
  it('resolves a relative link against the source note', () => {
    expect(resolveNoteLink('notes/a.md', 'b.md')).toBe('notes/b.md')
    expect(resolveNoteLink('notes/a.md', '../b.md')).toBe('b.md')
    expect(resolveNoteLink('notes/a.md', '/other/b.md')).toBe('other/b.md')
  })

  it('drops an anchor and a query', () => {
    expect(resolveNoteLink('notes/a.md', 'b.md#heading')).toBe('notes/b.md')
    expect(resolveNoteLink('notes/a.md', 'b.md?raw')).toBe('notes/b.md')
  })

  it('decodes an encoded destination', () => {
    expect(resolveNoteLink('a.md', 'my%20note%20%28draft%29.md')).toBe('my note (draft).md')
  })

  it('rejects external links and paths that leave the repo', () => {
    expect(resolveNoteLink('notes/a.md', 'https://example.com')).toBeNull()
    expect(resolveNoteLink('a.md', '../b.md')).toBeNull()
  })
})
