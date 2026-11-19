import { describe, expect, it } from 'vitest'
import { toastLifetime, type ToastKind } from './toasts.svelte'

describe('toastLifetime', () => {
  it('gives every severity a timeout', () => {
    for (const kind of ['info', 'success', 'warning', 'error'] as ToastKind[]) {
      expect(toastLifetime(kind, false)).toBeGreaterThan(0)
    }
  })

  it('keeps the graver toast on screen longer', () => {
    expect(toastLifetime('info', false)).toBe(4000)
    expect(toastLifetime('success', false)).toBe(4500)
    expect(toastLifetime('warning', false)).toBe(6500)
    expect(toastLifetime('error', false)).toBe(9000)
  })

  it('adds reading time when there is an action', () => {
    expect(toastLifetime('info', true)).toBe(5000)
    expect(toastLifetime('success', true)).toBe(5500)
    expect(toastLifetime('warning', true)).toBe(8000)
    expect(toastLifetime('error', true)).toBe(10500)
  })
})
