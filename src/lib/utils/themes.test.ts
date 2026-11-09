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

  // A neutral source (#ffffff, #797979, --text at low alpha) composites to a
  // chroma-free grey, which is what "the scrollbar looks cold" means in
  // practice. The warm palette has to survive the alpha blend.
  it('keeps the scrollbar sliders warm', () => {
    const sliders = [...PANE.matchAll(/'scrollbarSlider\.\w+':\s*'(#[0-9a-f]{6})([0-9a-f]{2})'/g)]
    expect(sliders.length).toBe(9)
    const chroma = (hex: string) => {
      const v = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16))
      return Math.max(...v) - Math.min(...v)
    }
    for (const [, hex] of sliders) expect(chroma(hex)).toBeGreaterThan(15)
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
