import { describe, expect, it } from 'vitest'
import type { FileNode } from '../stores/ui.svelte'
import { defaultDesc, sortNodes, visibleNodes } from './file-sort'

const node = (name: string, type: 'file' | 'dir', modified = 0): FileNode => ({
  name,
  path: name,
  type,
  modified,
})

const names = (nodes: FileNode[]) => nodes.map((n) => n.name)

describe('visibleNodes', () => {
  it('drops dot files unless they are enabled', () => {
    const nodes = [node('a.md', 'file'), node('.env', 'file'), node('.git', 'dir')]
    expect(names(visibleNodes(nodes, false))).toEqual(['a.md'])
    expect(names(visibleNodes(nodes, true))).toEqual(['a.md', '.env', '.git'])
  })
})

describe('sortNodes', () => {
  const files = [node('b.md', 'file', 10), node('a.md', 'file', 30), node('c.md', 'file', 20)]

  it('sorts by name in both directions', () => {
    expect(names(sortNodes(files, 'name', false, false))).toEqual(['a.md', 'b.md', 'c.md'])
    expect(names(sortNodes(files, 'name', true, false))).toEqual(['c.md', 'b.md', 'a.md'])
  })

  it('sorts by modification in both directions', () => {
    expect(names(sortNodes(files, 'modified', true, false))).toEqual(['a.md', 'c.md', 'b.md'])
    expect(names(sortNodes(files, 'modified', false, false))).toEqual(['b.md', 'c.md', 'a.md'])
  })

  it('keeps folders on top without changing the chosen order', () => {
    const mixed = [node('a.md', 'file'), node('z-dir', 'dir'), node('b-directory', 'dir')]
    expect(names(sortNodes(mixed, 'name', false, true))).toEqual([
      'b-directory',
      'z-dir',
      'a.md',
    ])
    expect(names(sortNodes(mixed, 'name', true, true))).toEqual(['z-dir', 'b-directory', 'a.md'])
  })

  it('keeps folders on top with the modified order', () => {
    const mixed = [
      node('new.md', 'file', 30),
      node('old-dir', 'dir', 10),
      node('mid.md', 'file', 20),
      node('new-dir', 'dir', 40),
    ]
    expect(names(sortNodes(mixed, 'modified', true, true))).toEqual([
      'new-dir',
      'old-dir',
      'new.md',
      'mid.md',
    ])
  })

  it('does not mutate the input', () => {
    const input = [node('b.md', 'file'), node('a.md', 'file')]
    sortNodes(input, 'name', false, false)
    expect(names(input)).toEqual(['b.md', 'a.md'])
  })

  it('groups files by extension, both directions', () => {
    const mixed = [
      node('notes', 'file'),
      node('b.py', 'file'),
      node('a.md', 'file'),
      node('src', 'dir'),
      node('z.rs', 'file'),
    ]
    expect(names(sortNodes(mixed, 'type', false, false))).toEqual([
      'notes',
      'src',
      'a.md',
      'b.py',
      'z.rs',
    ])
    expect(names(sortNodes(mixed, 'type', true, false))).toEqual([
      'z.rs',
      'b.py',
      'a.md',
      'src',
      'notes',
    ])
  })

  it('combines the type order with folders first', () => {
    const mixed = [
      node('notes', 'file'),
      node('b.py', 'file'),
      node('a.md', 'file'),
      node('src', 'dir'),
      node('z.rs', 'file'),
    ]
    expect(names(sortNodes(mixed, 'type', false, true))).toEqual([
      'src',
      'notes',
      'a.md',
      'b.py',
      'z.rs',
    ])
    expect(names(sortNodes(mixed, 'type', true, true))).toEqual([
      'src',
      'z.rs',
      'b.py',
      'a.md',
      'notes',
    ])
  })

  it('sorts by name inside the same extension', () => {
    const files = [node('b.md', 'file'), node('a.md', 'file')]
    expect(names(sortNodes(files, 'type', false, false))).toEqual(['a.md', 'b.md'])
  })

  it('defaults to newest first only for the modified mode', () => {
    expect(defaultDesc('modified')).toBe(true)
    expect(defaultDesc('name')).toBe(false)
    expect(defaultDesc('type')).toBe(false)
  })
})
