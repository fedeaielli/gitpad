import { invoke } from '@tauri-apps/api/core'

export type ToastKind = 'error' | 'warning' | 'info' | 'success'

export interface Toast {
  id: number
  /// Optional heading; the message becomes its quieter second line.
  title?: string
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

/// How long a toast lives, in ms. Severity buys reading time, and an action
/// buys a little more so it stays clickable.
const LIFETIME: Record<ToastKind, number> = {
  info: 2500,
  success: 3000,
  warning: 4500,
  error: 6000,
}

const ACTION_BONUS: Record<ToastKind, number> = {
  info: 1000,
  success: 1000,
  warning: 1500,
  error: 1500,
}

export function toastLifetime(kind: ToastKind, hasAction: boolean): number {
  return LIFETIME[kind] + (hasAction ? ACTION_BONUS[kind] : 0)
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
  opts?: { title?: string; url?: string; action?: string },
) {
  const id = ++seq
  const toast: Toast = { id, message, kind, ...opts }
  const next = [...toasts.items, toast]
  for (const dropped of next.slice(0, Math.max(0, next.length - 3))) clearCountdown(dropped.id)
  toasts.items = next.slice(-3)
  const ttl = toastLifetime(kind, Boolean(opts?.url))
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
  toasts.items = toasts.items.filter((toast) => toast.id !== id)
}
