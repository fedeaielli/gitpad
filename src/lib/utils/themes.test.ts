import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { DEFAULT_FONT, cleanFamily } from './themes'

const PANE = readFileSync(
  fileURLToPath(new URL('../components/EditorPane.svelte', import.meta.url)),
  'utf8',
)

describe('monaco theme colors', () => {
  // Monaco builds every theme color with Color.fromHex(), which returns
  // Color.red for anything that is not #hex. A rgba() value therefore paints
  // solid red instead of failing loudly.
  it('are all #hex, since rgba() silently becomes red', () => {
    const values = [...PANE.matchAll(/^\s*'[a-zA-Z0-9.]+':\s*'([^']+)',/gm)].map((m) => m[1])
    expect(values.length).toBeGreaterThan(50)
    expect(values.filter((v) => !/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(v))).toEqual([])
  })
})

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
