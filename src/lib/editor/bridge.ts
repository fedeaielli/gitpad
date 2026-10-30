import type { editor } from 'monaco-editor'

type Instance = editor.IStandaloneCodeEditor

// The editor is loaded lazily, so callers outside EditorPane.svelte go through this bridge.
let current: Instance | null = null
let flush: (() => Promise<void>) | null = null

export function setEditorInstance(next: Instance | null) {
  current = next
}

export function getEditor() {
  return current
}

export function registerFlush(fn: (() => Promise<void>) | null) {
  flush = fn
}

/// Used by the close handshake in App.svelte before the app exits.
export async function flushPendingSave() {
  await flush?.()
}

/// Jumps to a search hit; no-op until the editor chunk has loaded.
export function revealLine(line: number, column: number) {
  if (!current) return
  current.revealLineInCenter(line)
  current.setPosition({ lineNumber: line, column })
  current.focus()
}
