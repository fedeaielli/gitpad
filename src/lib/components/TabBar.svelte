<script lang="ts">
  import { tabs, activateTab, closeTab } from '../stores/tabs.svelte'
  import { ui, cyclePreview } from '../stores/ui.svelte'

  const isMarkdown = $derived(
    tabs.tabs.find((t) => t.id === tabs.activeTabId)?.language === 'markdown',
  )
</script>

<div class="tabbar">
  <button
    class="toggle"
    aria-label="Toggle sidebar"
    title="Toggle sidebar"
    onclick={() => (ui.sidebarOpen = !ui.sidebarOpen)}>☰</button
  >
  <div class="tabs">
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
  </div>
  <div class="actions">
    {#if tabs.activeTabId}
      <button
        class="icon"
        class:active={ui.timelineOpen}
        title="History for this file"
        aria-label="File history"
        aria-pressed={ui.timelineOpen}
        onclick={() => (ui.timelineOpen = !ui.timelineOpen)}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="6" />
          <path d="M8 4.6v3.8l2.6 1.5" />
        </svg>
      </button>
      {#if isMarkdown}
        <button class="preview" onclick={cyclePreview} title="Cycle preview (Ctrl+Shift+P)">
          Preview · {ui.previewMode}
        </button>
      {/if}
    {/if}
  </div>
</div>

<style>
  .tabbar {
    display: flex;
    align-items: stretch;
    box-sizing: border-box;
    flex: 0 0 35px;
    overflow: hidden;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
  }
  .tabs {
    display: flex;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
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
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 0 0 auto;
    margin-left: auto;
    padding: 0 8px;
    border-left: 1px solid var(--border);
  }
  .icon {
    display: flex;
    padding: 3px;
    border-radius: 3px;
    color: var(--text);
  }
  .icon:hover {
    background: var(--bg-raised);
  }
  .icon.active {
    color: var(--accent);
  }
  .icon svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
  }
  .preview {
    padding: 1px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .preview:hover {
    color: var(--text);
    border-color: var(--accent);
  }
</style>
