import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { coalesceChanges } from './fs-changes'

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

describe('coalesceChanges', () => {
  it('merges a burst into one refresh after the quiet window', async () => {
    const apply = vi.fn(async () => {})
    const changes = coalesceChanges(apply, 1000)

    changes.add([{ path: 'a', status: 'A' }])
    vi.advanceTimersByTime(500)
    changes.add([{ path: 'b', status: 'A' }])
    vi.advanceTimersByTime(999)
    expect(apply).not.toHaveBeenCalled()

    await vi.advanceTimersByTimeAsync(1)

    expect(apply).toHaveBeenCalledTimes(1)
    expect(apply).toHaveBeenCalledWith([
      { path: 'a', status: 'A' },
      { path: 'b', status: 'A' },
    ])
  })

  it('keeps the first position and the latest status', async () => {
    const apply = vi.fn(async () => {})
    const changes = coalesceChanges(apply, 1000)

    changes.add([
      { path: 'a', status: 'A' },
      { path: 'b', status: 'M' },
    ])
    changes.add([{ path: 'a', status: 'M' }])

    await vi.advanceTimersByTimeAsync(1000)

    expect(apply).toHaveBeenCalledWith([
      { path: 'a', status: 'M' },
      { path: 'b', status: 'M' },
    ])
  })

  it('does not overlap refreshes and drains what arrived meanwhile', async () => {
    let release = () => {}
    const apply = vi
      .fn<(changes: { path: string; status: string }[]) => Promise<void>>()
      .mockImplementationOnce(
        () => new Promise<void>((resolve) => (release = resolve)),
      )
      .mockResolvedValue(undefined)
    const changes = coalesceChanges(apply, 1000)

    changes.add([{ path: 'a', status: 'A' }])
    await vi.advanceTimersByTimeAsync(1000)
    expect(apply).toHaveBeenCalledTimes(1)

    // This batch lands while the first refresh is still running.
    changes.add([{ path: 'b', status: 'A' }])
    await vi.advanceTimersByTimeAsync(1000)
    expect(apply).toHaveBeenCalledTimes(1)

    release()
    await vi.advanceTimersByTimeAsync(0)

    expect(apply).toHaveBeenCalledTimes(2)
    expect(apply).toHaveBeenLastCalledWith([{ path: 'b', status: 'A' }])
  })
})
