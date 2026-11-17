/// Links between Markdown notes. Paths are repo-relative and have no leading
/// slash, the same form the file tree and the tabs use.
export function noteRelativePath(from: string, to: string): string {
  const fromDir = from.split('/').slice(0, -1)
  const toParts = to.split('/')
  let common = 0
  while (
    common < fromDir.length &&
    common < toParts.length - 1 &&
    fromDir[common] === toParts[common]
  ) {
    common++
  }
  return [...fromDir.slice(common).map(() => '..'), ...toParts.slice(common)].join('/')
}

/// Pick-list order: the current folder first, then every other note by path.
/// The source note is left out: a link to itself is never useful.
export function noteOrder(paths: string[], from: string): string[] {
  const folder = from.split('/').slice(0, -1).join('/')
  const near: string[] = []
  const far: string[] = []
  for (const path of paths) {
    if (path === from) continue
    const dir = path.split('/').slice(0, -1).join('/')
    ;(dir === folder ? near : far).push(path)
  }
  const byPath = (a: string, b: string) => a.localeCompare(b, undefined, { numeric: true })
  return [...near.sort(byPath), ...far.sort(byPath)]
}

/// The note name is the link text: the folder already shows in the path.
export function noteLinkLabel(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1).replace(/\.md$/i, '')
}

/// shortcut: the destination is percent-encoded per segment so spaces and
/// parentheses survive; a file name containing `%2F` would decode as a folder.
export function noteLink(from: string, to: string): string {
  const destination = noteRelativePath(from, to)
    .split('/')
    .map((part) => encodeURIComponent(part).replace(/\(/g, '%28').replace(/\)/g, '%29'))
    .join('/')
  return `[${noteLinkLabel(to)}](${destination})`
}

/// Repo-relative target of a link written in `from`, or null when the link is
/// external or escapes the repo. Anchors and queries are dropped.
export function resolveNoteLink(from: string, href: string): string | null {
  const clean = href.split('#')[0].split('?')[0]
  if (!clean || /^[a-z][a-z0-9+.-]*:/i.test(clean)) return null
  const parts = clean.startsWith('/') ? [] : from.split('/').slice(0, -1)
  for (const raw of clean.split('/')) {
    let part: string
    try {
      part = decodeURIComponent(raw)
    } catch {
      return null
    }
    if (!part || part === '.') continue
    if (part === '..') {
      if (parts.length === 0) return null
      parts.pop()
    } else {
      parts.push(part)
    }
  }
  return parts.join('/')
}
