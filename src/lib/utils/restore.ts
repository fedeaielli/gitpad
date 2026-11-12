import { invoke } from '@tauri-apps/api/core'
import { applyGitStatus, type GitStatus } from '../stores/git.svelte'
import { ui } from '../stores/ui.svelte'
import { reloadTabFromDisk } from '../editor/bridge'

/// Put one file back to its state at `hash`: status, open tab and tree must
/// all agree afterwards.
export async function restoreFileAt(hash: string, path: string) {
  applyGitStatus(await invoke<GitStatus>('git_restore_file', { hash, path }))
  await reloadTabFromDisk(path)
  ui.treeVersion += 1
}
