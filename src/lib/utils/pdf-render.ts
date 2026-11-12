/// CSS width a page is drawn at: the pane, capped to the 1000 px page width.
function displayWidth(containerWidth: number): number {
  return Math.min(1000, Math.max(320, containerWidth - 32))
}

/// PDF viewport scale: enough pixels for the screen, but at most 2 device
/// pixels per CSS pixel. The bitmap memory grows with the square of the scale.
export function renderScale(containerWidth: number, pageWidth: number, dpr: number): number {
  return (displayWidth(containerWidth) / pageWidth) * Math.min(dpr || 1, 2)
}
