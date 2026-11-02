import type { FileNode } from '../stores/ui.svelte'

export type SortMode = 'name' | 'folders' | 'modified' | 'type'

export const SORT_MODES: SortMode[] = ['name', 'modified', 'type', 'folders']

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

export function sortNodes(nodes: FileNode[], mode: SortMode, desc: boolean): FileNode[] {
  const list = [...nodes]
  const name = (a: FileNode, b: FileNode) => collator.compare(a.name, b.name) * (desc ? -1 : 1)
  if (mode === 'modified') {
    return list.sort((a, b) => ((a.modified ?? 0) - (b.modified ?? 0)) * (desc ? -1 : 1) || name(a, b))
  }
  if (mode === 'type') {
    return list.sort(
      (a, b) =>
        Number(b.type === 'dir') - Number(a.type === 'dir') ||
        collator.compare(extension(a), extension(b)) * (desc ? -1 : 1) ||
        name(a, b),
    )
  }
  // Folders stay on top: grouping is not an ordering, so the direction never flips it.
  if (mode === 'folders') {
    return list.sort(
      (a, b) => Number(b.type === 'dir') - Number(a.type === 'dir') || name(a, b),
    )
  }
  return list.sort(name)
}
