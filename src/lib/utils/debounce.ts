export function debounce(fn: () => void, ms: number | (() => number)) {
  let timer: ReturnType<typeof setTimeout> | undefined
  const delay = () => (typeof ms === 'number' ? ms : ms())
  return {
    run() {
      clearTimeout(timer)
      timer = setTimeout(fn, delay())
    },
    cancel() {
      clearTimeout(timer)
      timer = undefined
    },
  }
}
