import type { editor } from 'monaco-editor'
import type { UpstreamChange } from '../utils/pull-reload'

type Instance = editor.IStandaloneCodeEditor

// The editor is loaded lazily, so callers outside EditorPane.svelte go through this bridge.
let current: Instance | null = null
let flush: (() => Promise<boolean>) | null = null
let reload: ((changes: UpstreamChange[]) => Promise<void>) | null = null

export function setEditorInstance(next: Instance | null) {
  current = next
}

export function getEditor() {
  return current
}

export function registerFlush(fn: (() => Promise<boolean>) | null) {
  flush = fn
}

/// Used by the close handshake in App.svelte before the app exits.
export async function flushPendingSave() {
  return flush ? await flush() : true
}

export function registerReload(fn: ((changes: UpstreamChange[]) => Promise<void>) | null) {
  reload = fn
}

/// Used by App.svelte when a pull rewrote files that have open tabs.
export async function reloadOpenTabs(changes: UpstreamChange[]) {
  if (reload) await reload(changes)
}

/// Jumps to a search hit; no-op until the editor chunk has loaded.
export function revealLine(line: number, column: number) {
  if (!current) return
  current.revealLineInCenter(line)
  current.setPosition({ lineNumber: line, column })
  current.focus()
}
