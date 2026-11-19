import { describe, expect, it } from 'vitest'
import { TOAST_GAP, TOAST_PEEK, stackPlacement } from './toastStack'

const heights = { 1: 60, 2: 80, 3: 100 }

describe('stackPlacement', () => {
  it('tucks older cards behind the newest one', () => {
    expect(stackPlacement(2, [1, 2, 3], heights, false)).toEqual({ offsetY: 0, scale: 1 })
    expect(stackPlacement(1, [1, 2, 3], heights, false)).toEqual({
      offsetY: 100 + TOAST_PEEK - 0.9 * 80,
      scale: 0.9,
    })
    expect(stackPlacement(0, [1, 2, 3], heights, false)).toEqual({
      offsetY: 100 + TOAST_PEEK * 2 - 0.8 * 60,
      scale: 0.8,
    })
  })

  it('stacks full size above the newest card when expanded', () => {
    expect(stackPlacement(2, [1, 2, 3], heights, true)).toEqual({ offsetY: 0, scale: 1 })
    expect(stackPlacement(1, [1, 2, 3], heights, true)).toEqual({
      offsetY: 100 + TOAST_GAP,
      scale: 1,
    })
    expect(stackPlacement(0, [1, 2, 3], heights, true)).toEqual({
      offsetY: 100 + 80 + TOAST_GAP * 2,
      scale: 1,
    })
  })
})
