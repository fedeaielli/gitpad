import { describe, expect, it } from 'vitest'
import { renderScale } from './pdf-render'

describe('renderScale', () => {
  it('caps a wide pane at the 1000 px page width', () => {
    expect(renderScale(1600, 800, 2)).toBe(2.5)
  })

  it('uses the pane width when it is narrower', () => {
    expect(renderScale(600, 800, 1)).toBeCloseTo(568 / 800)
  })

  it('caps the device pixel ratio at 2', () => {
    expect(renderScale(1600, 800, 3)).toBe(2.5)
  })

  it('never renders below 320 px', () => {
    expect(renderScale(100, 800, 1)).toBeCloseTo(320 / 800)
  })
})
