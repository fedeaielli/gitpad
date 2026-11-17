import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { pickAndImport } from './import'

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const openMock = vi.mocked(open)
const invokeMock = vi.mocked(invoke)

describe('pickAndImport', () => {
  beforeEach(() => {
    openMock.mockReset()
    invokeMock.mockReset()
  })

  it('does nothing when the picker is cancelled', async () => {
    openMock.mockResolvedValue(null)
    await expect(pickAndImport(true, null)).resolves.toBeNull()
    expect(invokeMock).not.toHaveBeenCalled()
  })

  it('imports the picked source into the given base', async () => {
    openMock.mockResolvedValue('/home/user/notes.md')
    const result = { path: 'notes (1).md', ignoredFiles: 0, ignoredBytes: 0 }
    invokeMock.mockResolvedValue(result)
    await expect(pickAndImport(false, 'inbox')).resolves.toEqual(result)
    expect(invokeMock).toHaveBeenCalledWith('fs_import', {
      source: '/home/user/notes.md',
      destDir: 'inbox',
    })
  })
})
