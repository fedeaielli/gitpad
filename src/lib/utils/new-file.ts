/// Frontend pre-check for Ctrl+N. `resolve_in` in Rust stays the real defence.
export function newFileError(name: string, existing: string[]): string | null {
  const path = name.trim()
  if (!path) return null
  const segments = path.split('/')
  if (path.startsWith('/') || segments.includes('..') || segments.includes('.git')) {
    return 'invalid file name'
  }
  if (path.endsWith('/')) return 'a file name is required'
  if (existing.includes(path)) return `${path} already exists`
  return null
}
