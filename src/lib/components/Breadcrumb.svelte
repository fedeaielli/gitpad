<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tabs } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
  import FileIcon from './FileIcon.svelte'

  const active = $derived(tabs.tabs.find((t) => t.id === tabs.activeTabId))
  const parts = $derived((active?.path ?? '').split('/').filter(Boolean))

  let root = $state('')

  // Retried after a clone/import, so a fresh install fills it in without a restart.
  $effect(() => {
    ui.treeVersion
    if (root) return
    void invoke<string>('fs_repo_name')
      .then((name) => (root = name))
      .catch(() => {})
  })
</script>

{#if active}
  <nav class="crumbs" aria-label="File path">
    {#if root}
      <span class="part">{root}</span>
      {#if parts.length > 0}<span class="sep" aria-hidden="true">›</span>{/if}
    {/if}
    {#each parts as part, i (i)}
      {#if i > 0}<span class="sep" aria-hidden="true">›</span>{/if}
      {#if i === parts.length - 1}
        <span class="part last"><FileIcon name={part} />{part}</span>
      {:else}
        <span class="part">{part}</span>
      {/if}
    {/each}
  </nav>
{/if}

<style>
  .crumbs {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    padding: 3px 10px;
    background: var(--surface-0);
    color: var(--muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
    user-select: none;
  }
  .part {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .last {
    color: var(--text);
  }
  .sep {
    opacity: 0.7;
  }
</style>
