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

export function languageFor(path: string): string {
  const dot = path.lastIndexOf('.')
  if (dot === -1) return 'plaintext'
  return LANG_MAP[path.slice(dot + 1).toLowerCase()] ?? 'plaintext'
}
