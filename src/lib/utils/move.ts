/// A drag lands on a folder: it cannot take itself, its own subtree, or the
/// folder the entry already lives in (that drop would be a no-op).
export function canMoveInto(from: string, folder: string): boolean {
  return folder !== from && !folder.startsWith(`${from}/`) && parentOf(from) !== folder
}

/// The path an entry gets inside `folder`; `folder` is empty for the repo root.
export function moveTarget(from: string, folder: string): string {
  const name = from.split('/').pop() ?? from
  return folder ? `${folder}/${name}` : name
}

function parentOf(path: string): string {
  return path.split('/').slice(0, -1).join('/')
}
