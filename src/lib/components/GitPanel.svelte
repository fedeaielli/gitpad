<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from '@tauri-apps/plugin-dialog'
  import {
    errorMessage,
    gitStatus,
    applyGitStatus,
    syncNow,
    pullOnly,
    pushOnly,
    type FileChange,
    type GitStatus,
  } from '../stores/git.svelte'
  import { ui } from '../stores/ui.svelte'
  import { tabs, openTab, activateTab, discardUnder } from '../stores/tabs.svelte'
  import { languageFor } from '../utils/langmap'

  let commitMessage = $state('')
  let cloneUrl = $state('')
  let cloning = $state(false)
  let showClone = $state(false)

  async function commit() {
    if (!commitMessage.trim()) return
    if (await syncNow(commitMessage.trim())) commitMessage = ''
  }

  async function openChange(change: FileChange) {
    if (change.status === 'D') return
    const existing = tabs.tabs.find((t) => t.path === change.path)
    if (existing) {
      activateTab(existing.id)
      return
    }
    try {
      const content = await invoke<string>('fs_read_file', { path: change.path })
      openTab({
        id: change.path,
        path: change.path,
        content,
        modified: false,
        language: languageFor(change.path),
      })
    } catch (e) {
      gitStatus.error = errorMessage(e)
    }
  }

  async function discard(change: FileChange) {
    const ok = await ask(`Discard changes to ${change.path}?`, {
      title: 'Discard changes',
      kind: 'warning',
    })
    if (!ok) return
    try {
      applyGitStatus(await invoke<GitStatus>('git_discard_file', { path: change.path }))
      discardUnder(change.path)
    } catch (e) {
      gitStatus.error = errorMessage(e)
    }
  }

  async function clone() {
    if (!cloneUrl.trim()) return
    cloning = true
    gitStatus.error = null
    try {
      await invoke('git_clone', { url: cloneUrl.trim() })
      ui.treeVersion += 1
      cloneUrl = ''
      showClone = false
      applyGitStatus(await invoke<GitStatus>('git_status'))
    } catch (e) {
      gitStatus.error = errorMessage(e)
    } finally {
      cloning = false
    }
  }
</script>

<div class="panel">
  {#if gitStatus.configured}
    <p class="branch" title="Current branch from git_status">⎇ {gitStatus.branch || '—'}</p>
  {/if}

  <div class="row">
    <button
      disabled={gitStatus.syncing || gitStatus.conflicts.length > 0}
      title={gitStatus.conflicts.length > 0 ? 'Resolve the conflicts first' : undefined}
      onclick={() => pullOnly()}
    >
      Pull
    </button>
    <button
      disabled={gitStatus.syncing || gitStatus.conflicts.length > 0}
      title={gitStatus.conflicts.length > 0 ? 'Resolve the conflicts first' : undefined}
      onclick={() => pushOnly()}
    >
      Push
    </button>
    <button
      class="primary"
      disabled={gitStatus.syncing || gitStatus.conflicts.length > 0}
      title={gitStatus.conflicts.length > 0 ? 'Resolve the conflicts first' : undefined}
      onclick={() => syncNow(null)}
    >
      {gitStatus.syncing ? 'Syncing…' : 'Sync'}
    </button>
    <button onclick={() => (showClone = !showClone)}>Clone</button>
  </div>

  {#if showClone && !gitStatus.configured}
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault()
        void clone()
      }}
    >
      <input bind:value={cloneUrl} placeholder="https://github.com/user/repo.git" />
      <button type="submit" disabled={cloning}>OK</button>
    </form>
  {/if}

  <form
    class="row"
    onsubmit={(e) => {
      e.preventDefault()
      void commit()
    }}
  >
    <input bind:value={commitMessage} placeholder="Commit message" />
    <button type="submit" disabled={gitStatus.syncing || gitStatus.conflicts.length > 0}>
      Commit
    </button>
  </form>

  {#if gitStatus.changes.length > 0}
    <ul class="changes">
      {#each gitStatus.changes as change (change.path)}
        <li>
          <button
            class="change"
            disabled={change.status === 'D'}
            title={change.status === 'D' ? `${change.path} (deleted)` : change.path}
            onclick={() => openChange(change)}
          >
            <span class="badge {change.status}">{change.status}</span>
            <span class="path">{change.path}</span>
          </button>
          <button class="discard" title="Discard changes" onclick={() => discard(change)}>×</button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if gitStatus.conflicts.length > 0}
    <p class="conflicts">⚠ {gitStatus.conflicts.length} conflict(s)</p>
  {/if}
  {#if gitStatus.error}
    <p class="error">{gitStatus.error}</p>
  {/if}

  <p class="meta">
    {gitStatus.modifiedFiles.length} changed · {gitStatus.ahead} ahead · {gitStatus.behind} behind
  </p>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 8px 8px;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .row input {
    flex: 1;
    min-width: 0;
  }
  button {
    padding: 3px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-raised);
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary {
    color: var(--ok);
  }
  .conflicts {
    margin: 0;
    color: var(--err);
    font-size: 11px;
  }
  .error {
    margin: 0;
    color: var(--err);
    font-size: 11px;
    overflow-wrap: anywhere;
  }
  .meta {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
  }
  .branch {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .changes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .changes li {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .change {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    text-align: left;
    border-color: transparent;
    background: transparent;
    padding: 2px 4px;
  }
  .change:disabled {
    opacity: 1;
    color: var(--muted);
    text-decoration: line-through;
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }
  .badge {
    flex: 0 0 auto;
    width: 12px;
    font-size: 10px;
    font-weight: 600;
  }
  .badge.M {
    color: var(--accent);
  }
  .badge.A {
    color: var(--ok);
  }
  .badge.D {
    color: var(--err);
  }
  .discard {
    border-color: transparent;
    background: transparent;
    color: var(--muted);
    padding: 0 5px;
  }
  .discard:hover:not(:disabled) {
    color: var(--err);
    border-color: var(--err);
  }
</style>
