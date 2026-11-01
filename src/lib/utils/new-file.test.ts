import { describe, expect, it } from 'vitest'
import { newFileError } from './new-file'

const tree = ['notes.md', 'todo.txt', 'archive', 'archive/old.md']

describe('newFileError', () => {
  it('ignores an empty name', () => {
    expect(newFileError('   ', tree)).toBeNull()
  })

  it('rejects traversal, absolute paths and .git', () => {
    expect(newFileError('../x.md', tree)).toBe('invalid file name')
    expect(newFileError('/etc/passwd', tree)).toBe('invalid file name')
    expect(newFileError('.git/config', tree)).toBe('invalid file name')
    expect(newFileError('a/../b.md', tree)).toBe('invalid file name')
  })

  it('asks for a file name when the path ends with a slash', () => {
    expect(newFileError('archive/', tree)).toBe('a file name is required')
  })

  it('rejects an existing file', () => {
    expect(newFileError('notes.md', tree)).toBe('notes.md already exists')
  })

  it('rejects an existing folder', () => {
    expect(newFileError('archive', tree)).toBe('archive already exists')
  })

  it('accepts new names, including in an existing folder', () => {
    expect(newFileError('notes.md', [])).toBeNull()
    expect(newFileError('a/b.md', tree)).toBeNull()
    expect(newFileError('archive/new.md', tree)).toBeNull()
  })
})
