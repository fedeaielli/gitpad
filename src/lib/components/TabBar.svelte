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
    /* Visible, not hidden: the open tab's concave corners hang outside its box
       and must not be clipped by the strip. Horizontal scrolling is handled by
       .tabs itself. */
    overflow: visible;
    /* No bottom border: the line under the strip is the inactive tabs' own
       bottom border, and the open tab omits its slice. No bottom padding
       either, so the tabs hang flush onto the editor. */
    padding: 5px 6px 0;
    border-radius: var(--r-lg) var(--r-lg) 0 0;
    background: var(--surface-0);
  }
  .tabs {
    display: flex;
    flex: 1;
    min-width: 0;
    /* Adjacent, as in VS Code: the tabs' bottom borders have to join into one
       continuous line, so there is no gap between them. */
    gap: 0;
    overflow-x: auto;
    overflow-y: hidden;
    /* The open tab's concave corners sit 8px outside its box. This padding is
       the room they need at the two ends of the strip; the negative margin
       cancels it so the tabs keep their original inset. */
    padding-inline: 8px;
    margin-inline: -8px;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    box-sizing: border-box;
    max-width: 200px;
    /* Top and side borders stay transparent: the open tab gets real 1px edges,
       and without them its label would shift sideways on activation. */
    border: 1px solid transparent;
    border-bottom-color: var(--border);
    border-radius: var(--r-md) var(--r-md) 0 0;
    background: var(--surface-0);
    color: var(--muted);
  }
  /* Hover is a lifted grey, never the surface the open tab shows. */
  .tab:not(.active):hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .tab.active {
    /* Editor surface, no bottom border. Heights are stretched, so dropping the
       bottom border does not shorten the tab: its background simply paints the
       pixel the line used to occupy, which is the merge. No negative margin,
       because the scroll container clips anything that leaves the strip. */
    z-index: 1;
    border-color: var(--border);
    border-top-color: var(--accent);
    border-bottom: none;
    border-radius: 4px 4px 0 0;
    background: var(--editor-surface);
    color: var(--text);
  }
  /* Concave bottom corners: an 8px square with one rounded corner, filled by a
     hard offset shadow in the editor surface. The rounded corner is the curve
     the tab peels away with; the rest of the square covers the strip line
     beside the tab, so the two surfaces read as one. */
  .tab.active::before,
  .tab.active::after {
    content: '';
    position: absolute;
    bottom: 0;
    width: 8px;
    height: 8px;
    background: transparent;
  }
  .tab.active::before {
    left: -8px;
    border-bottom-right-radius: 6px;
    box-shadow: 3px 1px 0 1px var(--editor-surface);
  }
  .tab.active::after {
    right: -8px;
    border-bottom-left-radius: 6px;
    box-shadow: -3px 1px 0 1px var(--editor-surface);
  }
  .tab.active::before {
    left: -8px;
    border-bottom-right-radius: 6px;
    box-shadow: 3px 1px 0 1px var(--editor-surface);
  }
  .tab.active::after {
    right: -8px;
    border-bottom-left-radius: 6px;
    box-shadow: -3px 1px 0 1px var(--editor-surface);
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
