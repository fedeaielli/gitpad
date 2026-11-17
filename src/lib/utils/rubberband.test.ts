import { describe, expect, it } from 'vitest'
import { rubberband, resist } from './rubberband'

describe('resist', () => {
  it('leaves the value alone inside the bounds', () => {
    expect(resist(300, 150, 900)).toBe(300)
  })

  it('follows past a bound by less than the overshoot', () => {
    // Soft, but still moving: a hard stop would return the bound itself.
    expect(resist(1100, 150, 900)).toBeGreaterThan(900)
    expect(resist(1100, 150, 900)).toBeLessThan(1100)
    expect(resist(0, 150, 900)).toBeLessThan(150)
    expect(resist(0, 150, 900)).toBeGreaterThan(0)
  })

  it('resists more the further it is pulled, but never by a full dimension', () => {
    const near = resist(1000, 150, 900) - 900
    const far = resist(100000, 150, 900) - 900
    expect(far).toBeGreaterThan(near)
    expect(far).toBeLessThan(900)
  })
})

describe('rubberband', () => {
  it('is near-linear for tiny overshoots and saturates at the dimension', () => {
    expect(rubberband(1, 100)).toBeCloseTo(0.55, 2)
    expect(rubberband(1e6, 100)).toBeLessThan(100)
  })
})
