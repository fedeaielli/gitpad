<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { ui } from '../stores/ui.svelte'
  import MenuBar from './MenuBar.svelte'
  import { tip } from '../utils/tooltip'

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
    <span class="logo" aria-hidden="true"></span>
    GitPad
  </span>
  <MenuBar />
  <button
    type="button"
    class="search"
    aria-label="Search files, or type > for commands (Ctrl+P)"
    use:tip={'Search files (Ctrl+P) — type > for commands'}
    onclick={() => (ui.commandPaletteOpen = true)}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="7" cy="7" r="4.2" />
      <path d="M10.2 10.2 13.5 13.5" />
    </svg>
    <span class="label">Search files</span>
    <span class="chip">&gt; commands</span>
    <kbd>Ctrl+P</kbd>
  </button>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="controls">
    <button class="win" aria-label="Minimize" use:tip={'Minimize'} onclick={() => void appWindow.minimize()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" /></svg>
    </button>
    <button
      class="win"
      aria-label={maximized ? 'Restore' : 'Maximize'}
      use:tip={maximized ? 'Restore' : 'Maximize'}
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
    <button class="win close" aria-label="Close" use:tip={'Close'} onclick={() => void appWindow.close()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    position: relative;
    display: flex;
    flex: 0 0 var(--h-titlebar);
    align-items: center;
    gap: 10px;
    padding-left: 10px;
    background: var(--surface-base);
    user-select: none;
  }
  /* Centred on the window, not on the space left by the title and controls. */
  .search {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    box-sizing: border-box;
    width: 40%;
    min-width: 200px;
    max-width: 480px;
    height: 22px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-2);
    color: var(--text);
    font-size: 12px;
    text-align: left;
  }
  .search:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .search:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: 1px;
  }
  .search svg {
    width: 12px;
    height: 12px;
    flex: 0 0 auto;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
  }
  .search .label {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  /* The label is the only primary text; the chip and the key are secondary (>= 3:1). */
  .search .chip,
  .search kbd {
    flex: 0 0 auto;
    color: var(--muted);
    font-size: 10px;
    white-space: nowrap;
  }
  .search .chip {
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
  }
  /* Narrow windows: icon only, so it cannot collide with the title or controls. */
  @media (max-width: 560px) {
    .search {
      width: 24px;
      min-width: 0;
      justify-content: center;
      padding: 0;
    }
    .search .label,
    .search .chip,
    .search kbd {
      display: none;
    }
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  /* A mask, not an image: the mark takes the title colour in every theme. */
  .logo {
    flex: 0 0 auto;
    width: 16px;
    height: 18px;
    background: var(--muted);
    mask: url('/logo.png') center / contain no-repeat;
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
    background: var(--surface-2);
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
