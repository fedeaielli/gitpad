import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { debounce } from './debounce'

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

describe('debounce', () => {
  it('runs once, using the delay resolved at run time', () => {
    const fn = vi.fn()
    let delay = 100
    const d = debounce(fn, () => delay)

    d.run()
    delay = 200
    d.run()

    vi.advanceTimersByTime(100)
    expect(fn).not.toHaveBeenCalled()

    vi.advanceTimersByTime(100)
    expect(fn).toHaveBeenCalledTimes(1)
  })

  it('does nothing after cancel', () => {
    const fn = vi.fn()
    const d = debounce(fn, 50)

    d.run()
    d.cancel()
    vi.advanceTimersByTime(1000)

    expect(fn).not.toHaveBeenCalled()
  })
})
