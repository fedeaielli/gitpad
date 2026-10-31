import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

import { invoke } from '@tauri-apps/api/core'

type GitModule = typeof import('./git.svelte')
let git: GitModule

const status = {
  synced: true,
  modifiedFiles: [] as string[],
  conflicts: [],
  ahead: 0,
  behind: 0,
  diverged: false,
}

beforeEach(async () => {
  vi.resetModules()
  vi.mocked(invoke).mockReset()
  git = await import('./git.svelte')
})

describe('error helpers', () => {
  it('reads Tauri error objects and plain strings', () => {
    expect(git.errorMessage({ code: 'IO_ERROR', message: 'denied' })).toBe('denied')
    expect(git.errorMessage('boom')).toBe('boom')
  })

  it('only treats NOT_FOUND as a missing repository', () => {
    expect(git.isNoRepoError({ code: 'NOT_FOUND', message: 'none' })).toBe(true)
    expect(git.isNoRepoError({ code: 'IO_ERROR', message: 'denied' })).toBe(false)
    expect(git.isNoRepoError('NOT_FOUND')).toBe(false)
  })

  it('ignores auth and missing repo errors, surfaces IO failures', () => {
    expect(git.isIgnorableSyncError('not authenticated')).toBe(true)
    expect(git.isIgnorableSyncError({ code: 'NOT_FOUND', message: 'none' })).toBe(true)
    expect(git.isIgnorableSyncError({ code: 'IO_ERROR', message: 'denied' })).toBe(false)
  })
})

describe('syncNow', () => {
  it('does nothing when no repository is configured', async () => {
    expect(await git.syncNow(null)).toBe(false)
    expect(invoke).not.toHaveBeenCalled()
  })

  it('does nothing while a sync is already running', async () => {
    git.gitStatus.configured = true
    git.gitStatus.syncing = true

    expect(await git.syncNow(null)).toBe(false)
    expect(invoke).not.toHaveBeenCalled()
  })

  it('pushes, marks the status and clears the flag', async () => {
    git.gitStatus.configured = true
    vi.mocked(invoke).mockResolvedValue(status)

    expect(await git.syncNow('msg')).toBe(true)
    expect(invoke).toHaveBeenCalledWith('git_push', { message: 'msg' })
    expect(git.gitStatus.lastPush).toBeInstanceOf(Date)
    expect(git.gitStatus.syncing).toBe(false)
  })

  it('surfaces a real error and keeps the repo configured', async () => {
    git.gitStatus.configured = true
    vi.mocked(invoke).mockRejectedValue({ code: 'IO_ERROR', message: 'denied' })

    expect(await git.syncNow(null)).toBe(false)
    expect(git.gitStatus.error).toBe('denied')
    expect(git.gitStatus.configured).toBe(true)
    expect(git.gitStatus.syncing).toBe(false)
  })
})
