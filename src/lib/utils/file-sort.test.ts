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
    expect(names(sortNodes(files, 'name', false))).toEqual(['a.md', 'b.md', 'c.md'])
    expect(names(sortNodes(files, 'name', true))).toEqual(['c.md', 'b.md', 'a.md'])
  })

  it('sorts by modification in both directions', () => {
    expect(names(sortNodes(files, 'modified', true))).toEqual(['a.md', 'c.md', 'b.md'])
    expect(names(sortNodes(files, 'modified', false))).toEqual(['b.md', 'c.md', 'a.md'])
  })

  it('keeps folders on top whichever the direction', () => {
    const mixed = [node('a.md', 'file'), node('z-dir', 'dir'), node('b-directory', 'dir')]
    expect(names(sortNodes(mixed, 'folders', false))).toEqual(['b-directory', 'z-dir', 'a.md'])
    expect(names(sortNodes(mixed, 'folders', true))).toEqual(['z-dir', 'b-directory', 'a.md'])
  })

  it('does not mutate the input', () => {
    const input = [node('b.md', 'file'), node('a.md', 'file')]
    sortNodes(input, 'name', false)
    expect(names(input)).toEqual(['b.md', 'a.md'])
  })

  it('defaults to newest first only for the modified mode', () => {
    expect(defaultDesc('modified')).toBe(true)
    expect(defaultDesc('name')).toBe(false)
    expect(defaultDesc('folders')).toBe(false)
  })
})
