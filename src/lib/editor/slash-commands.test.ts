import { describe, expect, it } from 'vitest'
import { SLASH_COMMANDS, slashQuery, type SlashCommand } from './slash-commands'

function cmd(id: string): SlashCommand {
  const found = SLASH_COMMANDS.find((candidate) => candidate.id === id)
  if (!found) throw new Error(`missing command ${id}`)
  return found
}

describe('slashQuery', () => {
  it('only fires at the start of a line', () => {
    expect(slashQuery('/bo')).toEqual({ start: 0, query: 'bo', cell: false })
    expect(slashQuery('   /code')).toEqual({ start: 3, query: 'code', cell: false })
    expect(slashQuery('/')).toEqual({ start: 0, query: '', cell: false })
    expect(slashQuery('/h1')).toEqual({ start: 0, query: 'h1', cell: false })
  })

  it('ignores slashes in the middle of a line or inside a path', () => {
    expect(slashQuery('a /bo')).toBeNull()
    expect(slashQuery('src/lib')).toBeNull()
    expect(slashQuery('https://example.com')).toBeNull()
    expect(slashQuery('      /deep')).toBeNull()
  })

  it('fires at the start of a table cell', () => {
    expect(slashQuery('| /col')).toEqual({ start: 2, query: 'col', cell: true })
    expect(slashQuery('| a | /bo')).toEqual({ start: 6, query: 'bo', cell: true })
    expect(slashQuery('| a /bo |')).toBeNull()
    expect(slashQuery('| /usr/bin')).toBeNull()
    expect(slashQuery('text | /col')).toBeNull()
  })
})

describe('inline commands', () => {
  it('wraps a selection and keeps the caret after it', () => {
    expect(cmd('bold').build('ciao', 'text')).toEqual({ text: '**ciao**', select: [8, 8] })
    expect(cmd('italic').build('ciao', 'text').text).toBe('*ciao*')
    expect(cmd('strike').build('ciao', 'text').text).toBe('~~ciao~~')
    expect(cmd('inline').build('ciao', 'text').text).toBe('`ciao`')
  })

  it('selects the placeholder when nothing is selected', () => {
    expect(cmd('bold').build('', 'text')).toEqual({ text: '**bold**', select: [2, 6] })
    expect(cmd('h2').build('', 'text')).toEqual({ text: '## Heading', select: [3, 10] })
  })

  it('puts the caret on the URL of a link', () => {
    expect(cmd('link').build('', 'text')).toEqual({ text: '[text](https://)', select: [1, 5] })
    expect(cmd('link').build('ciao', 'text')).toEqual({ text: '[ciao](https://)', select: [7, 15] })
    expect(cmd('image').build('logo', 'text')).toEqual({ text: '![logo](https://)', select: [8, 16] })
  })
})

describe('block commands', () => {
  it('opens a fence with the last language and puts the caret inside', () => {
    expect(cmd('code').build('', 'rust')).toEqual({ text: '```rust\n\n```', select: [8, 8] })
  })

  it('fences a selection with the current language', () => {
    const built = cmd('code').build('fn main() {}', 'rust')
    expect(built.text).toBe('```rust\nfn main() {}\n```')
    expect(built.text.slice(built.select[0], built.select[1])).toBe('fn main() {}')
  })

  it('prefixes every non-empty line', () => {
    expect(cmd('list').build('a\n\nb', 'text')).toEqual({ text: '- a\n\n- b', select: [0, 8] })
    expect(cmd('quote').build('a\nb', 'text').text).toBe('> a\n> b')
    expect(cmd('task').build('a', 'text').text).toBe('- [ ] a')
  })

  it('numbers the selected lines in order', () => {
    expect(cmd('number').build('a\nb\nc', 'text').text).toBe('1. a\n2. b\n3. c')
    expect(cmd('number').build('', 'text').text).toBe('1. ')
  })

  it('indents the nested lists by two spaces', () => {
    expect(cmd('sublist').build('a\nb', 'text').text).toBe('  - a\n  - b')
    expect(cmd('sublist').build('', 'text')).toEqual({ text: '  - ', select: [4, 4] })
    expect(cmd('subnumber').build('a\nb', 'text').text).toBe('  1. a\n  2. b')
  })

  it('inserts a table or a rule on their own', () => {
    expect(cmd('table').build('', 'text').text).toContain('| --- | --- |')
    expect(cmd('hr').build('', 'text').text).toBe('---\n')
  })
})
