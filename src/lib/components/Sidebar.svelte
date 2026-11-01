<script lang="ts">
  import FileTree from './FileTree.svelte'
  import GitPanel from './GitPanel.svelte'
  import HistoryPanel from './HistoryPanel.svelte'
  import { ui } from '../stores/ui.svelte'

  const SORTS = ['name', 'folders', 'modified'] as const

  function cycleSort() {
    const next = SORTS[(SORTS.indexOf(ui.sortMode) + 1) % SORTS.length]
    ui.sortMode = next
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
          title={ui.showHidden ? 'Hide dot files' : 'Show dot files'}
          aria-pressed={ui.showHidden}
          onclick={() => (ui.showHidden = !ui.showHidden)}
        >
          {ui.showHidden ? '◉' : '◌'}
        </button>
        <button title="Sort by {ui.sortMode} (click to change)" onclick={cycleSort}>⇅</button>
        <button title="Collapse all folders" onclick={() => (ui.expanded = [])}>⊟</button>
      </div>
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
    background: var(--surface-1);
    border-right: 1px solid var(--border);
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
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
    /* Same height as the tab bar so both bottom borders line up. */
    box-sizing: border-box;
    min-height: var(--h-tabbar);
    padding: 2px 8px;
    border-bottom: 1px solid var(--border);
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
    border-radius: 3px;
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
  .actions button[aria-pressed='true'] {
    color: var(--accent-fg);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
