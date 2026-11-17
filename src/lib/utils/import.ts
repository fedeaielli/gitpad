import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ui } from '../stores/ui.svelte'

/// Native picker only; null when the user cancels.
export async function pickImportSource(directory: boolean): Promise<string | null> {
  const source = await open({
    multiple: false,
    directory,
    title: directory ? 'Import folder' : 'Import file',
  })
  return typeof source === 'string' ? source : null
}

/// Copies into `base` (null = repo root) and returns the path relative to the repo.
export async function importSource(source: string, base: string | null): Promise<string> {
  ui.importing = true
  try {
    return await invoke<string>('fs_import', { source, destDir: base })
  } finally {
    ui.importing = false
    ui.importPct = null
    ui.importCommitting = false
  }
}

export async function pickAndImport(directory: boolean, base: string | null): Promise<string | null> {
  const source = await pickImportSource(directory)
  if (!source) return null
  return importSource(source, base)
}
