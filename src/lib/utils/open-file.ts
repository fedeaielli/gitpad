import { invoke } from '@tauri-apps/api/core'
import { activateTab, openTab, tabs } from '../stores/tabs.svelte'
import { languageFor } from './langmap'

/// Opens a file in a tab, or focuses it when it is already open. Throws on read errors.
export async function openFileInTab(path: string): Promise<void> {
  if (tabs.tabs.some((t) => t.id === path)) {
    activateTab(path)
    return
  }
  const content = await invoke<string>('fs_read_file', { path })
  openTab({ id: path, path, content, modified: false, language: languageFor(path) })
}
