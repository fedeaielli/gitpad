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
  import { tip } from '../utils/tooltip'

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

  /// An open file narrows the log to its commits; no tab means the whole repo.
  const active = $derived(tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null)
  const filePath = $derived(active?.path ?? '')

  async function load(target = filePath) {
    if (!gitStatus.configured) return
    commits = target
      ? await invoke<CommitEntry[]>('git_file_log', { path: target }).catch(() => [])
      : await invoke<CommitEntry[]>('git_log').catch(() => [])
  }

  // Reload when the view opens, on another tab, and whenever a sync finishes.
  $effect(() => {
    if (ui.activeView === 'history' && !gitStatus.syncing) void load(filePath)
  })

  /// Clicking a commit shows its own diff in the bottom panel, like Ctrl+H does
  /// for the file's history.
  function open(commit: CommitEntry) {
    ui.timelineCommit = {
      hash: commit.hash,
      shortHash: commit.shortHash,
      message: commit.message,
    }
    ui.timelineOpen = true
  }

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
  {#if filePath}
    <p class="filter">
      History for <code>{filePath.split('/').pop()}</code>
    </p>
  {/if}
  {#if commits.length === 0}
    <p class="hint">{filePath ? 'No commits touch this file yet.' : 'No commits yet.'}</p>
  {/if}
  <ul class="commits">
    {#each commits as commit (commit.hash)}
      <li
        class="commit"
        class:open={expanded === commit.hash}
        class:active={ui.timelineCommit?.hash === commit.hash}
      >
        <div class="head-row">
          <button class="row" onclick={() => open(commit)}>
            <code>{commit.shortHash}</code>
            <span class="msg">{commit.message}</span>
            <span class="meta">
              {day(commit.timestamp)} · {commit.author} · {commit.filesChanged}
            </span>
          </button>
          <button
            class="expand"
            class:open={expanded === commit.hash}
            aria-expanded={expanded === commit.hash}
            aria-label="Show files"
            onclick={() => toggle(commit)}
          >
            ›
          </button>
        </div>
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
                  title={file.status === 'D' ? `${file.path} does not exist in this commit` : undefined}
                  use:tip={file.status === 'D'
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
  .filter {
    margin: 0;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .filter code {
    color: var(--accent-fg);
  }
  .commits,
  .files {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .commit.open {
    background: var(--surface-2);
  }
  .commit.active {
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .head-row {
    display: flex;
    align-items: center;
  }
  .row {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    padding: 4px 8px;
    border-color: transparent;
    background: transparent;
    text-align: left;
  }
  .expand {
    flex: 0 0 18px;
    width: 18px;
    height: 18px;
    margin-right: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
    transition: transform var(--t-fast);
  }
  .expand.open {
    transform: rotate(90deg);
  }
  .expand:hover {
    color: var(--text);
  }
  .row code {
    color: var(--accent-fg);
    font-size: 11px;
  }
  .msg {
    overflow: hidden;
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    overflow: hidden;
    color: var(--muted);
    font-size: 10px;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    border-radius: var(--r-pill);
    font-size: 10px;
    font-weight: 600;
  }
  .badge.M {
    color: var(--accent-fg);
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
