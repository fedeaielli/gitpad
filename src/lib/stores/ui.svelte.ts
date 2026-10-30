import { tabs } from './tabs.svelte'

export const THEMES = ['gitpad-dark', 'vs-dark', 'vs', 'hc-black']

export type Keybinding = 'vscode' | 'vim' | 'emacs'

export const KEYBINDINGS: Keybinding[] = ['vscode', 'vim', 'emacs']

/// Matches the default map in src-tauri/src/config.rs.
export const DEFAULT_EXTENSIONS: Record<string, string> = {
  txt: 'plaintext',
  md: 'markdown',
  json: 'json',
  yaml: 'yaml',
  toml: 'toml',
  sh: 'shell',
  py: 'python',
  rs: 'rust',
  js: 'javascript',
  ts: 'typescript',
}

export interface FileNode {
  name: string
  path: string
  type: 'file' | 'dir'
  children?: FileNode[]
}

export const ui = $state({
  sidebarOpen: true,
  activeView: 'files' as 'files' | 'git',
  previewMode: 'off' as 'off' | 'split' | 'full',
  commandPaletteOpen: false,
  settingsOpen: false,
  fileTree: [] as FileNode[],
  /** Bumped after clone/rename so the tree reloads without prop drilling. */
  treeVersion: 0,
  conflictsHidden: false,
  editorTheme: 'gitpad-dark',
  fontSize: 13,
  fontFamily: 'ui-monospace, monospace',
  autoSync: true,
  saveDebounceMs: 2000,
  pushIdleMs: 30_000,
  keybindings: 'vscode' as Keybinding,
  extensions: { ...DEFAULT_EXTENSIONS },
  cursor: { line: 1, column: 1 },
  onboardingComplete: false,
})

/// off → split → full → off, only while a markdown tab is active.
export function cyclePreview() {
  const active = tabs.tabs.find((t) => t.id === tabs.activeTabId)
  if (active?.language !== 'markdown') return
  ui.previewMode = ui.previewMode === 'off' ? 'split' : ui.previewMode === 'split' ? 'full' : 'off'
}
