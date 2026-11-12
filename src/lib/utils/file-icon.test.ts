import { describe, expect, it } from 'vitest'
import { iconName } from './file-icon'

describe('iconName', () => {
  it('maps the common file types', () => {
    expect(iconName('README.md', false)).toBe('file_type_markdown')
    expect(iconName('notes.MD', false)).toBe('file_type_markdown')
    expect(iconName('main.rs', false)).toBe('file_type_rust')
    expect(iconName('app.tsx', false)).toBe('file_type_reactts')
    expect(iconName('config.yml', false)).toBe('file_type_yaml')
    expect(iconName('photo.JPG', false)).toBe('file_type_image')
  })

  it('keeps the special file names', () => {
    expect(iconName('package.json', false)).toBe('file_type_npm')
    expect(iconName('.gitignore', false)).toBe('file_type_git')
  })

  it('falls back for folders and unknown files', () => {
    expect(iconName('src', true)).toBe('folder_type_src')
    expect(iconName('docs', true)).toBe('default_folder')
    expect(iconName('LICENSE', false)).toBe('default_file')
    expect(iconName('archive.zip', false)).toBe('default_file')
  })
})
