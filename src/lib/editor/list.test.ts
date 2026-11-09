import { describe, expect, it } from 'vitest'
import { listItem } from './list'

const prefix = (line: string) => listItem(line)?.prefix ?? null

describe('listItem', () => {
  it('counts numbers up and keeps the delimiter', () => {
    expect(prefix('1. a')).toBe('2. ')
    expect(prefix('9. a')).toBe('10. ')
    expect(prefix('2) a')).toBe('3) ')
  })

  it('keeps the indentation of a nested item', () => {
    expect(prefix('  1. a')).toBe('  2. ')
    expect(prefix('\t- a')).toBe('\t- ')
  })

  it('repeats the bullet and resets a task to unchecked', () => {
    expect(prefix('- a')).toBe('- ')
    expect(prefix('* a')).toBe('* ')
    expect(prefix('+ a')).toBe('+ ')
    expect(prefix('- [x] a')).toBe('- [ ] ')
  })

  it('marks an item with no content as empty', () => {
    expect(listItem('1. ')?.empty).toBe(true)
    expect(listItem('- [ ]  ')?.empty).toBe(true)
    expect(listItem('1. a')?.empty).toBe(false)
  })

  it('ignores lines that are not list items', () => {
    expect(prefix('a')).toBeNull()
    expect(prefix('1.foo')).toBeNull()
    expect(prefix('# 1. a')).toBeNull()
    expect(prefix('')).toBeNull()
  })
})
