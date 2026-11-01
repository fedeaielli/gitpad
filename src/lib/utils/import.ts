import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

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
export function importSource(source: string, base: string | null): Promise<string> {
  return invoke<string>('fs_import', { source, destDir: base })
}

export async function pickAndImport(directory: boolean, base: string | null): Promise<string | null> {
  const source = await pickImportSource(directory)
  if (!source) return null
  return importSource(source, base)
}
