export type ConfirmKind = 'info' | 'warning' | 'error'

export interface ConfirmOptions {
  title: string
  message: string
  kind?: ConfirmKind
  confirmLabel?: string
  cancelLabel?: string
}

interface Pending extends ConfirmOptions {
  resolve: (ok: boolean) => void
}

export const confirmState = $state({ current: null as Pending | null })

/// One dialog at a time: a close prompt must not overwrite an open discard.
const queue: Pending[] = []

export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    queue.push({ ...options, resolve })
    if (!confirmState.current) showNext()
  })
}

function showNext() {
  confirmState.current = queue.shift() ?? null
}

export function settleConfirm(ok: boolean) {
  const done = confirmState.current
  if (!done) return
  confirmState.current = null
  done.resolve(ok)
  showNext()
}
