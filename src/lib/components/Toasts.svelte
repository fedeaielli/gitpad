<script lang="ts">
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
  import { stackPlacement } from './toastStack'

  let expanded = $state(false)
  let heights = $state<Record<number, number>>({})
  const ids = $derived(toasts.items.map((toast) => toast.id))

  // Keep the measured heights bounded to the cards on screen.
  $effect(() => {
    const live = new Set(ids)
    for (const key of Object.keys(heights)) {
      const id = Number(key)
      if (!live.has(id)) delete heights[id]
    }
  })

  function follow(toast: Toast) {
    if (!toast.url) return
    void openExternal(toast.url)
    dismissToast(toast.id)
  }

  function rise(_node: Element) {
    return {
      duration: prefersReducedMotion.current ? 0 : 180,
      css: (t: number) => `opacity: ${t}; translate: 0 ${(1 - t) * 16}px;`,
    }
  }

  function fall(_node: Element) {
    return {
      duration: prefersReducedMotion.current ? 0 : 120,
      css: (t: number) => `opacity: ${1 - t}; translate: 0 ${t * 8}px;`,
    }
  }

  function onFocusOut(event: FocusEvent) {
    const next = event.relatedTarget as Node | null
    if (!next || !(event.currentTarget as HTMLElement).contains(next)) expanded = false
  }
</script>

{#snippet kindIcon(kind: ToastKind)}
  <svg
    class="icon {kind}"
    viewBox="0 0 16 16"
    width="16"
    height="16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    {#if kind === 'success'}
      <circle cx="8" cy="8" r="6.25" />
      <path d="M5.2 8.2 7.2 10.2 10.8 5.9" />
    {:else if kind === 'error'}
      <path d="M5.5 2.5h5l3 3v5l-3 3h-5l-3-3v-5l3-3Z" />
      <path d="m6.2 6.2 3.6 3.6m-.1-3.6-3.6 3.6" />
    {:else if kind === 'warning'}
      <path d="M8 2.7 14 13H2L8 2.7Z" />
      <path d="M8 6.3v3.3M8 11.4v.1" />
    {:else}
      <circle cx="8" cy="8" r="6.25" />
      <path d="M8 7.3v4M8 4.7v.1" />
    {/if}
  </svg>
{/snippet}

{#if toasts.items.length > 0}
  <!-- One tray, newest card in front; the older ones peek above it until the
       pointer or keyboard focus opens the stack. -->
  <div
    class="tray"
    role="region"
    aria-label="Notifications"
    onmouseenter={() => (expanded = true)}
    onmouseleave={() => (expanded = false)}
    onfocusin={() => (expanded = true)}
    onfocusout={onFocusOut}
  >
    {#each toasts.items as toast, i (toast.id)}
      {@const placement = stackPlacement(i, ids, heights, expanded)}
      <div class="slot" style="z-index: {i + 1}" in:rise out:fall>
        <div
          class="toast"
          role={toast.kind === 'error' ? 'alert' : 'status'}
          style="transform: translateY(-{placement.offsetY}px) scale({placement.scale})"
          bind:offsetHeight={heights[toast.id]}
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
          </div>
          {#if toast.url}
            <button class="btn action" onclick={() => follow(toast)}>
              {toast.action ?? 'Open link'}
            </button>
          {/if}
          <button
            class="icon-btn close"
            aria-label="Dismiss notification"
            onclick={() => dismissToast(toast.id)}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M5 5l6 6M11 5l-6 6" stroke="currentColor" stroke-width="1.3" />
            </svg>
          </button>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .tray {
    position: fixed;
    right: var(--sp-4);
    bottom: calc(var(--h-statusbar) + var(--sp-4));
    z-index: 70;
    width: min(384px, calc(100vw - var(--sp-6)));
    pointer-events: none;
  }
  /* The slot is unpositioned in the stack and carries the enter/exit motion;
     the card inside moves and scales to its place in the stack. */
  .slot {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: var(--surface-3);
    box-shadow: var(--shadow-1);
    color: var(--text);
    font-size: var(--fs-md);
    pointer-events: auto;
    transform-origin: bottom center;
    transition: transform var(--t-spring) var(--ease-spring);
  }
  /* Severity lives in the icon: shadcn keeps the surface neutral. */
  .icon {
    flex: none;
    color: var(--muted);
  }
  .icon.error {
    color: var(--err);
  }
  .body {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: var(--sp-1);
  }
  .title {
    font-weight: 600;
    line-height: 1.35;
  }
  .title + .message {
    color: var(--muted);
  }
  .message {
    line-height: 1.35;
    overflow-wrap: anywhere;
  }
  .action {
    flex: none;
  }
  .close {
    flex: none;
  }
  .close svg {
    display: block;
    width: 14px;
    height: 14px;
  }
  @media (prefers-reduced-motion: reduce) {
    .toast {
      transition: none;
    }
  }
</style>
