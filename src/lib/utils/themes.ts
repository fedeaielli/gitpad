import { accentTokens } from './color'

/// App theme id → Monaco theme id. Each app theme has its own Monaco theme so
/// the editor surface matches the surrounding chrome.
const MONACO: Record<string, string> = {
  'gitpad-dark': 'gitpad-dark',
  'vs-dark': 'gitpad-dark',
  vs: 'gitpad-light',
  'hc-black': 'gitpad-hc',
}

export function monacoTheme(theme: string) {
  return MONACO[theme] ?? 'gitpad-dark'
}

/// Stylesheet pairs swapped with the theme; both are Vite asset URLs.
const STYLES: Record<string, { markdown: string; code: string }> = {}

export function applyTheme(theme: string) {
  document.documentElement.dataset.theme = theme
  const styles = STYLES[theme]
  if (!styles) return
  for (const [id, href] of [
    ['gitpad-markdown-theme', styles.markdown],
    ['gitpad-code-theme', styles.code],
  ]) {
    let link = document.getElementById(id) as HTMLLinkElement | null
    if (!link) {
      link = document.createElement('link')
      link.id = id
      link.rel = 'stylesheet'
      document.head.appendChild(link)
    }
    link.href = href
  }
}

export function registerThemeStyles(map: Record<string, { markdown: string; code: string }>) {
  Object.assign(STYLES, map)
  // The lazy preview chunk registers after the first applyTheme call.
  applyTheme(document.documentElement.dataset.theme ?? 'gitpad-dark')
}

/// The sentinel value of the "Default" option: it means "let each surface pick".
export const DEFAULT_FONT = 'ui-monospace, monospace'

/// Repairs a family saved before fs_list_fonts dropped the fc-list path, when
/// the stored value looked like "/usr/share/fonts/x.ttf: Adwaita Sans".
export function cleanFamily(value: string): string {
  if (!value || value === DEFAULT_FONT) return DEFAULT_FONT
  const rest = value.includes(': ') ? value.slice(value.lastIndexOf(': ') + 2) : value
  const name = rest.split(':style=')[0].split(',')[0].trim()
  return name || DEFAULT_FONT
}

/// The chrome follows the editor font; the Default value keeps the system stack
/// that app.css sets on body.
export function applyUiFont(font: string) {
  document.body.style.fontFamily = font === DEFAULT_FONT ? '' : font
}

/// Panel surface the accent is drawn on, mirroring --surface-1 in app.css.
export function surfaceForTheme() {
  const theme = document.documentElement.dataset.theme
  if (theme === 'vs') return '#f2eee4'
  if (theme === 'hc-black') return '#0a0a0a'
  return '#23211e'
}

/// An empty accent drops the overrides and falls back to the theme's own.
export function applyAccent(accent: string) {
  const root = document.documentElement.style
  if (!accent) {
    root.removeProperty('--accent')
    root.removeProperty('--accent-fg')
    root.removeProperty('--on-accent')
    return
  }
  const light = document.documentElement.dataset.theme === 'vs'
  const tokens = accentTokens(accent, surfaceForTheme(), light)
  if (!tokens) return
  root.setProperty('--accent', accent)
  root.setProperty('--accent-fg', tokens.fg)
  root.setProperty('--on-accent', tokens.on)
}
