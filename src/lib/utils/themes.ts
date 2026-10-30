/// App theme id → Monaco theme id. Custom dark theme covers both dark presets.
const MONACO: Record<string, string> = {
  'gitpad-dark': 'gitpad-dark',
  'vs-dark': 'gitpad-dark',
  vs: 'vs',
  'hc-black': 'hc-black',
}

export function monacoTheme(theme: string) {
  return MONACO[theme] ?? 'gitpad-dark'
}

export function isLightTheme(theme: string) {
  return theme === 'vs'
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
}
