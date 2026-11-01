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

/// Panel surface the accent is drawn on, mirroring --surface-1 in app.css.
function surfaceForTheme() {
  const theme = document.documentElement.dataset.theme
  if (theme === 'vs') return '#f3f3f3'
  if (theme === 'hc-black') return '#0a0a0a'
  return '#252526'
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
