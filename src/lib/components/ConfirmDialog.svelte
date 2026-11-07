<script lang="ts">
  import { confirmState, settleConfirm } from '../stores/confirm.svelte'

  let confirmButton = $state<HTMLButtonElement>()

  const current = $derived(confirmState.current)
  const kind = $derived(current?.kind ?? 'info')

  function onKeydown(event: KeyboardEvent) {
    if (!current || event.key !== 'Escape') return
    // Do not let the app's other overlays close behind this one.
    event.stopPropagation()
    settleConfirm(false)
  }

  $effect(() => {
    if (current) confirmButton?.focus()
  })
</script>

<svelte:window onkeydown={onKeydown} />

{#if current}
  <div class="overlay">
    <button class="scrim" aria-label="Cancel" onclick={() => settleConfirm(false)}></button>
    <div
      class="dialog {kind}"
      role={kind === 'info' ? 'dialog' : 'alertdialog'}
      aria-modal="true"
      aria-labelledby="confirm-title"
      aria-describedby="confirm-message"
    >
      <div class="head">
        <span class="icon" aria-hidden="true">{kind === 'info' ? 'i' : '!'}</span>
        <h2 id="confirm-title">{current.title}</h2>
      </div>
      <p id="confirm-message">{current.message}</p>
      <div class="actions">
        <button onclick={() => settleConfirm(false)}>{current.cancelLabel ?? 'Cancel'}</button>
        <button
          class="confirm"
          bind:this={confirmButton}
          onclick={() => settleConfirm(true)}
        >
          {current.confirmLabel ?? 'Confirm'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 110;
    display: grid;
    place-items: center;
  }
  .scrim {
    position: absolute;
    inset: 0;
    border: 0;
    border-radius: 0;
    background: var(--scrim);
  }
  .dialog {
    position: relative;
    z-index: 1;
    width: min(420px, calc(100vw - 48px));
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--r-overlay);
    background: var(--surface-3);
    box-shadow: var(--shadow-2);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .icon {
    display: grid;
    flex: 0 0 20px;
    box-sizing: border-box;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: var(--r-pill);
    background: var(--surface-2);
    color: var(--accent-fg);
    font-size: var(--fs-sm);
    font-weight: 700;
    line-height: 1;
  }
  .dialog.warning .icon {
    border: 1px solid var(--warn);
    background: transparent;
    color: var(--warn);
  }
  .dialog.error .icon {
    background: var(--err);
    color: var(--on-accent);
  }
  h2 {
    margin: 0;
    color: var(--text);
    font-size: var(--fs-md);
    font-weight: 600;
  }
  p {
    margin: 10px 0 16px;
    color: var(--text);
    font-size: var(--fs-sm);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .actions button {
    padding: 4px 12px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-sm);
  }
  .actions button:hover {
    background: var(--surface-2);
  }
  .actions .confirm {
    border-color: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  .dialog.warning .confirm,
  .dialog.error .confirm {
    border-color: var(--err);
    color: var(--err);
  }
</style>
