export type ViewerKind = 'image' | 'pdf' | 'docx' | 'odt'

const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'avif', 'ico'])

const VIEWERS: Record<string, ViewerKind> = {
  pdf: 'pdf',
  docx: 'docx',
  odt: 'odt',
}
for (const ext of IMAGE_EXTENSIONS) VIEWERS[ext] = 'image'

/// The read-only viewer a file needs, or null when it is text.
export function viewerFor(path: string): ViewerKind | null {
  const dot = path.lastIndexOf('.')
  if (dot === -1) return null
  return VIEWERS[path.slice(dot + 1).toLowerCase()] ?? null
}

export function imageMime(path: string): string {
  const ext = path.slice(path.lastIndexOf('.') + 1).toLowerCase()
  if (ext === 'svg') return 'image/svg+xml'
  if (ext === 'jpg' || ext === 'jpeg') return 'image/jpeg'
  if (ext === 'ico') return 'image/x-icon'
  return `image/${ext}`
}
