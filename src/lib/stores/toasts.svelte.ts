export type ToastKind = 'error' | 'info'

export interface Toast {
  id: number
  message: string
  kind: ToastKind
}

let seq = 0

export const toasts = $state({ items: [] as Toast[] })

/// Transient, non-blocking feedback for failures that have no inline spot.
export function pushToast(message: string, kind: ToastKind = 'error') {
  const id = ++seq
  toasts.items = [...toasts.items, { id, message, kind }].slice(-3)
  setTimeout(() => dismissToast(id), 5000)
}

export function dismissToast(id: number) {
  toasts.items = toasts.items.filter((t) => t.id !== id)
}
