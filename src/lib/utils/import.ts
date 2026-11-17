import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ui } from '../stores/ui.svelte'
import { pushToast } from '../stores/toasts.svelte'

export interface ImportResult {
  path: string
  ignoredFiles: number
  ignoredBytes: number
}

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
export async function importSource(source: string, base: string | null): Promise<ImportResult> {
  ui.importing = true
  try {
    return await invoke<ImportResult>('fs_import', { source, destDir: base })
  } finally {
    ui.importing = false
    ui.importPct = null
    ui.importCommitting = false
  }
}

export async function pickAndImport(
  directory: boolean,
  base: string | null,
): Promise<ImportResult | null> {
  const source = await pickImportSource(directory)
  if (!source) return null
  return importSource(source, base)
}

/// GitHub rejects files over 100 MiB; say so when the import had to skip them.
export function notifyIgnored(result: ImportResult) {
  if (result.ignoredFiles === 0) return
  const count =
    result.ignoredFiles === 1 ? '1 file' : `${result.ignoredFiles} files`
  pushToast(`${count} over 100 MB kept local and not synced`, 'warning')
}
