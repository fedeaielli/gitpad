/// The icons we ship, one per extension. Everything else falls back to the
/// generic file icon, so this map only needs the types the app can show.
const BY_EXTENSION: Record<string, string> = {
  c: 'c',
  h: 'cpp',
  cc: 'cpp',
  cpp: 'cpp',
  cxx: 'cpp',
  hh: 'cpp',
  hpp: 'cpp',
  hxx: 'cpp',
  css: 'css',
  go: 'go',
  html: 'html',
  htm: 'html',
  xhtml: 'html',
  java: 'java',
  js: 'js',
  jsx: 'js',
  mjs: 'js',
  cjs: 'js',
  json: 'json',
  jsonc: 'json',
  webmanifest: 'json',
  md: 'markdown',
  markdown: 'markdown',
  mdown: 'markdown',
  pdf: 'pdf',
  py: 'python',
  rs: 'rust',
  sh: 'shell',
  bash: 'shell',
  bashrc: 'shell',
  bash_profile: 'shell',
  bash_aliases: 'shell',
  profile: 'shell',
  zsh: 'shell',
  zshrc: 'shell',
  fish: 'shell',
  sql: 'sql',
  svelte: 'svelte',
  svg: 'svg',
  txt: 'text',
  csv: 'text',
  tsv: 'text',
  toml: 'toml',
  ts: 'typescript',
  tsx: 'reactts',
  vue: 'vue',
  xml: 'xml',
  xsd: 'xml',
  yaml: 'yaml',
  yml: 'yaml',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  gif: 'image',
  webp: 'image',
  bmp: 'image',
  ico: 'image',
  tiff: 'image',
  avif: 'image',
}

const BY_NAME: Record<string, string> = {
  'package.json': 'npm',
  'package-lock.json': 'npm',
  'npm-shrinkwrap.json': 'npm',
  '.npmrc': 'npm',
  '.npmignore': 'npm',
  '.gitignore': 'git',
  '.gitattributes': 'git',
  '.gitconfig': 'git',
  '.gitmodules': 'git',
  '.gitkeep': 'git',
  '.mailmap': 'git',
}

const BY_FOLDER: Record<string, string> = {
  src: 'folder_type_src',
  source: 'folder_type_src',
  sources: 'folder_type_src',
}

/// Icon basename, without `.svg`, matching a file in `src/lib/icons`.
export function iconName(name: string, dir: boolean): string {
  if (dir) return BY_FOLDER[name] ?? 'default_folder'
  const named = BY_NAME[name]
  if (named) return `file_type_${named}`
  const dot = name.lastIndexOf('.')
  if (dot === -1) return 'default_file'
  const type = BY_EXTENSION[name.slice(dot + 1).toLowerCase()]
  return type ? `file_type_${type}` : 'default_file'
}
