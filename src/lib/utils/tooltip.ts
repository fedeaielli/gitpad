/// Native `title` hints are drawn by GTK and ignore the theme tokens, so a
/// hover in dark mode flashes an unstyled box. This is one shared node instead.
const SHOW_DELAY = 400
const GAP = 6
const EDGE = 8

let tipEl: HTMLDivElement | null = null
let showTimer: ReturnType<typeof setTimeout> | undefined
let anchor: HTMLElement | null = null

function hide() {
  clearTimeout(showTimer)
  if (tipEl) tipEl.hidden = true
}

function place(node: HTMLElement) {
  if (!tipEl) return
  const box = node.getBoundingClientRect()
  // Measure at 0,0 first: width is needed before the final left can be chosen.
  tipEl.style.left = '0px'
  tipEl.style.top = '0px'
  tipEl.hidden = false
  const size = tipEl.getBoundingClientRect()
  const left = Math.min(
    Math.max(EDGE, box.left + box.width / 2 - size.width / 2),
    window.innerWidth - size.width - EDGE,
  )
  const above = box.top - size.height - GAP
  tipEl.style.left = `${left}px`
  tipEl.style.top = `${above >= EDGE ? above : box.bottom + GAP}px`
}

function ensure(): HTMLDivElement {
  if (tipEl) return tipEl
  const el = document.createElement('div')
  el.className = 'gitpad-tip'
  el.setAttribute('role', 'tooltip')
  el.hidden = true
  document.body.append(el)
  tipEl = el
  // Registered once: a scroll or Escape must never leave a hint floating.
  window.addEventListener('scroll', hide, true)
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hide()
  })
  window.addEventListener('blur', hide)
  return el
}

/// Usage: `use:tip={'Explorer'}`. An empty or undefined label shows nothing.
export function tip(node: HTMLElement, text: string | undefined) {
  let label = text

  function show() {
    if (!label) return
    const el = ensure()
    el.textContent = label
    anchor = node
    place(node)
  }

  function schedule() {
    if (!label) return
    clearTimeout(showTimer)
    showTimer = setTimeout(show, SHOW_DELAY)
  }

  node.addEventListener('pointerenter', schedule)
  node.addEventListener('pointerleave', hide)
  node.addEventListener('focusin', schedule)
  node.addEventListener('focusout', hide)
  node.addEventListener('pointerdown', hide)

  return {
    update(next: string | undefined) {
      label = next
      if (!label) hide()
      else if (anchor === node && tipEl && !tipEl.hidden) show()
    },
    destroy() {
      hide()
      node.removeEventListener('pointerenter', schedule)
      node.removeEventListener('pointerleave', hide)
      node.removeEventListener('focusin', schedule)
      node.removeEventListener('focusout', hide)
      node.removeEventListener('pointerdown', hide)
    },
  }
}
