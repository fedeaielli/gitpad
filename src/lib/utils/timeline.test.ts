import { describe, expect, it } from 'vitest'
import { clampTimelineHeight, MAX_TIMELINE, MIN_TIMELINE } from './timeline'

describe('clampTimelineHeight', () => {
  it('keeps the history between its own bounds', () => {
    expect(clampTimelineHeight(900, 50)).toBe(MIN_TIMELINE)
    expect(clampTimelineHeight(900, 900)).toBe(MAX_TIMELINE)
    expect(clampTimelineHeight(900, 360)).toBe(360)
  })

  it('leaves room for the editor when the column is short', () => {
    expect(clampTimelineHeight(500, 480)).toBe(340)
    expect(clampTimelineHeight(200, 600)).toBe(MIN_TIMELINE)
  })
})
