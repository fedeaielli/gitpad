import { describe, expect, it } from 'vitest'
import { stepIndex } from './select'

describe('stepIndex', () => {
  it('steps down and wraps at the end', () => {
    expect(stepIndex(0, 3, 'ArrowDown')).toBe(1)
    expect(stepIndex(2, 3, 'ArrowDown')).toBe(0)
  })

  it('steps up and wraps at the start', () => {
    expect(stepIndex(1, 3, 'ArrowUp')).toBe(0)
    expect(stepIndex(0, 3, 'ArrowUp')).toBe(2)
  })

  it('jumps to either end', () => {
    expect(stepIndex(1, 4, 'Home')).toBe(0)
    expect(stepIndex(1, 4, 'End')).toBe(3)
  })

  it('opens from "nothing highlighted" at the first entry', () => {
    expect(stepIndex(-1, 3, 'ArrowDown')).toBe(0)
  })

  it('ignores keys it does not own, and an empty list', () => {
    expect(stepIndex(0, 3, 'Enter')).toBeNull()
    expect(stepIndex(0, 0, 'ArrowDown')).toBeNull()
  })
})
