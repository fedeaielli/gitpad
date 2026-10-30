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
})
