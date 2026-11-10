<script lang="ts">
  import { tabs, activateTab, closeTab } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
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
          <FileIcon name={tab.path} />
          <span class="name">{tab.path}</span>
        </button>
        {#if tab.modified}
          <button class="dirty" aria-label="Close {tab.path}" onclick={() => closeTab(tab.id)}
            ><span class="dot" aria-hidden="true"></span></button
          >
        {:else}
          <button class="close" aria-label="Close {tab.path}" onclick={() => closeTab(tab.id)}
            >×</button
          >
        {/if}
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
          <div class="seg" role="group" aria-label="Markdown preview">
            <button
              class="icon"
              class:active={ui.previewMode === 'off'}
              aria-pressed={ui.previewMode === 'off'}
              aria-label="Editor only"
              use:tip={'Editor only'}
              onclick={() => (ui.previewMode = 'off')}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M11.9 2.6a1.7 1.7 0 0 1 2.4 2.4l-7.6 7.6-3.2.8.8-3.2 7.6-7.6z" />
              </svg>
            </button>
            <button
              class="icon"
              class:active={ui.previewMode === 'split'}
              aria-pressed={ui.previewMode === 'split'}
              aria-label="Split preview"
              use:tip={'Split preview'}
              onclick={() => (ui.previewMode = 'split')}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <rect x="2.5" y="2.5" width="11" height="11" rx="1.5" />
                <path d="M8 2.5v11" />
              </svg>
            </button>
            <button
              class="icon"
              class:active={ui.previewMode === 'full'}
              aria-pressed={ui.previewMode === 'full'}
              aria-label="Preview only"
              use:tip={'Preview only'}
              onclick={() => (ui.previewMode = 'full')}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M1.5 8S3.9 3.8 8 3.8 14.5 8 14.5 8 12.1 12.2 8 12.2 1.5 8 1.5 8Z" />
                <circle cx="8" cy="8" r="1.7" />
              </svg>
            </button>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tabbar {
    position: relative;
    display: flex;
    align-items: stretch;
    box-sizing: border-box;
    flex: 0 0 var(--h-tabbar);
    /* Visible, not hidden: the open tab's concave corners hang outside its box
       and must not be clipped by the strip. Horizontal scrolling is handled by
       .tabs itself. */
    overflow: visible;
    /* No bottom border: the line under the strip is its own ::after, and the
       open tab paints over its slice. No bottom padding either, so the tabs
       hang flush onto the editor. */
    padding: 4px var(--r-md) 0;
    border-radius: var(--r-lg) var(--r-lg) 0 0;
    background: var(--tabstrip-bg);
  }
  /* The line the open tab cuts through. It runs the full width of the strip,
     under every inactive tab, and only the open one erases its stretch of it. */
  .tabbar::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 1px;
    background: var(--border);
    pointer-events: none;
  }
  .tabs {
    position: relative;
    z-index: 1;
    display: flex;
    flex: 1;
    min-width: 0;
    /* Adjacent, as in VS Code: the tabs have to join into one continuous line,
       so there is no gap between them. */
    gap: 0;
    overflow-x: auto;
    overflow-y: hidden;
    /* The open tab's concave corners sit one radius outside its box. This
       padding is the room they need at the two ends of the strip; the negative
       margin cancels it so the tabs keep their original inset. */
    padding-inline: var(--r-md);
    margin-inline: calc(-1 * var(--r-md));
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    /* An equal share of the strip, not the width of its label: with a content
       sizing the unsaved dot sat a whole long path away from the neighbours. */
    flex: 1 1 0;
    min-width: 96px;
    box-sizing: border-box;
    max-width: 200px;
    /* 1px short of the strip floor, so the line stays visible under the tab.
       The open tab drops the margin and reaches the editor instead. */
    margin-bottom: 1px;
    /* Top and side borders stay transparent: the open tab paints over them, and
       without the reserved 1px its label would shift on activation. */
    border: 1px solid transparent;
    border-radius: var(--r-md);
    color: var(--muted);
  }
  /* Hover is a lifted grey, never the surface the open tab shows. */
  .tab:not(.active):hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .tab.active {
    /* Editor surface, no bottom border and no bottom margin: the tab body
       reaches the strip floor and paints over the 1px line, which is the merge.
       No negative margin, because the scroll container clips what leaves the
       strip. */
    z-index: 1;
    margin-bottom: 0;
    border-color: var(--border);
    border-bottom: none;
    border-radius: var(--r-md) var(--r-md) 0 0;
    background: var(--editor-surface);
    color: var(--text);
  }
  /* Concave bottom corners, the same construction VS Code uses: a square one
     radius wide, with one rounded corner and a hard offset shadow in the editor
     surface. The curve is the shoulder the tab peels away with; the shadow
     fills the rest so the two surfaces read as one. */
  .tab.active::before,
  .tab.active::after {
    content: '';
    position: absolute;
    bottom: 0;
    width: var(--r-md);
    height: var(--r-md);
    box-sizing: border-box;
    border-bottom: 1px solid var(--border);
    background: transparent;
    clip-path: inset(0);
  }
  .tab.active::before {
    right: 100%;
    border-right: 1px solid var(--border);
    border-bottom-right-radius: var(--r-md);
    box-shadow: calc(var(--r-md) / 2) calc(var(--r-md) / 2) 0 calc(var(--r-md) / 2) var(--editor-surface);
  }
  .tab.active::after {
    left: 100%;
    border-left: 1px solid var(--border);
    border-bottom-left-radius: var(--r-md);
    box-shadow: calc(-1 * var(--r-md) / 2) calc(var(--r-md) / 2) 0 calc(var(--r-md) / 2) var(--editor-surface);
  }
  .label {
    display: flex;
    align-items: center;
    /* Owns the empty stretch of the tab: the whole surface opens the file, and
       the close control is pushed to the tab's right edge. */
    flex: 1;
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
  /* The unsaved dot takes the close button's place, as in VS Code: one slot,
     one meaning. */
  .dirty {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    padding: 0;
    border-radius: var(--r-sm);
  }
  .dirty:hover {
    background: var(--surface-2);
  }
  .dirty:hover .dot {
    display: none;
  }
  .dirty:hover::before {
    content: '×';
    color: var(--text);
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
    gap: 6px;
    flex: 0 0 auto;
    margin-left: auto;
    padding: 0 6px;
  }
  /* A short divider, not a full-height border: the strip already has its own
     line along the floor and the two would meet in a T. */
  .actions::before {
    align-self: center;
    width: 1px;
    height: 16px;
    background: var(--border);
    content: '';
  }
  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    width: 24px;
    height: 24px;
    padding: 0;
    border-radius: var(--r-sm);
    color: var(--muted);
  }
  .icon:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  /* Fill and colour together: colour alone is not a state. */
  .icon.active {
    background: var(--surface-2);
    color: var(--accent-fg);
  }
  .icon svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  /* One outline, three segments: the editor is one place, not three buttons. */
  .seg {
    display: flex;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
  }
  .seg .icon {
    width: 22px;
    height: 22px;
    border-radius: 0;
  }
</style>
