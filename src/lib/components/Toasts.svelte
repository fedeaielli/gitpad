<script lang="ts">
  import { fly } from 'svelte/transition'
  import { flip } from 'svelte/animate'
  import { toasts, dismissToast } from '../stores/toasts.svelte'
</script>

<div class="toasts" aria-label="Notifications">
  {#each toasts.items as toast (toast.id)}
    <div
      class="toast {toast.kind}"
      role={toast.kind === 'error' ? 'alert' : 'status'}
      transition:fly={{ y: 6, duration: 120 }}
      animate:flip={{ duration: 150 }}
    >
      <span class="icon" aria-hidden="true">
        {#if toast.kind === 'error'}!{:else if toast.kind === 'success'}✓{:else}i{/if}
      </span>
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
    box-shadow: var(--shadow-2);
    pointer-events: auto;
  }
  .toast.error {
    --kind: var(--err);
  }
  .toast.success {
    --kind: var(--ok);
  }
  .toast.info {
    --kind: var(--accent-fg);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: var(--r-pill);
    background: var(--surface-3);
    color: var(--kind);
    font-size: 12px;
    font-weight: 700;
    line-height: 1;
  }
  .message {
    color: var(--text);
    font-size: 13px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .close {
    display: grid;
    align-self: center;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    color: var(--muted);
    font-size: 15px;
    line-height: 1;
  }
  .close:hover {
    background: var(--surface-3);
    color: var(--text);
  }
</style>
