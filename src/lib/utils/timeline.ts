export const MIN_TIMELINE = 120
export const MAX_TIMELINE = 600
export const MIN_EDITOR = 160

/// Tallest the history may get without swallowing the editor.
export function timelineMax(columnHeight: number): number {
  return Math.max(MIN_TIMELINE, Math.min(MAX_TIMELINE, columnHeight - MIN_EDITOR))
}

/// Keep the history inside the editor column and leave room for the editor.
export function clampTimelineHeight(columnHeight: number, wanted: number): number {
  return Math.min(timelineMax(columnHeight), Math.max(MIN_TIMELINE, wanted))
}
