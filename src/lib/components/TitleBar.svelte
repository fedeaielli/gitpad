<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  const appWindow = getCurrentWindow()
  let maximized = $state(false)

  onMount(() => {
    const sync = () => void appWindow.isMaximized().then((v) => (maximized = v))
    sync()
    const unlisten = appWindow.onResized(sync)
    return () => void unlisten.then((fn) => fn())
  })
</script>

<header class="titlebar" data-tauri-drag-region>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span class="title" data-tauri-drag-region ondblclick={() => void appWindow.toggleMaximize()}>
    GitPad
  </span>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="controls">
    <button class="win" aria-label="Minimize" title="Minimize" onclick={() => void appWindow.minimize()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" /></svg>
    </button>
    <button
      class="win"
      aria-label={maximized ? 'Restore' : 'Maximize'}
      title={maximized ? 'Restore' : 'Maximize'}
      onclick={() => void appWindow.toggleMaximize()}
    >
      <svg viewBox="0 0 10 10" aria-hidden="true">
        {#if maximized}
          <rect x="0.5" y="2.5" width="7" height="7" />
          <rect x="2.5" y="0.5" width="7" height="7" />
        {:else}
          <rect x="0.5" y="0.5" width="9" height="9" />
        {/if}
      </svg>
    </button>
    <button class="win close" aria-label="Close" title="Close" onclick={() => void appWindow.close()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    flex: 0 0 32px;
    align-items: center;
    gap: 10px;
    padding-left: 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-panel);
    user-select: none;
  }
  .title {
    color: var(--muted);
    font-size: 12px;
  }
  .spacer {
    flex: 1;
  }
  .controls {
    display: flex;
    height: 100%;
  }
  .win {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
    color: var(--text);
  }
  .win:hover {
    background: var(--bg-raised);
  }
  .win.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .win svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
  }
</style>
