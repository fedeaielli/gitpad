<script lang="ts">
  import { ui } from '../stores/ui.svelte'
  import { focusTrap } from '../utils/focus'
  import { helpRows } from '../utils/shortcuts'

  function close() {
    ui.shortcutsOpen = false
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') close()
  }}
/>

<div class="overlay" role="presentation">
  <button class="backdrop" aria-label="Close keyboard shortcuts" onclick={close}></button>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Keyboard shortcuts" use:focusTrap>
    <header>
      <h2>Keyboard shortcuts</h2>
      <button class="icon-btn lg close" aria-label="Close keyboard shortcuts" onclick={close}>×</button>
    </header>
    <ul class="list">
      {#each helpRows as shortcut (shortcut.label)}
        <li>
          <span>{shortcut.label}</span>
          <span class="keys">
            {#each shortcut.keys.split('+') as key, i (key)}
              {#if i > 0}<span class="plus">+</span>{/if}
              <kbd>{key}</kbd>
            {/each}
          </span>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 45;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: gp-fade-in var(--t-base) var(--ease-out);
  }
  .backdrop {
    position: absolute;
    inset: 0;
    background: var(--scrim);
  }
  .modal {
    position: relative;
    display: flex;
    flex-direction: column;
    width: min(420px, 92vw);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--r-overlay);
    background: var(--surface-1);
    box-shadow: var(--shadow-2);
    animation: gp-panel-in var(--t-spring) var(--ease-spring);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-3) var(--sp-4);
    border-bottom: 1px solid var(--divider);
    background: var(--surface-1);
  }
  h2 {
    margin: 0;
    color: var(--text);
    font-size: var(--fs-lg);
    font-weight: 600;
  }
  .close {
    font-size: var(--fs-xl);
  }
  .list {
    margin: 0;
    padding: var(--sp-2) var(--sp-4) var(--sp-3);
    background: var(--surface-0);
    list-style: none;
  }
  .list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-4);
    padding: var(--sp-1) 0;
    color: var(--text);
    font-size: var(--fs-sm);
  }
  .keys {
    display: flex;
    align-items: center;
    gap: var(--sp-05);
    flex: 0 0 auto;
  }
  kbd {
    padding: var(--sp-0) var(--sp-05);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-1);
    color: var(--muted);
    font-family: inherit;
    font-size: var(--fs-xs);
  }
  .plus {
    color: var(--muted);
  }
</style>
