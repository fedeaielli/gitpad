export const LANG_MAP: Record<string, string> = {
  md: 'markdown',
  txt: 'plaintext',
  rs: 'rust',
  py: 'python',
  js: 'javascript',
  ts: 'typescript',
  sh: 'shell',
  bash: 'shell',
  json: 'json',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  html: 'html',
  css: 'css',
  cpp: 'cpp',
  c: 'c',
  go: 'go',
  java: 'java',
}

/// Extensions added in Settings take precedence over the built-in map.
let custom: Record<string, string> = {}

export function setCustomLanguages(map: Record<string, string>) {
  custom = map
}

export function languageFor(path: string): string {
  const dot = path.lastIndexOf('.')
  if (dot === -1) return 'plaintext'
  const ext = path.slice(dot + 1).toLowerCase()
  return custom[ext] ?? LANG_MAP[ext] ?? 'plaintext'
}
