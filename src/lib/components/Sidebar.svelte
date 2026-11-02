<script lang="ts">
  import FileTree from './FileTree.svelte'
  import GitPanel from './GitPanel.svelte'
  import HistoryPanel from './HistoryPanel.svelte'
  import { ui } from '../stores/ui.svelte'
  import { defaultDesc, SORT_MODES, type SortMode } from '../utils/file-sort'
  import { tip } from '../utils/tooltip'

  const SORT_LABELS: Record<SortMode, string> = {
    name: 'Name',
    modified: 'Modified',
    folders: 'Folders first',
  }

  let menuOpen = $state(false)
  let trigger = $state<HTMLButtonElement>()
  let tree = $state<{ createAtRoot: (kind: 'file' | 'folder') => void }>()

  /// Below this width the header cannot hold the title and three buttons on one line.
  const NARROW = 200

  /// Picking a mode also snaps the direction back to that mode's natural one.
  function chooseSort(mode: SortMode) {
    ui.sortMode = mode
    ui.sortDesc = defaultDesc(mode)
  }

  function closeMenu(refocus = false) {
    menuOpen = false
    if (refocus) trigger?.focus()
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && menuOpen) {
      e.stopPropagation()
      closeMenu(true)
    }
  }

</script>

<svelte:window onkeydown={onKeydown} />

<aside class="panel">
  <div class="head">
    <span>
      {ui.activeView === 'files'
        ? 'Explorer'
        : ui.activeView === 'git'
          ? 'Source control'
          : 'History'}
    </span>
    {#if ui.activeView === 'files'}
      <div class="actions">
        {#if ui.sidebarWidth >= NARROW}
          <button use:tip={'New file'} aria-label="New file" onclick={() => tree?.createAtRoot('file')}>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M4 1.8h5.4l2.8 2.8v9.6H4z" />
              <path d="M9.4 1.8v2.8h2.8" />
              <path d="M8.1 8.2v4.2M6 10.3h4.2" />
            </svg>
          </button>
          <button
            use:tip={'New folder'}
            aria-label="New folder"
            onclick={() => tree?.createAtRoot('folder')}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M1.8 3.8h4.4l1.4 1.8h6.6v6.8H1.8z" />
              <path d="M8.3 7.4v3.4M6.6 9.1h3.4" />
            </svg>
          </button>
        {/if}
        <button
          class="options"
          bind:this={trigger}
          use:tip={'Tree options'}
          aria-label="Tree options"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={() => (menuOpen = !menuOpen)}
        >
          ⋯
        </button>
      </div>
      {#if menuOpen}
          <!-- Click-anywhere-else closes the menu without stealing the click. -->
          <button class="scrim" aria-label="Close menu" onclick={() => closeMenu()}></button>
          <div class="menu" role="menu" aria-label="Tree options">
            <p class="menu-head">Sort by</p>
            {#each SORT_MODES as mode (mode)}
              <button
                type="button"
                class="menu-item"
                role="menuitemradio"
                aria-checked={ui.sortMode === mode}
                onclick={() => {
                  chooseSort(mode)
                  closeMenu()
                }}
              >
                <span class="tick">{ui.sortMode === mode ? '✓' : ''}</span>
                {SORT_LABELS[mode]}
              </button>
            {/each}
            <button
              type="button"
              class="menu-item"
              role="menuitemcheckbox"
              aria-checked={ui.sortDesc}
              onclick={() => (ui.sortDesc = !ui.sortDesc)}
            >
              <span class="tick">{ui.sortDesc ? '✓' : ''}</span>
              Descending
            </button>
            <hr />
            <button
              type="button"
              class="menu-item"
              role="menuitemcheckbox"
              aria-checked={ui.showHidden}
              onclick={() => (ui.showHidden = !ui.showHidden)}
            >
              <span class="tick">{ui.showHidden ? '✓' : ''}</span>
              Show hidden files
            </button>
            <button
              type="button"
              class="menu-item"
              role="menuitem"
              disabled={ui.expanded.length === 0}
              onclick={() => {
                ui.expanded = []
                closeMenu()
              }}
            >
              <span class="tick"></span>
              Collapse all
            </button>
          </div>
      {/if}
    {/if}
  </div>
  <div class="scroll">
    {#if ui.activeView === 'files'}
      <FileTree bind:this={tree} />
    {:else if ui.activeView === 'git'}
      <GitPanel />
    {:else}
      <HistoryPanel />
    {/if}
  </div>
</aside>

<style>
  .panel {
    position: relative;
    display: flex;
    /* Fills the wrapper, which owns the width; min-width keeps long names from
       pushing the panel wider than the drag asked for. */
    flex: 1 1 auto;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    /* Rounded by the wrapper that also holds the activity bar. */
    border-radius: 0;
    background: var(--surface-1);
  }
  .head {
    position: relative;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
    /* Same height as the tab bar so the two headers line up. */
    box-sizing: border-box;
    min-height: var(--h-tabbar);
    padding: 2px 8px;
    color: var(--muted);
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .actions {
    display: flex;
    gap: 4px;
    margin-left: auto;
  }
  .actions button {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 22px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 15px;
    line-height: 22px;
    padding: 0 5px;
  }
  .actions svg {
    display: block;
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .actions button:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    border: 0;
    background: transparent;
  }
  .menu {
    position: absolute;
    top: calc(100% - 2px);
    right: 6px;
    z-index: 41;
    min-width: 180px;
    padding: var(--sp-1);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-3);
    box-shadow: var(--shadow-2);
    font-size: var(--fs-sm);
    letter-spacing: normal;
    text-transform: none;
  }
  .menu-head {
    margin: 0;
    padding: var(--sp-1) var(--sp-2);
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  .menu hr {
    margin: var(--sp-1) 0;
    border: 0;
    border-top: 1px solid var(--border);
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    width: 100%;
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-sm);
    text-align: left;
  }
  .menu-item:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .menu-item:disabled {
    color: var(--muted);
    cursor: default;
  }
  .tick {
    width: 12px;
    color: var(--accent-fg);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
