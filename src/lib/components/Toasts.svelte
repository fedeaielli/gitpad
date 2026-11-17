<script lang="ts">
  import { fly } from 'svelte/transition'
  import { flip } from 'svelte/animate'
  import { prefersReducedMotion } from 'svelte/motion'
  import {
    toasts,
    dismissToast,
    pauseToast,
    resumeToast,
    type Toast,
    type ToastKind,
  } from '../stores/toasts.svelte'
  import { openExternal } from '../utils/open'

  function icon(kind: ToastKind) {
    if (kind === 'error' || kind === 'warning') return '!'
    return kind === 'success' ? '✓' : 'i'
  }

  function follow(toast: Toast) {
    if (!toast.url) return
    void openExternal(toast.url)
    dismissToast(toast.id)
  }
</script>

<div class="toasts" role="region" aria-label="Notifications">
  {#each toasts.items as toast (toast.id)}
    <div
      class="toast {toast.kind}"
      role={toast.kind === 'error' ? 'alert' : 'status'}
      in:fly={{ y: 8, duration: prefersReducedMotion.current ? 0 : 140 }}
      out:fly={{ y: 4, duration: prefersReducedMotion.current ? 0 : 100 }}
      animate:flip={{ duration: prefersReducedMotion.current ? 0 : 150 }}
      onmouseenter={() => pauseToast(toast.id)}
      onmouseleave={() => resumeToast(toast.id)}
      onfocusin={() => pauseToast(toast.id)}
      onfocusout={() => resumeToast(toast.id)}
    >
      <span class="icon" aria-hidden="true">{icon(toast.kind)}</span>
      <div class="body">
        <span class="message">{toast.message}</span>
        {#if toast.url}
          <button class="action" onclick={() => follow(toast)}>
            {toast.action ?? 'Open link'}
          </button>
        {/if}
      </div>
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
    right: var(--sp-3);
    bottom: calc(var(--h-statusbar) + var(--sp-4));
    z-index: 70;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    width: min(380px, calc(100vw - 24px));
    pointer-events: none;
  }
  .toast {
    --kind: var(--accent-fg);
    display: grid;
    grid-template-columns: 20px 1fr 24px;
    align-items: start;
    gap: var(--sp-25);
    padding: var(--sp-3) var(--sp-3) var(--sp-3) var(--sp-4);
    border: 1px solid var(--border);
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
  .body {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: var(--sp-05);
  }
  .message {
    color: var(--text);
    font-size: var(--fs-sm);
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  /* The action names what it does; the message itself is not a link. */
  .action {
    align-self: start;
    padding: 0;
    border: none;
    background: none;
    color: var(--accent-fg);
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    text-decoration: none;
  }
  .action:hover {
    text-decoration: underline;
  }
  .close {
    display: grid;
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
