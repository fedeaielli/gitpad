<script lang="ts">
  import { toasts, dismissToast } from '../stores/toasts.svelte'
</script>

<div class="toasts" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <button class="toast {toast.kind}" onclick={() => dismissToast(toast.id)}>
      {toast.message}
    </button>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 12px;
    bottom: 32px;
    z-index: 70;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 360px;
  }
  .toast {
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-raised);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
    color: var(--text);
    text-align: left;
    font-size: 12px;
  }
  .toast.error {
    border-color: var(--err);
    color: var(--err);
  }
</style>
