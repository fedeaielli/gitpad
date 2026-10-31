import { describe, expect, it } from 'vitest'
import { saveDecision } from './save-guard'

describe('saveDecision', () => {
  it('skips when nothing changed', () => {
    expect(saveDecision({ content: 'same', savedContent: 'same' })).toBe('skip')
  })

  it('writes when the content differs', () => {
    expect(saveDecision({ content: 'new', savedContent: 'old' })).toBe('write')
  })

  it('writes again after a save when the content goes back to the original', () => {
    const tab = { content: 'old', savedContent: 'old' }
    tab.content = 'new'
    expect(saveDecision(tab)).toBe('write')
    tab.savedContent = tab.content // refreshed after a successful write
    tab.content = 'old'
    expect(saveDecision(tab)).toBe('write')
  })

  it('refuses to empty a file that had content', () => {
    expect(saveDecision({ content: '', savedContent: 'old' })).toBe('refuse')
  })

  it('skips a new empty file', () => {
    expect(saveDecision({ content: '', savedContent: '' })).toBe('skip')
  })

  it('writes when the user types into a new empty file', () => {
    expect(saveDecision({ content: 'hello', savedContent: '' })).toBe('write')
  })
})
