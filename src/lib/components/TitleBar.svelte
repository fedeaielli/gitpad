<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { ui } from '../stores/ui.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { getEditor } from '../editor/bridge'
  import { undoState } from '../editor/history.svelte'
  import MenuBar from './MenuBar.svelte'
  import { tip } from '../utils/tooltip'

  const appWindow = getCurrentWindow()
  let maximized = $state(false)
  const hasTab = $derived(tabs.tabs.length > 0)

  /// Undo/redo run through the shared editor bridge, like the Edit menu does.
  function step(action: 'undo' | 'redo') {
    const editor = getEditor()
    editor?.focus()
    editor?.trigger('titlebar', action, null)
  }

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
  <div class="center">
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
    <div class="nav">
      <button
        type="button"
        class="step"
        aria-label="Undo (Ctrl+Z)"
        use:tip={'Undo (Ctrl+Z)'}
        disabled={!hasTab || !undoState.canUndo}
        onclick={() => step('undo')}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3 5 8l5 5" /></svg>
      </button>
      <button
        type="button"
        class="step"
        aria-label="Redo (Ctrl+Y)"
        use:tip={'Redo (Ctrl+Y)'}
        disabled={!hasTab || !undoState.canRedo}
        onclick={() => step('redo')}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3l5 5-5 5" /></svg>
      </button>
    </div>
  </div>
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
  .center {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    width: 40%;
    min-width: 240px;
    max-width: 520px;
  }
  .search {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 6px;
    min-width: 0;
    box-sizing: border-box;
    height: 22px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-2);
    color: var(--text);
    font-size: var(--fs-sm);
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
  /* Undo / redo, bare chevrons beside the search field, as in the VS Code title bar. */
  .nav {
    display: flex;
    flex: 0 0 auto;
    gap: 2px;
  }
  .step {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: var(--r-sm);
    color: var(--muted);
  }
  .step:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }
  .step:disabled {
    /* Unmistakable: an arrow that cannot act must not look clickable. */
    opacity: 0.35;
    cursor: default;
  }
  .step svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
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
    font-size: var(--fs-2xs);
    white-space: nowrap;
  }
  /* A suffix of the field, not a badge floating inside it: separator, no box. */
  .search .chip {
    padding-left: 8px;
    border-left: 1px solid var(--border);
  }
  /* Narrow windows: icon only, so it cannot collide with the title or controls. */
  @media (max-width: 560px) {
    .search {
      flex: 0 0 24px;
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
    font-size: var(--fs-sm);
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
