<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'

  const appWindow = getCurrentWindow()
</script>

<header class="titlebar" data-tauri-drag-region>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span class="title" data-tauri-drag-region ondblclick={() => void appWindow.toggleMaximize()}>
    GitPad
  </span>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="lights">
    <button class="dot" aria-label="Close" title="Close" onclick={() => void appWindow.close()}>
      <svg viewBox="0 0 12 12" aria-hidden="true"><circle cx="6" cy="6" r="6" fill="#ff5f57" /></svg>
    </button>
    <button class="dot" aria-label="Minimize" title="Minimize" onclick={() => void appWindow.minimize()}>
      <svg viewBox="0 0 12 12" aria-hidden="true"><circle cx="6" cy="6" r="6" fill="#febc2e" /></svg>
    </button>
    <button
      class="dot"
      aria-label="Maximize"
      title="Maximize"
      onclick={() => void appWindow.toggleMaximize()}
    >
      <svg viewBox="0 0 12 12" aria-hidden="true"><circle cx="6" cy="6" r="6" fill="#28c840" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    flex: 0 0 32px;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
    user-select: none;
  }
  .lights {
    display: flex;
    gap: 8px;
  }
  .dot {
    display: flex;
    padding: 0;
  }
  .dot svg {
    width: 12px;
    height: 12px;
  }
  .dot:hover svg {
    filter: brightness(1.25);
  }
  .title {
    color: var(--muted);
    font-size: 12px;
  }
  .spacer {
    flex: 1;
  }
</style>
