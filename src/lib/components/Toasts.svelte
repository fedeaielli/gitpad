<script lang="ts">
  import { fly } from 'svelte/transition'
  import { flip } from 'svelte/animate'
  import { toasts, dismissToast, type ToastKind } from '../stores/toasts.svelte'

  function icon(kind: ToastKind) {
    if (kind === 'error' || kind === 'warning') return '!'
    return kind === 'success' ? '✓' : 'i'
  }
</script>

<div class="toasts" aria-label="Notifications">
  {#each toasts.items as toast (toast.id)}
    <div
      class="toast {toast.kind}"
      role={toast.kind === 'error' ? 'alert' : 'status'}
      transition:fly={{ y: 6, duration: 120 }}
      animate:flip={{ duration: 150 }}
    >
      <span class="icon" aria-hidden="true">{icon(toast.kind)}</span>
      <span class="message">{toast.message}</span>
      <button
        class="close"
        aria-label="Dismiss notification"
        onclick={() => dismissToast(toast.id)}
      >
        ×
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 12px;
    bottom: calc(var(--h-statusbar) + 16px);
    z-index: 70;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(380px, calc(100vw - 24px));
    pointer-events: none;
  }
  .toast {
    --kind: var(--accent-fg);
    display: grid;
    grid-template-columns: 20px 1fr 22px;
    align-items: start;
    gap: 10px;
    padding: 11px 12px 11px 14px;
    border: 1px solid var(--border);
    border-left: 3px solid var(--kind);
    border-radius: var(--r-md);
    background: var(--surface-2);
    box-shadow: var(--shadow-1);
    pointer-events: auto;
  }
  .toast.error {
    --kind: var(--err);
  }
  .toast.warning {
    --kind: var(--warn);
  }
  .toast.success,
  .toast.info {
    --kind: var(--accent-fg);
  }
  .icon {
    display: grid;
    place-items: center;
    box-sizing: border-box;
    width: 20px;
    height: 20px;
    border-radius: var(--r-pill);
    background: var(--surface-3);
    color: var(--kind);
    font-size: var(--fs-sm);
    font-weight: 700;
    line-height: 1;
  }
  .toast.error .icon {
    background: var(--err);
    color: var(--on-accent);
  }
  .toast.warning .icon {
    border: 1px solid var(--warn);
    background: transparent;
  }
  .message {
    color: var(--text);
    font-size: var(--fs-sm);
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .close {
    display: grid;
    align-self: center;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    color: var(--muted);
    font-size: var(--fs-lg);
    line-height: 1;
  }
  .close:hover {
    background: var(--surface-3);
    color: var(--text);
  }
</style>
