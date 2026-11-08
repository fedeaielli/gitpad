import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const CSS = readFileSync(fileURLToPath(new URL('../../app.css', import.meta.url)), 'utf8').replace(
  /\/\*[\s\S]*?\*\//g,
  '',
)

type Vars = Record<string, string>

function declarations(selector: string): Vars {
  const start = CSS.indexOf(`${selector} {`)
  if (start < 0) throw new Error(`missing rule ${selector}`)
  const body = CSS.slice(CSS.indexOf('{', start) + 1, CSS.indexOf('}', start))
  const out: Vars = {}
  for (const line of body.split(';')) {
    const at = line.indexOf(':')
    if (at < 0) continue
    const name = line.slice(0, at).trim()
    if (name.startsWith('--')) out[name] = line.slice(at + 1).trim()
  }
  return out
}

function resolve(vars: Vars, name: string): string {
  const raw = vars[name]
  if (raw === undefined) throw new Error(`undefined token ${name}`)
  return raw.replace(/var\((--[a-z0-9-]+)\)/g, (_, ref: string) => resolve(vars, ref))
}

function rgb(value: string): [number, number, number] | null {
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value.trim())
  if (!hex) return null
  const digits = hex[1].length === 3 ? [...hex[1]].map((c) => c + c).join('') : hex[1]
  return [0, 2, 4].map((i) => parseInt(digits.slice(i, i + 2), 16)) as [number, number, number]
}

function luminance([r, g, b]: [number, number, number]): number {
  const [rl, gl, bl] = [r, g, b]
    .map((v) => v / 255)
    .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
  return 0.2126 * rl + 0.7152 * gl + 0.0722 * bl
}

/// WCAG 2.1 relative-contrast ratio between two tokens of one theme.
function contrast(vars: Vars, a: string, b: string): number {
  const [ca, cb] = [rgb(resolve(vars, a)), rgb(resolve(vars, b))]
  if (!ca || !cb) throw new Error(`not a hex colour: ${a} / ${b}`)
  const [la, lb] = [luminance(ca), luminance(cb)]
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05)
}

const ROOT = declarations(':root')
const THEMES = [
  { name: 'dark', vars: ROOT },
  { name: 'vs', vars: { ...ROOT, ...declarations(":root[data-theme='vs']") } },
  { name: 'hc-black', vars: { ...ROOT, ...declarations(":root[data-theme='hc-black']") } },
]

const COLOUR_TOKENS = [
  '--surface-0',
  '--surface-1',
  '--surface-2',
  '--surface-3',
  '--text',
  '--muted',
  '--disabled',
  '--border',
  '--border-strong',
  '--accent',
  '--on-accent',
  '--accent-fg',
  '--selection',
  '--ok',
  '--warn',
  '--err',
  '--scrim',
]

describe.each(THEMES)('theme $name', ({ vars }) => {
  it('primary text reaches 4.5:1 on every surface', () => {
    for (const surface of ['--surface-0', '--surface-1', '--surface-2', '--surface-3']) {
      expect(contrast(vars, '--text', surface)).toBeGreaterThanOrEqual(4.5)
    }
  })

  it('secondary text reaches 4.5:1 on the chrome surfaces', () => {
    for (const surface of ['--surface-1', '--surface-2']) {
      expect(contrast(vars, '--muted', surface)).toBeGreaterThanOrEqual(4.5)
    }
  })

  it('accent text reaches 4.5:1 on surface-1', () => {
    expect(contrast(vars, '--accent-fg', '--surface-1')).toBeGreaterThanOrEqual(4.5)
  })

  it('text on accent fills reaches 4.5:1', () => {
    expect(contrast(vars, '--on-accent', '--accent')).toBeGreaterThanOrEqual(4.5)
  })

  it('status colours reach 4.5:1 on surface-2', () => {
    for (const status of ['--ok', '--warn', '--err']) {
      expect(contrast(vars, status, '--surface-2')).toBeGreaterThanOrEqual(4.5)
    }
  })

  it('text on the selection colour reaches 4.5:1', () => {
    expect(contrast(vars, '--text', '--selection')).toBeGreaterThanOrEqual(4.5)
  })

  it('strong borders reach 3:1 on every surface', () => {
    for (const surface of ['--surface-0', '--surface-1', '--surface-2', '--surface-3']) {
      expect(contrast(vars, '--border-strong', surface)).toBeGreaterThanOrEqual(3)
    }
  })
})

describe('token set', () => {
  it.each(THEMES)('$name defines every colour token', ({ vars }) => {
    for (const token of COLOUR_TOKENS) expect(vars[token], token).toBeDefined()
  })

  it('no longer defines the legacy aliases', () => {
    for (const alias of ['--bg', '--bg-panel', '--bg-raised']) {
      expect(ROOT[alias], alias).toBeUndefined()
    }
  })
})
