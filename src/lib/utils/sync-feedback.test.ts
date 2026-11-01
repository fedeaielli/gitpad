import { describe, expect, it } from 'vitest'
import { syncOutcome, type SyncOutcomeInput } from './sync-feedback'

const base: SyncOutcomeInput = {
  saveOk: true,
  saveError: null,
  wrote: true,
  hadChanges: false,
  hadAhead: false,
  pushed: true,
  conflicts: false,
  error: null,
}

describe('syncOutcome', () => {
  it('reports a full save and push', () => {
    expect(syncOutcome(base)).toEqual({ kind: 'success', message: 'Saved and pushed' })
  })

  it('reports nothing to push when nothing was written or pending', () => {
    expect(syncOutcome({ ...base, wrote: false })).toEqual({
      kind: 'info',
      message: 'Saved, nothing to push',
    })
  })

  it('asks for the conflicts before anything else', () => {
    expect(syncOutcome({ ...base, conflicts: true, pushed: false })).toEqual({
      kind: 'error',
      message: 'Resolve the conflicts first',
    })
  })

  it('keeps the local commit visible when the push failed', () => {
    expect(syncOutcome({ ...base, pushed: false, error: 'network down' })).toEqual({
      kind: 'error',
      message: 'Saved, push failed: network down. Your commit is safe locally.',
    })
  })

  it('never claims a push when the save failed', () => {
    const out = syncOutcome({
      ...base,
      saveOk: false,
      saveError: 'save failed: permission denied',
      wrote: false,
      pushed: false,
    })
    expect(out.kind).toBe('error')
    expect(out.message).toContain('save failed: permission denied')
    expect(out.message).toContain('will be pushed automatically')
    expect(out.message).not.toContain('Saved and pushed')
  })
})
