/// Labels and keys shown in the menu, the palette and the Ctrl+K dialog.
/// shortcut: display only, App.svelte still matches keys by hand; move the
/// handler onto this table if a third place ever needs to dispatch shortcuts.
export interface Shortcut {
  label: string
  keys: string
  /// Only these appear in the Ctrl+K dialog, in this order.
  help?: boolean
}

export const SHORTCUTS = {
  palette: { label: 'Command palette', keys: 'Ctrl+P', help: true },
  newFile: { label: 'New file', keys: 'Ctrl+N', help: true },
  importFile: { label: 'Import file', keys: 'Ctrl+O', help: true },
  saveSync: { label: 'Save and sync', keys: 'Ctrl+S', help: true },
  format: { label: 'Format document', keys: 'Ctrl+Shift+I', help: true },
  preview: { label: 'Toggle markdown preview', keys: 'Ctrl+Shift+P', help: true },
  find: { label: 'Find', keys: 'Ctrl+F', help: true },
  findInFiles: { label: 'Find in files', keys: 'Ctrl+Shift+F', help: true },
  replace: { label: 'Find and replace', keys: 'Ctrl+R', help: true },
  goToLine: { label: 'Go to line', keys: 'Ctrl+G', help: true },
  lineComment: { label: 'Toggle line comment', keys: 'Ctrl+/', help: true },
  blockComment: { label: 'Toggle block comment', keys: 'Shift+Alt+A', help: true },
  closeTab: { label: 'Close tab', keys: 'Ctrl+W', help: true },
  history: { label: 'File history', keys: 'Ctrl+H', help: true },
  shortcuts: { label: 'Keyboard shortcuts', keys: 'Ctrl+K', help: true },
  closeDialog: { label: 'Close dialog', keys: 'Esc', help: true },
  undo: { label: 'Undo', keys: 'Ctrl+Z', help: false },
  redo: { label: 'Redo', keys: 'Ctrl+Y', help: false },
  cut: { label: 'Cut', keys: 'Ctrl+X', help: false },
  copy: { label: 'Copy', keys: 'Ctrl+C', help: false },
  paste: { label: 'Paste', keys: 'Ctrl+V', help: false },
} satisfies Record<string, Shortcut>

export function keyOf(id: keyof typeof SHORTCUTS): string {
  return SHORTCUTS[id].keys
}

export const helpRows = Object.values(SHORTCUTS).filter((shortcut) => shortcut.help)
