<script lang="ts">
  import { prefersReducedMotion } from 'svelte/motion'
  import { quintOut } from 'svelte/easing'
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
  let pointerInside = false
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

  // shadcn/Base UI motion: slide a full 150% of the card out of and back into
  // the tray, 500ms on the same curve as the stack.
  function rise(_node: Element) {
    return {
      duration: prefersReducedMotion.current ? 0 : 500,
      easing: quintOut,
      css: (t: number) => `translate: 0 ${(1 - t) * 150}%;`,
    }
  }

  function fall(_node: Element) {
    // Svelte runs an outro with `t` going 1 → 0, so p is the exit progress.
    // shadcn slides on its fast curve and fades on a softer one, so the card
    // stays readable while it leaves instead of blinking out.
    return {
      duration: prefersReducedMotion.current ? 0 : 500,
      css: (t: number) => {
        const p = 1 - t
        const slide = 1 - (1 - p) ** 5
        return `opacity: ${(1 - p) ** 2}; translate: 0 ${slide * 150}%;`
      },
    }
  }

  function onFocusOut(event: FocusEvent) {
    const tray = event.currentTarget as HTMLElement
    const next = event.relatedTarget as Node | null
    if (next && tray.contains(next)) return
    // A removed close button drops focus to the body; the pointer still over
    // the tray must not collapse the stack under the user.
    if (pointerInside) return
    expanded = false
  }
</script>

{#snippet kindIcon(kind: ToastKind)}
  <svg
    class="icon {kind}"
    viewBox="0 0 24 24"
    width="16"
    height="16"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    {#if kind === 'success'}
      <circle cx="12" cy="12" r="10" />
      <path d="m9 12 2 2 4-4" />
    {:else if kind === 'error'}
      <path
        d="M8.688 2h6.624a2 2 0 0 1 1.414.586l4.688 4.688A2 2 0 0 1 22 8.688v6.624a2 2 0 0 1-.586 1.414l-4.688 4.688a2 2 0 0 1-1.414.586H8.688a2 2 0 0 1-1.414-.586l-4.688-4.688A2 2 0 0 1 2 15.312V8.688a2 2 0 0 1 .586-1.414l4.688-4.688A2 2 0 0 1 8.688 2Z"
      />
      <path d="m15 9-6 6" />
      <path d="m9 9 6 6" />
    {:else if kind === 'warning'}
      <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
      <path d="M12 9v4" />
      <path d="M12 17h.01" />
    {:else}
      <circle cx="12" cy="12" r="10" />
      <path d="M12 16v-4" />
      <path d="M12 8h.01" />
    {/if}
  </svg>
{/snippet}

<!-- One tray, newest card in front; the older ones peek above it until the
     pointer or keyboard focus opens the stack. It stays mounted with no cards
     so the last exit animation can play. -->
<div
  class="tray"
  role="region"
  aria-label="Notifications"
  onmouseenter={() => {
    pointerInside = true
    expanded = true
  }}
  onmouseleave={() => {
    pointerInside = false
    expanded = false
  }}
  onfocusin={() => (expanded = true)}
  onfocusout={onFocusOut}
>
  {#each toasts.items as toast, i (toast.id)}
    {@const placement = stackPlacement(i, ids, heights, expanded)}
    <div class="slot" style="z-index: {i + 1}" in:rise out:fall>
      <div
        class="toast {toast.kind}"
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
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18M6 6l12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
          </svg>
        </button>
      </div>
    </div>
  {/each}
</div>

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
    --kind: var(--accent-fg);
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-3) var(--sp-4);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: color-mix(in srgb, var(--kind) 7%, var(--surface-3));
    box-shadow: var(--shadow-1);
    color: var(--text);
    font-size: var(--fs-md);
    pointer-events: auto;
    transform-origin: bottom center;
    /* Same clock and curve as the entry and exit, so the stack moves with the
       card instead of after it. */
    transition: transform 500ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .toast.success {
    --kind: var(--ok);
  }
  .toast.warning {
    --kind: var(--warn);
  }
  .toast.error {
    --kind: var(--err);
  }
  /* Severity lives in the icon and a 7% wash; the border stays neutral. */
  .icon {
    flex: none;
    color: var(--kind);
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
