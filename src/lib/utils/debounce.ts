export function debounce(fn: () => void, ms: number) {
  let timer: ReturnType<typeof setTimeout> | undefined
  return {
    run() {
      clearTimeout(timer)
      timer = setTimeout(fn, ms)
    },
    cancel() {
      clearTimeout(timer)
      timer = undefined
    },
  }
}
