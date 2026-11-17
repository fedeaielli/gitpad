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

  function follow(toast: Toast) {
    if (!toast.url) return
    void openExternal(toast.url)
    dismissToast(toast.id)
  }
</script>

{#snippet kindIcon(kind: ToastKind)}
  <svg
    class="icon"
    viewBox="0 0 16 16"
    width="16"
    height="16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    aria-hidden="true"
  >
    <circle cx="8" cy="8" r="6.25" />
    {#if kind === 'success'}
      <path d="M5.2 8.2 7.2 10.2 10.8 5.9" />
    {:else if kind === 'error'}
      <path d="M6.1 6.1l3.8 3.8M9.9 6.1l-3.8 3.8" />
    {:else if kind === 'warning'}
      <path d="M8 4.7v4.1M8 11.3v.1" />
    {:else}
      <path d="M8 7.3v4M8 4.7v.1" />
    {/if}
  </svg>
{/snippet}

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
      {@render kindIcon(toast.kind)}
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
    grid-template-columns: 16px 1fr 20px;
    align-items: start;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-2) var(--sp-2) var(--sp-3);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-2);
    /* Severity as a left edge, not a floating badge: inset keeps the corners round. */
    box-shadow:
      inset 3px 0 0 var(--kind),
      var(--shadow-1);
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
    display: block;
    color: var(--kind);
  }
  .body {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: var(--sp-0);
  }
  .message {
    color: var(--text);
    font-size: var(--fs-sm);
    line-height: 1.35;
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
    width: 20px;
    height: 20px;
    /* Centred on the 16px first line, not on the card. */
    margin-top: -2px;
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
