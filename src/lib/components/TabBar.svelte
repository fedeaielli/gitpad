<script lang="ts">
  import { tabs, activateTab, closeTab } from '../stores/tabs.svelte'
</script>

<div class="tabbar">
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
</style>
