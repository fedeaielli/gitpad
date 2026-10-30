<script lang="ts">
  import { tabs, activateTab, closeTab } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
</script>

<div class="tabbar">
  <button
    class="toggle"
    aria-label="Toggle sidebar"
    title="Toggle sidebar"
    onclick={() => (ui.sidebarOpen = !ui.sidebarOpen)}>☰</button
  >
  {#each tabs.tabs as tab (tab.id)}
    <div
      class="tab"
      class:active={tab.id === tabs.activeTabId}
      onauxclick={(e) => {
        if (e.button === 1) closeTab(tab.id)
      }}
    >
      <button class="label" onclick={() => activateTab(tab.id)}>
        {#if tab.modified}<span class="dot" aria-label="modified"></span>{/if}
        <span class="name">{tab.path}</span>
      </button>
      <button class="close" aria-label="Close {tab.path}" onclick={() => closeTab(tab.id)}>×</button>
    </div>
  {/each}
  <button
    class="search"
    aria-label="Search files"
    title="Search files (Ctrl+P)"
    onclick={() => (ui.commandPaletteOpen = true)}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="7" cy="7" r="4.2" />
      <path d="M10.2 10.2 13.5 13.5" />
    </svg>
    <span class="hint">Search files (Ctrl+P)</span>
  </button>
</div>

<style>
  .tabbar {
    display: flex;
    flex: 0 0 34px;
    overflow-x: auto;
    overflow-y: hidden;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
  }
  .tab {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    max-width: 240px;
    border-right: 1px solid var(--border);
  }
  .toggle {
    flex: 0 0 auto;
    padding: 0 10px;
    color: var(--muted);
    border-right: 1px solid var(--border);
  }
  .toggle:hover {
    color: var(--text);
  }
  .tab.active {
    background: var(--bg);
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .label {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 0 4px 0 10px;
    height: 100%;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    flex: 0 0 auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--warn);
  }
  .close {
    padding: 0 8px;
    color: var(--muted);
  }
  .close:hover {
    color: var(--text);
  }
  .search {
    position: sticky;
    right: 0;
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    padding: 0 10px;
    border-left: 1px solid var(--border);
    background: var(--bg-panel);
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
  }
  .search:hover {
    color: var(--text);
  }
  .search svg {
    width: 13px;
    height: 13px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
  }
  @media (max-width: 640px) {
    .hint {
      display: none;
    }
  }
</style>
