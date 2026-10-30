<script lang="ts">
  import FileTree from './FileTree.svelte'
  import GitPanel from './GitPanel.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { ui, cyclePreview } from '../stores/ui.svelte'

  const isMarkdown = $derived(
    tabs.tabs.find((t) => t.id === tabs.activeTabId)?.language === 'markdown',
  )
</script>

<aside class="panel">
  <div class="head">
    <span>{ui.activeView === 'files' ? 'Explorer' : 'Source control'}</span>
    {#if ui.activeView === 'files'}
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
    display: flex;
    flex: 0 0 260px;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
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
