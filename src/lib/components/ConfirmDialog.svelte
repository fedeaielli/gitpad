<script lang="ts">
  import { fade } from 'svelte/transition'
  import { prefersReducedMotion } from 'svelte/motion'
  import { confirmState, settleConfirm } from '../stores/confirm.svelte'
  import { focusTrap } from '../utils/focus'

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
    // Only an info dialog leads with its primary action. On a warning or an
    // error the trap's first control (Cancel) must keep the focus, or Enter
    // discards the work the dialog is asking about.
    if (current && kind === 'info') confirmButton?.focus()
  })
</script>

<svelte:window onkeydown={onKeydown} />

{#if current}
  <div class="overlay" out:fade={{ duration: prefersReducedMotion.current ? 0 : 120 }}>
    <button class="scrim" aria-label="Cancel" onclick={() => settleConfirm(false)}></button>
    <div
      class="dialog {kind}"
      role={kind === 'info' ? 'dialog' : 'alertdialog'}
      aria-modal="true"
      aria-labelledby="confirm-title"
      aria-describedby="confirm-message"
      use:focusTrap
    >
      <div class="head">
        <span class="icon" aria-hidden="true">{kind === 'info' ? 'i' : '!'}</span>
        <h2 id="confirm-title">{current.title}</h2>
      </div>
      <p id="confirm-message">{current.message}</p>
      <div class="actions">
        <button class="btn" onclick={() => settleConfirm(false)}>{current.cancelLabel ?? 'Cancel'}</button>
        <button
          class="btn {kind === 'info' ? 'btn-primary' : 'btn-danger'}"
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
    animation: gp-fade-in var(--t-base) var(--ease-out);
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
    padding: var(--sp-4);
    border: 1px solid var(--border);
    border-radius: var(--r-overlay);
    background: var(--surface-1);
    box-shadow: var(--shadow-2);
    animation: gp-panel-in var(--t-spring) var(--ease-spring);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-25);
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
    margin: var(--sp-25) 0 var(--sp-4);
    color: var(--text);
    font-size: var(--fs-sm);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }
</style>
