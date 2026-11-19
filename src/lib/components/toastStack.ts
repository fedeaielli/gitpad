/// Collapsed cards peek by one spacing step; the expanded stack keeps the same
/// step as a gap. Both match `--sp-3`.
export const TOAST_PEEK = 12
export const TOAST_GAP = 12

export interface ToastPlacement {
  /// Distance above the resting bottom row, in px.
  offsetY: number
  scale: number
}

/// Place one toast in the stack. `index` is its DOM position, oldest to newest;
/// `ids` is the same order, so the newest id is last. Older cards tuck behind
/// the newest one when collapsed and line up above it when expanded.
export function stackPlacement(
  index: number,
  ids: number[],
  heights: Record<number, number>,
  expanded: boolean,
): ToastPlacement {
  const newer = ids.slice(index + 1)
  if (expanded) {
    return {
      offsetY: newer.reduce((y, id) => y + (heights[id] ?? 0) + TOAST_GAP, 0),
      scale: 1,
    }
  }
  const newest = ids[ids.length - 1]
  const previous = ids[ids.length - 2]
  // A card that just arrived has no measured height yet; borrowing the previous
  // front's height keeps every older card still until the measurement lands.
  const frontHeight = heights[newest] ?? (previous === undefined ? 0 : heights[previous]) ?? 0
  const ownHeight = ids[index] === undefined ? 0 : (heights[ids[index]] ?? 0)
  const depth = newer.length
  const scale = Math.max(0, 1 - depth * 0.1)
  // Lift each older card so its top sits one peek above the card in front of it,
  // whatever the two heights are.
  return { offsetY: frontHeight + depth * TOAST_PEEK - scale * ownHeight, scale }
}
