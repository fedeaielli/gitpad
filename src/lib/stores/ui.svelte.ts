export interface FileNode {
  name: string
  path: string
  type: 'file' | 'dir'
  children?: FileNode[]
}

export const ui = $state({
  sidebarOpen: true,
  previewMode: 'off' as 'off' | 'split' | 'full',
  commandPaletteOpen: false,
  settingsOpen: false,
  fileTree: [] as FileNode[],
  /** Bumped after clone/rename so the tree reloads without prop drilling. */
  treeVersion: 0,
  editorTheme: 'gitpad-dark',
  cursor: { line: 1, column: 1 },
})
