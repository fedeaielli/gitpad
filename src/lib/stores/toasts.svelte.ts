import { invoke } from '@tauri-apps/api/core'

export type ToastKind = 'error' | 'info' | 'success'

export interface Toast {
  id: number
  message: string
  kind: ToastKind
}

let seq = 0

export const toasts = $state({ items: [] as Toast[] })

export type ToastCategory = 'editor' | 'git' | 'app'

/// Mirrors one line to the Rust logger without showing anything.
export function logLine(
  level: 'info' | 'warn' | 'error',
  message: string,
  category: ToastCategory = 'app',
) {
  void invoke('app_log', { level, category, message }).catch(() => {})
}

/// Transient, non-blocking feedback for failures that have no inline spot.
export function pushToast(
  message: string,
  kind: ToastKind = 'error',
  category: ToastCategory = 'editor',
) {
  const id = ++seq
  toasts.items = [...toasts.items, { id, message, kind }].slice(-3)
  setTimeout(() => dismissToast(id), 5000)
  // Mirrored to the file log, so a report shows what the user was told.
  logLine(kind === 'error' ? 'error' : 'info', message, category)
}

export function dismissToast(id: number) {
  toasts.items = toasts.items.filter((t) => t.id !== id)
}
