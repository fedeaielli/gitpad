import { describe, expect, it } from 'vitest'
import { trapTarget } from './focus'

describe('trapTarget', () => {
  it('lets focus move between items', () => {
    expect(trapTarget(1, 4, false)).toBeNull()
    expect(trapTarget(2, 4, true)).toBeNull()
  })

  it('wraps forward from the last item and back from the first', () => {
    expect(trapTarget(3, 4, false)).toBe(0)
    expect(trapTarget(0, 4, true)).toBe(3)
  })

  it('pulls an escaped focus back into the dialog', () => {
    expect(trapTarget(-1, 4, false)).toBe(0)
    expect(trapTarget(-1, 4, true)).toBe(3)
  })

  it('does nothing when the dialog has no focusable controls', () => {
    expect(trapTarget(-1, 0, false)).toBeNull()
  })
})
