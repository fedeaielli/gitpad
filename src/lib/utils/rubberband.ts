/// Resist past a bound instead of stopping dead: the further you pull, the less
/// it follows. Apple's rubber-banding constant (WWDC 2018, Designing Fluid Interfaces).
export function rubberband(overshoot: number, dimension: number, constant = 0.55): number {
  return (overshoot * dimension * constant) / (dimension + constant * Math.abs(overshoot))
}

/// Soft-clamp a drag position to [min, max], letting it overshoot a little.
export function resist(value: number, min: number, max: number): number {
  if (value < min) return min - rubberband(min - value, min)
  if (value > max) return max + rubberband(value - max, max)
  return value
}
