<script lang="ts">
  import { tabs, activateTab, closeTab } from '../stores/tabs.svelte'
  import { ui, cyclePreview } from '../stores/ui.svelte'
  import FileIcon from './FileIcon.svelte'
  import { tip } from '../utils/tooltip'

  const isMarkdown = $derived(
    tabs.tabs.find((t) => t.id === tabs.activeTabId)?.language === 'markdown',
  )
</script>

<div class="tabbar">
  <div class="tabs" role="tablist" aria-label="Open files">
    {#each tabs.tabs as tab (tab.id)}
      <div
        class="tab"
        role="presentation"
        class:active={tab.id === tabs.activeTabId}
        onauxclick={(e) => {
          if (e.button === 1) closeTab(tab.id)
        }}
      >
        <button
          class="label"
          role="tab"
          aria-selected={tab.id === tabs.activeTabId}
          onclick={() => activateTab(tab.id)}
        >
          {#if tab.modified}<span class="dot" role="img" aria-label="modified"></span>{/if}
          <FileIcon name={tab.path} />
          <span class="name">{tab.path}</span>
        </button>
        <button class="close" aria-label="Close {tab.path}" onclick={() => closeTab(tab.id)}>×</button>
      </div>
    {/each}
  </div>
  {#if tabs.tabs.length > 0}
    <div class="actions">
      {#if tabs.activeTabId}
        <button
          class="icon"
          class:active={ui.timelineOpen}
          use:tip={'History for this file'}
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
          <button class="preview" onclick={cyclePreview} use:tip={'Cycle preview (Ctrl+Shift+P)'}>
            Preview · {ui.previewMode}
          </button>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tabbar {
    display: flex;
    align-items: stretch;
    box-sizing: border-box;
    flex: 0 0 var(--h-tabbar);
    overflow: hidden;
    padding: 5px 6px;
    border-bottom: 1px solid var(--divider);
    border-radius: var(--r-lg) var(--r-lg) 0 0;
    background: var(--surface-1);
  }
  .tabs {
    display: flex;
    flex: 1;
    min-width: 0;
    gap: 2px;
    overflow-x: auto;
    overflow-y: hidden;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    max-width: 200px;
    border-radius: var(--r-md);
    color: var(--muted);
  }
  /* Hover is a lifted grey, never the surface the open tab shows. */
  .tab:not(.active):hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .tab.active {
    background: var(--surface-3);
    box-shadow: inset 0 2px 0 var(--accent);
    color: var(--text);
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
    /* 22px square: the shared 9px radius would read as a circle here. */
    border-radius: var(--r-sm);
    color: var(--text);
  }
  .icon:hover {
    background: var(--surface-2);
  }
  .icon.active {
    color: var(--accent-fg);
  }
  .icon svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
  }
  /* A toggle next to the tab icons: same height, same border, no accent. */
  .preview {
    display: flex;
    align-items: center;
    box-sizing: border-box;
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-0);
    color: var(--muted);
    font-size: var(--fs-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .preview:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
</style>
