import { describe, expect, it } from 'vitest'
import { toastLifetime } from './toasts.svelte'

describe('toastLifetime', () => {
  it('waits for the user when the message needs an action', () => {
    expect(toastLifetime('info', true)).toBe(0)
    expect(toastLifetime('error', false)).toBe(0)
  })

  it('times out the rest, longest for warnings', () => {
    expect(toastLifetime('success', false)).toBe(3500)
    expect(toastLifetime('info', false)).toBe(3500)
    expect(toastLifetime('warning', false)).toBeGreaterThan(toastLifetime('info', false))
  })
})
