<script lang="ts">
  import FileTree from './FileTree.svelte'
  import GitPanel from './GitPanel.svelte'

  let filesOpen = $state(true)
  let gitOpen = $state(true)
</script>

<aside class="sidebar">
  <section class="files">
    <button class="section" onclick={() => (filesOpen = !filesOpen)}>
      <span class="chevron">{filesOpen ? '▾' : '▸'}</span> Files
    </button>
    {#if filesOpen}
      <div class="scroll">
        <FileTree />
      </div>
    {/if}
  </section>

  <section class="git">
    <button class="section" onclick={() => (gitOpen = !gitOpen)}>
      <span class="chevron">{gitOpen ? '▾' : '▸'}</span> Git
    </button>
    {#if gitOpen}
      <GitPanel />
    {/if}
  </section>
</aside>

<style>
  .sidebar {
    display: flex;
    flex: 0 0 260px;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
  }
  section {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .files {
    flex: 1;
  }
  .git {
    border-top: 1px solid var(--border);
  }
  .section {
    padding: 6px 8px;
    text-align: left;
    color: var(--muted);
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: 0.06em;
  }
  .section:hover {
    color: var(--text);
  }
  .chevron {
    display: inline-block;
    width: 10px;
  }
  .scroll {
    flex: 1;
    overflow: auto;
  }
</style>
