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
    border: 0;
    border-left: 2px solid var(--border-strong);
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: var(--shadow-1);
    color: var(--text);
    text-align: left;
    font-size: 12px;
  }
  .toast.error {
    border-left-color: var(--err);
    color: var(--err);
  }
  .toast.info {
    border-left-color: var(--warn);
  }
  .toast.success {
    border-left-color: var(--ok);
  }
</style>
