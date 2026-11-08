const FOCUSABLE =
  'a[href]:not([tabindex="-1"]), button:not([disabled]):not([tabindex="-1"]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

/// Index to focus for a Tab at `index` in a list of `count`, or null to let the
/// browser move on. `index < 0` means focus had escaped the dialog.
export function trapTarget(index: number, count: number, shift: boolean): number | null {
  if (count === 0) return null
  if (index < 0) return shift ? count - 1 : 0
  if (shift && index === 0) return count - 1
  if (!shift && index === count - 1) return 0
  return null
}

/// Keeps Tab inside a modal and returns focus to the opener when it unmounts.
/// The spec calls this `focusTrap`; the name sticks so the call sites match.
export function focusTrap(node: HTMLElement) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null
  const items = () => Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE))

  function onKeydown(e: KeyboardEvent) {
    // Monaco and other widgets that consume Tab keep their own behaviour.
    if (e.key !== 'Tab' || e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey) return
    const list = items()
    const target = trapTarget(list.indexOf(document.activeElement as HTMLElement), list.length, e.shiftKey)
    if (target === null) return
    e.preventDefault()
    list[target].focus()
  }

  node.addEventListener('keydown', onKeydown)
  // Move the focus in: the first control is the natural start for a dialog.
  items()[0]?.focus()

  return {
    destroy() {
      node.removeEventListener('keydown', onKeydown)
      previous?.focus()
    },
  }
}
