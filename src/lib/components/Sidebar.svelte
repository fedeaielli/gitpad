<script lang="ts">
  import FileTree from './FileTree.svelte'
  import GitPanel from './GitPanel.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { ui, cyclePreview } from '../stores/ui.svelte'

  const isMarkdown = $derived(
    tabs.tabs.find((t) => t.id === tabs.activeTabId)?.language === 'markdown',
  )

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
  <button
    class="search"
    title="Search files (Ctrl+P)"
    onclick={() => (ui.commandPaletteOpen = true)}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="7" cy="7" r="4.2" />
      <path d="M10.2 10.2 13.5 13.5" />
    </svg>
    <span>Search files</span>
    <kbd>Ctrl+P</kbd>
  </button>
  <div class="head">
    <span>{ui.activeView === 'files' ? 'Explorer' : 'Source control'}</span>
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
      <button class="preview" onclick={cyclePreview} disabled={!isMarkdown}>
        Preview: {ui.previewMode}
      </button>
    {/if}
  </div>
  <div class="scroll">
    {#if ui.activeView === 'files'}
      <FileTree />
    {:else}
      <GitPanel />
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
    background: var(--bg-panel);
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
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 6px 8px 2px;
    padding: 3px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-raised);
    color: var(--muted);
    font-size: 12px;
    text-align: left;
  }
  .search:hover {
    border-color: var(--accent);
    color: var(--text);
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
  .search span {
    flex: 1;
  }
  .search kbd {
    font: inherit;
    font-size: 10px;
    opacity: 0.7;
  }
  .head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
    /* Same 35px baseline as the tab bar so both bottom borders line up. */
    box-sizing: border-box;
    min-height: 35px;
    padding: 2px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .actions {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }
  .actions button {
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    line-height: 18px;
    padding: 0 3px;
  }
  .actions button:hover {
    color: var(--text);
    background: var(--bg-raised);
  }
  .actions button[aria-pressed='true'] {
    color: var(--accent);
  }
  .preview {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--muted);
    font-size: 10px;
  }
  .preview:hover:not(:disabled) {
    color: var(--text);
  }
  .preview:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
