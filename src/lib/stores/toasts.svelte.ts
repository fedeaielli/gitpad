import { invoke } from '@tauri-apps/api/core'

export type ToastKind = 'error' | 'warning' | 'info' | 'success'

export interface Toast {
  id: number
  message: string
  kind: ToastKind
  /// Opens when the action is clicked.
  url?: string
  /// Label for that action; falls back to "Open link".
  action?: string
}

let seq = 0

export const toasts = $state({ items: [] as Toast[] })

export type ToastCategory = 'editor' | 'git' | 'app'

/// How long a toast lives, in ms. 0 means it waits for the user: an error needs
/// reading and a link needs clicking, so neither may vanish on a timer.
const LIFETIME: Record<ToastKind, number> = {
  error: 0,
  warning: 6000,
  success: 3500,
  info: 3500,
}

export function toastLifetime(kind: ToastKind, hasAction: boolean): number {
  return hasAction ? 0 : LIFETIME[kind]
}

interface Countdown {
  handle: ReturnType<typeof setTimeout>
  left: number
  started: number
}

const countdowns = new Map<number, Countdown>()

function run(id: number, ms: number) {
  countdowns.set(id, {
    handle: setTimeout(() => dismissToast(id), ms),
    left: ms,
    started: Date.now(),
  })
}

/// Stop the clock while the pointer or the focus is on the toast, so a message
/// being read does not disappear mid-sentence. resumeToast() picks it back up.
export function pauseToast(id: number) {
  const countdown = countdowns.get(id)
  if (!countdown) return
  clearTimeout(countdown.handle)
  countdown.left = Math.max(0, countdown.left - (Date.now() - countdown.started))
}

export function resumeToast(id: number) {
  const countdown = countdowns.get(id)
  if (countdown) run(id, countdown.left)
}

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
  url?: string,
  action?: string,
) {
  const id = ++seq
  const toast: Toast = { id, message, kind, url, action }
  const next = [...toasts.items, toast]
  for (const dropped of next.slice(0, Math.max(0, next.length - 3))) clearCountdown(dropped.id)
  toasts.items = next.slice(-3)
  const ttl = toastLifetime(kind, Boolean(url))
  if (ttl > 0) run(id, ttl)
  // Mirrored to the file log, so a report shows what the user was told.
  logLine(kind === 'error' ? 'error' : kind === 'warning' ? 'warn' : 'info', message, category)
}

function clearCountdown(id: number) {
  const countdown = countdowns.get(id)
  if (!countdown) return
  clearTimeout(countdown.handle)
  countdowns.delete(id)
}

export function dismissToast(id: number) {
  clearCountdown(id)
  toasts.items = toasts.items.filter((t) => t.id !== id)
}
