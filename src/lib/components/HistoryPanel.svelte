<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from '@tauri-apps/plugin-dialog'
  import {
    applyGitStatus,
    errorMessage,
    gitStatus,
    type FileChange,
    type GitStatus,
  } from '../stores/git.svelte'
  import { ui } from '../stores/ui.svelte'
  import { closeIfClean, tabs } from '../stores/tabs.svelte'

  interface CommitEntry {
    hash: string
    shortHash: string
    message: string
    author: string
    timestamp: number
    filesChanged: number
  }

  let commits = $state<CommitEntry[]>([])
  let expanded = $state('')
  let files = $state<FileChange[]>([])
  let busy = $state(false)
  let error = $state<string | null>(null)

  async function load() {
    if (!gitStatus.configured) return
    commits = await invoke<CommitEntry[]>('git_log').catch(() => [])
  }

  // Reload when the view opens and again whenever a sync finishes.
  $effect(() => {
    if (ui.activeView === 'history' && !gitStatus.syncing) void load()
  })

  async function toggle(commit: CommitEntry) {
    if (expanded === commit.hash) {
      expanded = ''
      files = []
      return
    }
    expanded = commit.hash
    files = await invoke<FileChange[]>('git_commit_files', { hash: commit.hash }).catch(() => [])
  }

  async function restore(commit: CommitEntry, file: FileChange) {
    const open = tabs.tabs.find((t) => t.path === file.path)
    if (open?.modified) {
      error = 'save or close the open file first'
      return
    }
    if (!(await ask(`Restore ${file.path} from ${commit.shortHash}?`, { title: 'Restore file' }))) {
      return
    }
    busy = true
    error = null
    try {
      closeIfClean(file.path)
      applyGitStatus(
        await invoke<GitStatus>('git_restore_file', { hash: commit.hash, path: file.path }),
      )
      ui.treeVersion += 1
      await load()
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  async function revert(commit: CommitEntry) {
    if (
      !(await ask(`Revert ${commit.shortHash}: ${commit.message}?`, {
        title: 'Revert commit',
        kind: 'warning',
      }))
    ) {
      return
    }
    busy = true
    error = null
    try {
      const touched = await invoke<FileChange[]>('git_commit_files', { hash: commit.hash })
      applyGitStatus(await invoke<GitStatus>('git_revert_commit', { hash: commit.hash }))
      for (const file of touched) closeIfClean(file.path)
      expanded = ''
      files = []
      ui.treeVersion += 1
      await load()
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  function day(timestamp: number) {
    return new Date(timestamp * 1000).toLocaleDateString()
  }
</script>

{#if !gitStatus.configured}
  <p class="hint">Clone a repository to see its history.</p>
{:else}
  <ul class="commits">
    {#each commits as commit (commit.hash)}
      <li class="commit" class:open={expanded === commit.hash}>
        <button class="row" onclick={() => toggle(commit)}>
          <code>{commit.shortHash}</code>
          <span class="msg">{commit.message}</span>
          <span class="meta">
            {day(commit.timestamp)} · {commit.author} · {commit.filesChanged}
          </span>
        </button>
        {#if expanded === commit.hash}
          <div class="actions">
            <button disabled={busy} onclick={() => revert(commit)}>Revert commit</button>
          </div>
          <ul class="files">
            {#each files as file (file.path)}
              <li>
                <span class="badge {file.status}">{file.status}</span>
                <span class="path">{file.path}</span>
                <button
                  disabled={busy || file.status === 'D'}
                  title={file.status === 'D'
                    ? `${file.path} does not exist in this commit`
                    : `Restore ${file.path}`}
                  onclick={() => restore(commit, file)}
                >
                  Restore
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </li>
    {/each}
  </ul>
{/if}
{#if error}
  <p class="error">{error}</p>
{/if}

<style>
  .hint,
  .error {
    margin: 0;
    padding: 4px 8px;
    color: var(--muted);
    font-size: 11px;
    overflow-wrap: anywhere;
  }
  .error {
    color: var(--err);
  }
  .commits,
  .files {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .commit.open {
    background: var(--bg-raised);
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 4px 8px;
    border-color: transparent;
    background: transparent;
    text-align: left;
  }
  .row code {
    color: var(--accent);
    font-size: 11px;
  }
  .msg {
    overflow: hidden;
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--muted);
    font-size: 10px;
  }
  .actions {
    padding: 0 8px 4px;
  }
  .files li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 1px 8px 1px 18px;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  .files button {
    padding: 0 5px;
    font-size: 10px;
  }
</style>
