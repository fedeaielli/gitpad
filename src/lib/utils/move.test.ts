import { describe, expect, it } from 'vitest'
import { canMoveInto, moveTarget } from './move'

describe('canMoveInto', () => {
  it('allows another folder and the root', () => {
    expect(canMoveInto('a.md', 'notes')).toBe(true)
    expect(canMoveInto('notes/a.md', '')).toBe(true)
    expect(canMoveInto('a/x.md', 'b')).toBe(true)
  })

  it('refuses itself, its subtree and its current parent', () => {
    expect(canMoveInto('notes', 'notes')).toBe(false)
    expect(canMoveInto('notes', 'notes/sub')).toBe(false)
    expect(canMoveInto('a.md', '')).toBe(false)
    expect(canMoveInto('notes/a.md', 'notes')).toBe(false)
  })

  it('does not mistake a shared prefix for a subtree', () => {
    expect(canMoveInto('note', 'notes')).toBe(true)
  })
})

describe('moveTarget', () => {
  it('joins the entry name to the folder', () => {
    expect(moveTarget('a.md', 'notes')).toBe('notes/a.md')
    expect(moveTarget('notes/a.md', '')).toBe('a.md')
    expect(moveTarget('notes/a.md', 'other')).toBe('other/a.md')
  })
})
