import { describe, expect, it } from 'vitest'
import { accentTokens, contrast, luminance, parseHex } from './color'

const PRESETS = [
  '#0078d4',
  '#8b5cf6',
  '#10b981',
  '#f38518',
  '#e5484d',
  '#ec4899',
  '#06b6d4',
  '#d7ba7d',
]

const DARK_PANEL = '#252526'
const LIGHT_PANEL = '#f3f3f3'

describe('parseHex', () => {
  it('expands #rgb', () => {
    expect(parseHex('#fff')).toEqual([255, 255, 255])
  })

  it('reads #rrggbb', () => {
    expect(parseHex('#0078d4')).toEqual([0, 120, 212])
  })

  it('rejects words', () => {
    expect(parseHex('invalid')).toBeNull()
  })

  it('rejects non-hex digits', () => {
    expect(parseHex('#gggggg')).toBeNull()
  })
})

describe('luminance', () => {
  it('is 0 for black and 1 for white', () => {
    expect(luminance(0, 0, 0)).toBe(0)
    expect(luminance(255, 255, 255)).toBeCloseTo(1, 5)
  })
})

describe('contrast', () => {
  it('is 21 for black on white', () => {
    expect(contrast('#ffffff', '#000000')).toBeCloseTo(21, 1)
  })

  it('is 1 for a colour against itself', () => {
    expect(contrast('#ffffff', '#ffffff')).toBeCloseTo(1, 5)
  })

  it('is null when a colour is invalid', () => {
    expect(contrast('#ffffff', 'nope')).toBeNull()
  })
})

describe('accentTokens', () => {
  it('keeps white text on the default blue', () => {
    const tokens = accentTokens('#0078d4', DARK_PANEL, false)
    expect(tokens?.on).toBe('#ffffff')
    expect(contrast(tokens!.on, '#0078d4')!).toBeGreaterThanOrEqual(4.5)
  })

  it('switches to black text on a pale accent', () => {
    const tokens = accentTokens('#d7ba7d', DARK_PANEL, false)
    expect(tokens?.on).toBe('#000000')
  })

  it('lightens the foreground for a dark panel', () => {
    const tokens = accentTokens('#0078d4', DARK_PANEL, false)
    expect(tokens).not.toBeNull()
    expect(contrast(tokens!.fg, DARK_PANEL)!).toBeGreaterThanOrEqual(4.5)
  })

  it('darkens the foreground for a light panel', () => {
    const tokens = accentTokens('#0078d4', LIGHT_PANEL, true)
    expect(tokens).not.toBeNull()
    expect(contrast(tokens!.fg, LIGHT_PANEL)!).toBeGreaterThanOrEqual(4.5)
  })

  it.each(PRESETS)('derives readable tokens for %s', (accent) => {
    const tokens = accentTokens(accent, DARK_PANEL, false)
    expect(tokens, accent).not.toBeNull()
    expect(contrast(tokens!.on, accent), `${accent} on-accent`).toBeGreaterThanOrEqual(4.5)
    expect(contrast(tokens!.fg, DARK_PANEL), `${accent} accent-fg`).toBeGreaterThanOrEqual(4.5)
  })

  it('returns null for an invalid accent', () => {
    expect(accentTokens('invalid', DARK_PANEL, false)).toBeNull()
  })
})
