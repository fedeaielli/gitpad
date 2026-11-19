/// Moves a node out of the component tree and into `body`. A `position: fixed`
/// box inside a scrolling ancestor is laid out against that ancestor in
/// WebKitGTK, so it is drawn where it looks right and hit-tested somewhere
/// else; `body` has no scrolling ancestor above it.
export function portal(node: HTMLElement) {
  document.body.appendChild(node)
  return {
    destroy() {
      node.remove()
    },
  }
}
