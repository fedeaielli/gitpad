/// Next highlighted index for a key inside a closed or open list. Null means
/// the key is not ours and the caller should let it through.
export function stepIndex(index: number, count: number, key: string): number | null {
  if (count === 0) return null
  if (key === 'ArrowDown') return (index + 1) % count
  if (key === 'ArrowUp') return (index - 1 + count) % count
  if (key === 'Home') return 0
  if (key === 'End') return count - 1
  return null
}
