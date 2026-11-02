import { describe, expect, it } from 'vitest'
import { DEFAULT_FONT, cleanFamily } from './themes'

describe('cleanFamily', () => {
  it('drops the fc-list path and style from a stored value', () => {
    expect(cleanFamily('/usr/share/fonts/Adwaita/AdwaitaSans-Italic.ttf: Adwaita Sans')).toBe(
      'Adwaita Sans',
    )
    expect(cleanFamily('/usr/share/fonts/x.ttf: Noto Sans,Noto Sans Light:style=Bold')).toBe(
      'Noto Sans',
    )
  })

  it('leaves a plain family and the default alone', () => {
    expect(cleanFamily('Inter')).toBe('Inter')
    expect(cleanFamily(DEFAULT_FONT)).toBe(DEFAULT_FONT)
    expect(cleanFamily('')).toBe(DEFAULT_FONT)
  })
})
