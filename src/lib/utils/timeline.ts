export const MIN_TIMELINE = 120
export const MAX_TIMELINE = 600
export const MIN_EDITOR = 160

/// Keep the history inside the editor column and leave room for the editor.
export function clampTimelineHeight(columnHeight: number, wanted: number): number {
  const max = Math.max(MIN_TIMELINE, columnHeight - MIN_EDITOR)
  return Math.min(MAX_TIMELINE, Math.max(MIN_TIMELINE, wanted), max)
}
