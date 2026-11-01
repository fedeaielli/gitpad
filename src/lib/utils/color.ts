/// Colour maths for the custom accent: expand, measure and derive the two
/// companion tokens. WCAG 2.1 relative luminance, no dependencies.

export type Rgb = [number, number, number]

const HEX = /^#?(?:[0-9a-f]{3}|[0-9a-f]{6})$/i

/// Expands `#rgb` to `#rrggbb`; null for anything else.
export function parseHex(hex: string): Rgb | null {
  const raw = hex.trim()
  if (!HEX.test(raw)) return null
  const digits = raw.replace('#', '')
  const full =
    digits.length === 3
      ? digits
          .split('')
          .map((d) => d + d)
          .join('')
      : digits
  return [
    parseInt(full.slice(0, 2), 16),
    parseInt(full.slice(2, 4), 16),
    parseInt(full.slice(4, 6), 16),
  ]
}

export function toHex(rgb: Rgb): string {
  return '#' + rgb.map((c) => Math.round(c).toString(16).padStart(2, '0')).join('')
}

/// WCAG 2.1 relative luminance of an sRGB triple (0–255 per channel).
export function luminance(r: number, g: number, b: number): number {
  const channel = (value: number) => {
    const s = value / 255
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
  }
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

function ratio(a: Rgb, b: Rgb): number {
  const la = luminance(a[0], a[1], a[2])
  const lb = luminance(b[0], b[1], b[2])
  const [hi, lo] = la >= lb ? [la, lb] : [lb, la]
  return (hi + 0.05) / (lo + 0.05)
}

/// WCAG contrast ratio (1–21); null when either colour is invalid.
export function contrast(hex1: string, hex2: string): number | null {
  const a = parseHex(hex1)
  const b = parseHex(hex2)
  if (!a || !b) return null
  return ratio(a, b)
}

const WHITE: Rgb = [255, 255, 255]
const BLACK: Rgb = [0, 0, 0]
const MIN_TEXT = 4.5

/// White text on the accent while it still clears 4.5:1, otherwise black.
/// Picking the higher ratio instead would flip the default blue to black text,
/// because #000000 scores 4.64 against #0078d4 and #ffffff 4.53.
function onAccent(accent: Rgb): string {
  return ratio(WHITE, accent) >= MIN_TEXT ? '#ffffff' : '#000000'
}

/// Nudges the accent towards white (dark themes) or black (light themes) until
/// it reads as text on the panel surface, 4% per step.
function foreground(accent: Rgb, surface: Rgb, light: boolean): string {
  const target = light ? BLACK : WHITE
  let current = accent
  for (let step = 0; step <= 50; step += 1) {
    if (ratio(current, surface) >= MIN_TEXT) return toHex(current)
    current = current.map((c, i) => Math.round(c + (target[i] - c) * 0.04)) as Rgb
  }
  return light ? '#000000' : '#ffffff'
}

/// `--on-accent` and `--accent-fg` for a user-chosen accent; null when the
/// accent or the surface is not a valid hex colour.
export function accentTokens(
  accent: string,
  surface: string,
  light: boolean,
): { on: string; fg: string } | null {
  const base = parseHex(accent)
  const panel = parseHex(surface)
  if (!base || !panel) return null
  return { on: onAccent(base), fg: foreground(base, panel, light) }
}
