<script lang="ts">
  import { fade, fly } from 'svelte/transition'
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

{#if toasts.items.length > 0}
  <!-- One tray, not one card per toast: stacked cards read as floating menus. -->
  <div
    class="tray"
    role="region"
    aria-label="Notifications"
    in:fade={{ duration: prefersReducedMotion.current ? 0 : 120 }}
    out:fade={{ duration: prefersReducedMotion.current ? 0 : 100 }}
  >
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
          {#if toast.title}
            <span class="title">{toast.title}</span>
          {/if}
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
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M5 5l6 6M11 5l-6 6" stroke="currentColor" stroke-width="1.3" />
          </svg>
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .tray {
    position: fixed;
    right: var(--sp-3);
    bottom: calc(var(--h-statusbar) + var(--sp-4));
    z-index: 70;
    display: flex;
    flex-direction: column;
    width: min(380px, calc(100vw - 24px));
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-2);
    box-shadow: var(--shadow-1);
    pointer-events: none;
  }
  .toast {
    --kind: var(--accent-fg);
    position: relative;
    display: grid;
    grid-template-columns: 16px 1fr;
    align-items: start;
    gap: var(--sp-2);
    /* Right gap reserves the close button's column, which is out of flow. */
    padding: var(--sp-25) calc(var(--sp-3) + 24px) var(--sp-25) var(--sp-3);
    /* Severity as a left edge, not a badge: the tray keeps the outer shape. */
    box-shadow: inset 2px 0 0 var(--kind);
    pointer-events: auto;
  }
  .toast + .toast {
    border-top: 1px solid var(--divider);
  }
  .toast:hover {
    /* Tints with the text colour, so it lifts in every theme (hc-black's
       surface-3 is darker than its surface-2). */
    background: color-mix(in srgb, var(--text) 6%, transparent);
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
    gap: var(--sp-05);
  }
  .title {
    color: var(--text);
    font-size: var(--fs-sm);
    font-weight: 600;
    line-height: 1.35;
  }
  .title + .message {
    color: var(--muted);
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
    /* Centred on the row's full height, clear of the text flow. */
    position: absolute;
    top: 50%;
    right: var(--sp-3);
    translate: 0 -50%;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    color: var(--muted);
    opacity: 0.55;
    transition: opacity var(--t-fast);
  }
  .toast:hover .close,
  .toast:focus-within .close {
    opacity: 1;
  }
  .close svg {
    display: block;
    width: 14px;
    height: 14px;
  }
  .close:hover {
    background: color-mix(in srgb, var(--text) 12%, transparent);
    color: var(--text);
  }
</style>
