import type { FileNode } from '../stores/ui.svelte'

export type SortMode = 'name' | 'modified' | 'type'

export const SORT_MODES: SortMode[] = ['name', 'modified', 'type']

/// Newest first reads better than oldest first, so "modified" flips the default.
export function defaultDesc(mode: SortMode): boolean {
  return mode === 'modified'
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })

/// Extension without the dot; names with no dot sort before every extension.
function extension(node: FileNode): string {
  const dot = node.name.lastIndexOf('.')
  return dot > 0 ? node.name.slice(dot + 1).toLowerCase() : ''
}

/// Dot files stay hidden unless the options menu enables them.
export function visibleNodes(nodes: FileNode[], showHidden: boolean): FileNode[] {
  return showHidden ? nodes : nodes.filter((node) => !node.name.startsWith('.'))
}

export function sortNodes(
  nodes: FileNode[],
  mode: SortMode,
  desc: boolean,
  foldersFirst: boolean,
): FileNode[] {
  const list = [...nodes]
  const name = (a: FileNode, b: FileNode) => collator.compare(a.name, b.name) * (desc ? -1 : 1)
  let compare: (a: FileNode, b: FileNode) => number
  if (mode === 'modified') {
    compare = (a, b) => ((a.modified ?? 0) - (b.modified ?? 0)) * (desc ? -1 : 1) || name(a, b)
  } else if (mode === 'type') {
    compare = (a, b) => collator.compare(extension(a), extension(b)) * (desc ? -1 : 1) || name(a, b)
  } else {
    compare = name
  }
  // Grouping is not an ordering: the direction never flips it.
  const dirsFirst = (a: FileNode, b: FileNode) =>
    Number(b.type === 'dir') - Number(a.type === 'dir')
  return list.sort((a, b) => (foldersFirst ? dirsFirst(a, b) || compare(a, b) : compare(a, b)))
}
