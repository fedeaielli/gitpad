<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import {
    applyGitStatus,
    errorMessage,
    gitStatus,
    type FileChange,
    type GitStatus,
  } from '../stores/git.svelte'
  import { ui } from '../stores/ui.svelte'
  import { confirmDialog } from '../stores/confirm.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { reloadTabFromDisk } from '../editor/bridge'
  import { tip } from '../utils/tooltip'

  interface CommitEntry {
    hash: string
    shortHash: string
    message: string
    author: string
    timestamp: number
    filesChanged: number
  }

  interface CommitGroup {
    key: string
    label: string
    commits: CommitEntry[]
  }

  let commits = $state<CommitEntry[]>([])
  let expanded = $state('')
  let files = $state<FileChange[]>([])
  let busy = $state(false)
  let error = $state<string | null>(null)
  /// Ignore a response that lost the race against a newer tab or expansion.
  let loadSeq = 0

  /// An open file narrows the log to its commits; no tab means the whole repo.
  const active = $derived(tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null)
  const filePath = $derived(active?.path ?? '')
  /// A folder delete is one commit full of `D` entries: reverting it is a restore.
  const allDeleted = $derived(files.length > 0 && files.every((f) => f.status === 'D'))
  const groups = $derived(groupByDay(commits))

  async function load(target = filePath) {
    if (!gitStatus.configured) return
    const seq = ++loadSeq
    const result = target
      ? await invoke<CommitEntry[]>('git_file_log', { path: target }).catch(() => [])
      : await invoke<CommitEntry[]>('git_log').catch(() => [])
    if (seq === loadSeq) commits = result
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
    const hash = commit.hash
    expanded = hash
    files = []
    const result = await invoke<FileChange[]>('git_commit_files', { hash }).catch(() => [])
    if (expanded === hash) files = result
  }

  async function restore(commit: CommitEntry, file: FileChange) {
    const open = tabs.tabs.find((t) => t.path === file.path)
    if (open?.modified) {
      error = 'save or close the open file first'
      return
    }
    const confirmed = await confirmDialog({
      title: 'Restore file',
      message: `Restore ${file.path} from ${commit.shortHash}?`,
      confirmLabel: 'Restore',
    })
    if (!confirmed) return
    busy = true
    error = null
    try {
      applyGitStatus(
        await invoke<GitStatus>('git_restore_file', { hash: commit.hash, path: file.path }),
      )
      // The tab stays open: EditorPane re-reads the file into its model.
      await reloadTabFromDisk(file.path)
      ui.treeVersion += 1
      await load()
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  async function revert(commit: CommitEntry) {
    const restore = allDeleted && expanded === commit.hash
    const question = restore
      ? `Restore ${files.length} deleted item${files.length === 1 ? '' : 's'} from ${commit.shortHash}?`
      : `Revert ${commit.shortHash}: ${commit.message}?`
    const confirmed = await confirmDialog({
      title: restore ? 'Restore deleted items' : 'Revert commit',
      message: question,
      kind: 'warning',
      confirmLabel: restore ? 'Restore' : 'Revert',
    })
    if (!confirmed) return
    busy = true
    error = null
    try {
      const touched = await invoke<FileChange[]>('git_commit_files', { hash: commit.hash })
      applyGitStatus(await invoke<GitStatus>('git_revert_commit', { hash: commit.hash }))
      for (const file of touched) await reloadTabFromDisk(file.path)
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

  function dayLabel(timestamp: number) {
    const date = new Date(timestamp * 1000)
    const today = new Date()
    const yesterday = new Date()
    yesterday.setDate(today.getDate() - 1)
    const day = date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    })
    const sameDay = (a: Date, b: Date) =>
      a.getFullYear() === b.getFullYear() &&
      a.getMonth() === b.getMonth() &&
      a.getDate() === b.getDate()
    if (sameDay(date, today)) return `Today · ${day}`
    if (sameDay(date, yesterday)) return `Yesterday · ${day}`
    return day
  }

  function timeLabel(timestamp: number) {
    return new Date(timestamp * 1000).toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  function fileCount(count: number) {
    return `${count} file${count === 1 ? '' : 's'}`
  }

  type CommitKind = 'delete' | 'revert' | 'import' | 'sync' | 'update' | 'other'

  function kindOf(message: string): CommitKind {
    if (message.startsWith('gitpad: delete ')) return 'delete'
    if (message.startsWith('gitpad: revert ')) return 'revert'
    if (message.startsWith('gitpad: import ')) return 'import'
    if (message.startsWith('gitpad: sync ')) return 'sync'
    if (message.startsWith('gitpad: update ')) return 'update'
    return 'other'
  }

  /// Consecutive commits share a day header; the log is already newest first.
  function groupByDay(entries: CommitEntry[]): CommitGroup[] {
    const groups: CommitGroup[] = []
    for (const commit of entries) {
      const key = new Date(commit.timestamp * 1000).toDateString()
      const last = groups[groups.length - 1]
      if (last?.key === key) last.commits.push(commit)
      else groups.push({ key, label: dayLabel(commit.timestamp), commits: [commit] })
    }
    return groups
  }
</script>

{#if !gitStatus.configured}
  <p class="empty">Open or clone a repository to see its history.</p>
{:else}
  {#if filePath}
    <p class="filter">
      History for <code>{filePath.split('/').pop()}</code>
    </p>
  {/if}
  {#if commits.length === 0}
    <p class="empty">
      {filePath ? 'No commits touch this file yet.' : 'No commits yet.'}
    </p>
  {/if}
  <ul class="commits">
    {#each groups as group (group.key)}
      <li class="day">{group.label}</li>
      {#each group.commits as commit (commit.hash)}
        <li
          class="commit"
          class:open={expanded === commit.hash}
          class:active={ui.timelineCommit?.hash === commit.hash}
        >
          <div class="head-row">
            <button class="row" onclick={() => open(commit)}>
              <span class="id">
                <code>{commit.shortHash}</code>
                <span class="meta">
                  {timeLabel(commit.timestamp)} · {commit.author} · {fileCount(commit.filesChanged)}
                </span>
              </span>
              <span class="msg {kindOf(commit.message)}">{commit.message}</span>
            </button>
            <button
              class="expand"
              class:open={expanded === commit.hash}
              aria-expanded={expanded === commit.hash}
              aria-label={expanded === commit.hash ? 'Hide files' : 'Show files'}
              onclick={() => toggle(commit)}
            >
              <span class="chev">
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M6 3.5 10.5 8 6 12.5" />
                </svg>
              </span>
            </button>
          </div>
          {#if expanded === commit.hash}
            <div class="actions">
              <button disabled={busy} onclick={() => revert(commit)}>
                {allDeleted
                  ? `Restore ${files.length} deleted item${files.length === 1 ? '' : 's'}`
                  : 'Revert commit'}
              </button>
            </div>
            <ul class="files">
              {#each files as file (file.path)}
                <li>
                  <span class="badge {file.status}">{file.status}</span>
                  <span class="path">{file.path}</span>
                  {#if file.status !== 'D'}
                    <button
                      disabled={busy}
                      use:tip={`Restore ${file.path}`}
                      onclick={() => restore(commit, file)}
                    >
                      Restore
                    </button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    {/each}
  </ul>
{/if}
{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<style>
  .error {
    margin: 0;
    padding: 4px 8px;
    color: var(--err);
    font-size: var(--fs-xs);
    overflow-wrap: anywhere;
  }
  .empty {
    margin: 0;
    padding: 24px 12px;
    color: var(--muted);
    font-size: var(--fs-xs);
    text-align: center;
    overflow-wrap: anywhere;
  }
  .filter {
    margin: 0;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: var(--fs-2xs);
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
  .commits {
    padding: 2px 0 8px;
  }
  .commit {
    margin: 5px 8px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-2);
    overflow: hidden;
  }
  .commit:hover,
  .commit.open {
    border-color: var(--border-strong);
  }
  .commit.active {
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .day {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 7px 8px 4px;
    background: var(--surface-1);
    color: var(--muted);
    font-size: var(--fs-2xs);
    font-weight: 600;
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
    padding: 6px 8px;
    border-color: transparent;
    background: transparent;
    text-align: left;
  }
  .row:hover {
    background: var(--surface-3);
  }
  .expand {
    display: grid;
    flex: 0 0 26px;
    place-items: center;
    width: 26px;
    height: auto;
    align-self: stretch;
    margin: 0;
    padding: 0;
    border: 0;
    border-left: 1px solid var(--border);
    border-radius: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-md);
    line-height: 1;
    transition:
      background var(--t-fast),
      color var(--t-fast);
  }
  .expand:hover,
  .expand.open {
    background: var(--surface-3);
    color: var(--text);
  }
  .chev {
    display: inline-flex;
    transition: transform var(--t-fast);
  }
  .chev svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .expand.open .chev {
    transform: rotate(90deg);
  }
  .row code {
    color: var(--muted);
    font-size: var(--fs-2xs);
  }
  /* Identity on one line, the message gets the other to itself. */
  .id {
    display: flex;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
  }
  .msg {
    overflow: hidden;
    color: var(--text);
    font-size: var(--fs-sm);
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .msg.delete {
    color: var(--err);
  }
  .msg.revert {
    color: var(--warn);
  }
  .msg.import {
    color: var(--ok);
  }
  .meta {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: var(--fs-2xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    padding: 0 8px 4px;
  }
  .actions button {
    min-height: 32px;
    padding: 0 12px;
    border: 1px solid var(--accent);
    background: transparent;
    color: var(--accent-fg);
    font-size: var(--fs-xs);
    font-weight: 600;
  }
  .actions button:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .actions button:disabled,
  .files button:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .files {
    padding-bottom: 4px;
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
    font-size: var(--fs-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: 0 0 auto;
    width: 12px;
    border-radius: var(--r-pill);
    font-size: var(--fs-2xs);
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
    /* Desktop target, not touch: 32px is the smallest that still reads as one. */
    min-height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-2xs);
  }
  .files button:hover:not(:disabled) {
    border-color: var(--border-strong);
    background: var(--surface-3);
  }
</style>
