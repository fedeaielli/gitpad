const TEXT_NS = 'urn:oasis:names:tc:opendocument:xmlns:text:1.0'
const STYLE_NS = 'urn:oasis:names:tc:opendocument:xmlns:style:1.0'
const FO_NS = 'urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0'
const TABLE_NS = 'urn:oasis:names:tc:opendocument:xmlns:table:1.0'
const OFFICE_NS = 'urn:oasis:names:tc:opendocument:xmlns:office:1.0'
const XLINK_NS = 'http://www.w3.org/1999/xlink'

interface TextStyle {
  bold?: boolean
  italic?: boolean
  underline?: boolean
}

/// Namespace-aware lookup without getElementsByTagNameNS: happy-dom returns
/// nothing from it, and the recursive filter works in both engines.
function elements(root: Element, ns: string, localName: string): Element[] {
  return Array.from(root.getElementsByTagName('*')).filter(
    (el) => el.namespaceURI === ns && el.localName === localName,
  )
}

/// getAttributeNS is also unreliable under happy-dom (attributes lose their
/// namespace), so fall back to the prefixed name the XML actually carries.
function attr(el: Element, ns: string, localName: string): string | null {
  const direct = el.getAttributeNS(ns, localName)
  if (direct !== null) return direct
  for (const attribute of Array.from(el.attributes)) {
    if (attribute.localName === localName || attribute.name.endsWith(`:${localName}`)) {
      return attribute.value
    }
  }
  return null
}

function parseStyles(xml?: string): Map<string, TextStyle> {
  const styles = new Map<string, TextStyle>()
  if (!xml) return styles
  const doc = new DOMParser().parseFromString(xml, 'application/xml')
  const root = doc.documentElement
  if (!root) return styles
  for (const style of elements(root, STYLE_NS, 'style')) {
    const name = attr(style, STYLE_NS, 'name')
    const props = elements(style, STYLE_NS, 'text-properties')[0]
    if (!name || !props) continue
    styles.set(name, {
      bold: attr(props, FO_NS, 'font-weight') === 'bold',
      italic: attr(props, FO_NS, 'font-style') === 'italic',
      underline: attr(props, STYLE_NS, 'text-underline-style') === 'solid',
    })
  }
  return styles
}

function parseOrderedLists(xml?: string): Set<string> {
  const ordered = new Set<string>()
  if (!xml) return ordered
  const doc = new DOMParser().parseFromString(xml, 'application/xml')
  const root = doc.documentElement
  if (!root) return ordered
  for (const style of elements(root, TEXT_NS, 'list-style')) {
    const name = attr(style, STYLE_NS, 'name')
    if (name && elements(style, TEXT_NS, 'list-level-style-number').length) {
      ordered.add(name)
    }
  }
  return ordered
}

function escape(text: string): string {
  return text.replace(
    /[&<>"]/g,
    (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[char] ?? char,
  )
}

function inline(node: Node, styles: Map<string, TextStyle>): string {
  if (node.nodeType === Node.TEXT_NODE) return escape(node.textContent ?? '')
  if (node.nodeType !== Node.ELEMENT_NODE) return ''
  const el = node as Element
  if (el.namespaceURI === TEXT_NS) {
    switch (el.localName) {
      case 's':
        return ' '.repeat(Number(attr(el, TEXT_NS, 'c') ?? '1'))
      case 'tab':
        return '\t'
      case 'line-break':
        return '<br>'
      case 'span': {
        let out = children(el, styles)
        const style = styles.get(attr(el, TEXT_NS, 'style-name') ?? '')
        if (!style) return out
        if (style.bold) out = `<strong>${out}</strong>`
        if (style.italic) out = `<em>${out}</em>`
        if (style.underline) out = `<u>${out}</u>`
        return out
      }
      case 'a': {
        const href = attr(el, XLINK_NS, 'href') ?? ''
        const out = children(el, styles)
        return /^https?:/i.test(href) ? `<a href="${escape(href)}">${out}</a>` : out
      }
      case 'bookmark':
        return ''
    }
  }
  return children(el, styles)
}

function children(el: Element, styles: Map<string, TextStyle>): string {
  return Array.from(el.childNodes)
    .map((node) => inline(node, styles))
    .join('')
}

function list(el: Element, styles: Map<string, TextStyle>, ordered: Set<string>): string {
  const tag = ordered.has(attr(el, TEXT_NS, 'style-name') ?? '') ? 'ol' : 'ul'
  const items = Array.from(el.children)
    .filter((child) => child.namespaceURI === TEXT_NS && child.localName === 'list-item')
    .map((item) => `<li>${blocks(item, styles, ordered)}</li>`)
    .join('')
  return `<${tag}>${items}</${tag}>`
}

function table(el: Element, styles: Map<string, TextStyle>, ordered: Set<string>): string {
  const rows = Array.from(el.children)
    .filter((child) => child.namespaceURI === TABLE_NS && child.localName === 'table-row')
    .map((row) => {
      const cells = Array.from(row.children)
        .filter(
          (cell) =>
            cell.namespaceURI === TABLE_NS &&
            (cell.localName === 'table-cell' || cell.localName === 'covered-table-cell'),
        )
        .map((cell) => `<td>${blocks(cell, styles, ordered)}</td>`)
        .join('')
      return `<tr>${cells}</tr>`
    })
    .join('')
  return `<table>${rows}</table>`
}

function block(el: Element, styles: Map<string, TextStyle>, ordered: Set<string>): string {
  if (el.namespaceURI === TEXT_NS) {
    if (el.localName === 'h') {
      const level = Math.min(6, Math.max(1, Number(attr(el, TEXT_NS, 'outline-level') ?? '1')))
      return `<h${level}>${children(el, styles)}</h${level}>`
    }
    if (el.localName === 'p') return `<p>${children(el, styles)}</p>`
    if (el.localName === 'list') return list(el, styles, ordered)
  }
  if (el.namespaceURI === TABLE_NS && el.localName === 'table') return table(el, styles, ordered)
  // Sections, indexes and frames: recurse so their text is not lost.
  return blocks(el, styles, ordered)
}

function blocks(el: Element, styles: Map<string, TextStyle>, ordered: Set<string>): string {
  return Array.from(el.children)
    .map((child) => block(child, styles, ordered))
    .join('')
}

/// Basic ODT → HTML: headings, paragraphs, inline styling, lists and tables.
/// Images, notes and headers are skipped on purpose.
/// shortcut: basic ODT fidelity only; upgrade to a full ODF renderer if needed.
export function odfToHtml(contentXml: string, stylesXml?: string): string {
  const styles = parseStyles(stylesXml)
  const ordered = parseOrderedLists(stylesXml)
  const doc = new DOMParser().parseFromString(contentXml, 'application/xml')
  const root = doc.documentElement
  if (!root) return ''
  const body = elements(root, OFFICE_NS, 'text')[0]
  return body ? blocks(body, styles, ordered) : ''
}
