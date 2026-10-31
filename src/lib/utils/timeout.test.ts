import { afterEach, describe, expect, it, vi } from 'vitest'
import { withTimeout } from './timeout'

describe('withTimeout', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('resolves with the value when the promise settles in time', async () => {
    vi.useFakeTimers()
    await expect(withTimeout(Promise.resolve(42), 1000, 'sync')).resolves.toBe(42)
  })

  it('rejects with a clear message when the promise is too slow', async () => {
    vi.useFakeTimers()
    const pending = withTimeout(new Promise<number>(() => {}), 120_000, 'sync')
    vi.advanceTimersByTime(120_000)
    await expect(pending).rejects.toThrow('sync timed out after 120s')
  })

  it('leaves no timer behind after a resolution', async () => {
    vi.useFakeTimers()
    await withTimeout(Promise.resolve(1), 1000, 'sync')
    expect(vi.getTimerCount()).toBe(0)
  })

  it('propagates the original error when the promise rejects in time', async () => {
    vi.useFakeTimers()
    const boom = new Error('boom')
    await expect(withTimeout(Promise.reject(boom), 1000, 'sync')).rejects.toBe(boom)
  })
})
