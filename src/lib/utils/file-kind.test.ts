import { describe, expect, it } from 'vitest'
import { imageMime, viewerFor } from './file-kind'

describe('viewerFor', () => {
  it('maps images, pdf and office documents', () => {
    expect(viewerFor('photo.PNG')).toBe('image')
    expect(viewerFor('notes/sub/photo.jpeg')).toBe('image')
    expect(viewerFor('scan.pdf')).toBe('pdf')
    expect(viewerFor('report.docx')).toBe('docx')
    expect(viewerFor('report.ODT')).toBe('odt')
  })

  it('leaves text and unknown files to the editor', () => {
    expect(viewerFor('notes.md')).toBeNull()
    expect(viewerFor('config.yaml')).toBeNull()
    expect(viewerFor('README')).toBeNull()
    expect(viewerFor('archive.zip')).toBeNull()
  })
})

describe('imageMime', () => {
  it('uses the real mime type', () => {
    expect(imageMime('logo.svg')).toBe('image/svg+xml')
    expect(imageMime('photo.JPG')).toBe('image/jpeg')
    expect(imageMime('icon.ico')).toBe('image/x-icon')
    expect(imageMime('shot.webp')).toBe('image/webp')
  })
})
