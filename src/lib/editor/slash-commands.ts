/// A pressed `/command` in a Markdown note. Pure data and string building:
/// the Monaco wiring lives in slash-register.ts, so this file is testable alone.
export type Built = { text: string; select: [number, number] }

export type SlashCommand = {
  id: string
  detail: string
  build: (selected: string, language: string) => Built
}

/// Ordered by how often a note needs them, not alphabetically.
export const SLASH_COMMANDS: SlashCommand[] = [
  { id: 'h1', detail: 'Heading 1', build: heading(1) },
  { id: 'h2', detail: 'Heading 2', build: heading(2) },
  { id: 'h3', detail: 'Heading 3', build: heading(3) },
  { id: 'bold', detail: 'Bold', build: wrap('**', '**', 'bold') },
  { id: 'italic', detail: 'Italic', build: wrap('*', '*', 'italic') },
  { id: 'strike', detail: 'Strikethrough', build: wrap('~~', '~~', 'text') },
  { id: 'inline', detail: 'Inline code', build: wrap('`', '`', 'code') },
  { id: 'code', detail: 'Code block', build: codeBlock },
  { id: 'link', detail: 'Link', build: link },
  { id: 'image', detail: 'Image', build: image },
  { id: 'list', detail: 'Bullet list', build: prefixed('- ') },
  { id: 'sublist', detail: 'Nested bullet', build: prefixed('  - ') },
  { id: 'number', detail: 'Numbered list', build: numberedLines('') },
  { id: 'subnumber', detail: 'Nested numbered list', build: numberedLines('  ') },
  { id: 'task', detail: 'Task list', build: prefixed('- [ ] ') },
  { id: 'quote', detail: 'Blockquote', build: prefixed('> ') },
  { id: 'table', detail: 'Table', build: table },
  { id: 'hr', detail: 'Horizontal rule', build: () => ({ text: '---\n', select: [4, 4] }) },
]

/// highlight.js names, so the preview colours the fence without a second map.
export const CODE_LANGUAGES = [
  'text',
  'rust',
  'typescript',
  'javascript',
  'python',
  'bash',
  'json',
  'yaml',
  'toml',
  'html',
  'css',
  'sql',
  'go',
  'c',
  'cpp',
  'java',
]

/// The `/word` typed at the start of a line, or null. Last line only: the
/// caller passes the line content up to the cursor.
export function slashQuery(lineText: string): { start: number; query: string } | null {
  const match = /^[ \t]{0,3}\/([a-zA-Z0-9]*)$/.exec(lineText)
  if (!match) return null
  return { start: lineText.indexOf('/'), query: match[1].toLowerCase() }
}

function heading(level: number) {
  return (selected: string): Built => {
    const mark = '#'.repeat(level) + ' '
    const text = mark + (selected || 'Heading')
    return { text, select: selected ? [text.length, text.length] : [mark.length, text.length] }
  }
}

function wrap(open: string, close: string, placeholder: string) {
  return (selected: string): Built => {
    const inner = selected || placeholder
    const text = open + inner + close
    return { text, select: selected ? [text.length, text.length] : [open.length, open.length + inner.length] }
  }
}

/// Only non-empty lines take the prefix, so a blank line stays a paragraph break.
function prefixed(prefix: string) {
  return (selected: string): Built => {
    if (!selected) return { text: prefix, select: [prefix.length, prefix.length] }
    const text = selected
      .split('\n')
      .map((line) => (line.trim() === '' ? line : prefix + line))
      .join('\n')
    return { text, select: [0, text.length] }
  }
}

function numberedLines(indent: string) {
  return (selected: string): Built => {
    if (!selected) {
      const text = `${indent}1. `
      return { text, select: [text.length, text.length] }
    }
    let n = 0
    const text = selected
      .split('\n')
      .map((line) => (line.trim() === '' ? line : `${indent}${++n}. ${line}`))
      .join('\n')
    return { text, select: [0, text.length] }
  }
}

function codeBlock(selected: string, language: string): Built {
  const lang = language || 'text'
  const open = '```' + lang + '\n'
  if (!selected) {
    const text = open + '\n```'
    return { text, select: [open.length, open.length] }
  }
  const text = open + selected + '\n```'
  return { text, select: [open.length, open.length + selected.length] }
}

function link(selected: string): Built {
  if (!selected) return { text: '[text](https://)', select: [1, 5] }
  const start = selected.length + 3
  return { text: `[${selected}](https://)`, select: [start, start + 8] }
}

function image(selected: string): Built {
  if (!selected) return { text: '![image](https://)', select: [2, 7] }
  const start = selected.length + 4
  return { text: `![${selected}](https://)`, select: [start, start + 8] }
}

function table(): Built {
  return {
    text: '| Column | Column |\n| --- | --- |\n| Text | Text |',
    select: [2, 8],
  }
}
