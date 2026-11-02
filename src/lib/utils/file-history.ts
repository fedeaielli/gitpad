import type { FileChange } from '../stores/git.svelte'

/// The newest commit touching a file holds its current content only when the
/// file has nothing pending: a dirty buffer or a working-tree change means the
/// disk no longer matches any commit, so restoring would still do something.
export function isCurrentVersion(
  index: number,
  path: string,
  changes: FileChange[],
  dirty: boolean,
): boolean {
  return index === 0 && !dirty && !changes.some((change) => change.path === path)
}
