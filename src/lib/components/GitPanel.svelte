<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { gitStatus, applyGitStatus, syncNow, type GitStatus } from '../stores/git.svelte'
  import { ui } from '../stores/ui.svelte'

  let commitMessage = $state('')
  let cloneUrl = $state('')
  let cloning = $state(false)
  let showClone = $state(false)

  async function commit() {
    if (!commitMessage.trim()) return
    if (await syncNow(commitMessage.trim())) commitMessage = ''
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
      gitStatus.error = String(e)
    } finally {
      cloning = false
    }
  }
</script>

<div class="panel">
  <div class="row">
    <button class="primary" disabled={gitStatus.syncing} onclick={() => syncNow(null)}>
      {gitStatus.syncing ? 'Syncing…' : 'Sync'}
    </button>
    <button onclick={() => (showClone = !showClone)}>Clone</button>
  </div>

  {#if showClone}
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
</style>
