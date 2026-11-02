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

  let resizing = $state(false)
  let startX = 0
  let startWidth = 0

  function startResize(e: PointerEvent) {
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    startX = e.clientX
    startWidth = ui.sidebarWidth
    resizing = true
  }
  function moveResize(e: PointerEvent) {
    if (!resizing) return
    ui.sidebarWidth = Math.min(500, Math.max(150, startWidth + e.clientX - startX))
  }
  function endResize() {
    resizing = false
  }
</script>

<svelte:window onkeydown={onKeydown} />

<aside class="panel" style="flex: 0 0 {ui.sidebarWidth}px">
  <div
    class="resize"
    class:active={resizing}
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize sidebar"
    aria-valuemin="150"
    aria-valuemax="500"
    aria-valuenow={Math.round(ui.sidebarWidth)}
    onpointerdown={startResize}
    onpointermove={moveResize}
    onpointerup={endResize}
    onpointercancel={endResize}
  ></div>
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
      <FileTree />
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
    flex: 0 0 260px;
    flex-direction: column;
    min-height: 0;
    border-radius: var(--r-lg);
    background: var(--surface-1);
  }
  .resize {
    position: absolute;
    top: 0;
    right: -2px;
    width: 4px;
    height: 100%;
    cursor: col-resize;
    z-index: 2;
  }
  .resize:hover,
  .resize.active {
    background: var(--accent);
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
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 15px;
    line-height: 22px;
    padding: 0 5px;
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
