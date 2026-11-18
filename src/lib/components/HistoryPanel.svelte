<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import {
    applyGitStatus,
    errorMessage,
    gitStatus,
    type CommitEntry,
    type FileChange,
    type GitStatus,
  } from '../stores/git.svelte'
  import { ui } from '../stores/ui.svelte'
  import { confirmDialog } from '../stores/confirm.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { reloadTabFromDisk } from '../editor/bridge'
  import { restoreFileAt } from '../utils/restore'
  import { tip } from '../utils/tooltip'

  interface CommitGroup {
    key: string
    label: string
    commits: CommitEntry[]
  }

  let commits = $state<CommitEntry[]>([])
  let expanded = $state('')
  let files = $state<FileChange[]>([])
  let busy = $state(false)
  let loading = $state(false)
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
    loading = true
    error = null
    try {
      const result = target
        ? await invoke<CommitEntry[]>('git_file_log', { path: target })
        : await invoke<CommitEntry[]>('git_log')
      if (seq === loadSeq) commits = result
    } catch (e) {
      // Keep the last good list on screen; the summary's error line explains why.
      if (seq === loadSeq) error = errorMessage(e)
    } finally {
      if (seq === loadSeq) loading = false
    }
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
      await restoreFileAt(commit.hash, file.path)
      // The tab stays open: EditorPane re-reads the file into its model.
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
      : `Revert ${commit.shortHash}: ${humanize(commit.message)}?`
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

  /// GitPad writes `gitpad: <verb> <detail> <ISO8601>`; the rail carries the verb,
  /// so the row only needs the sentence a person would say.
  function humanize(message: string) {
    if (!message.startsWith('gitpad: ')) return message
    const body = message
      .replace(/\s+\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/, '')
      .slice('gitpad: '.length)
    const space = body.indexOf(' ')
    const verb = space === -1 ? body : body.slice(0, space)
    const detail = space === -1 ? '' : body.slice(space + 1)
    switch (verb) {
      case 'update':
        return detail ? `Updated ${detail}` : 'Updated notes'
      case 'delete':
        return detail ? `Deleted ${detail}` : 'Deleted a note'
      case 'rename':
        return detail ? `Renamed ${detail}` : 'Renamed a note'
      case 'import':
        return detail ? `Imported ${detail}` : 'Imported notes'
      case 'revert':
        return detail ? `Reverted ${detail}` : 'Reverted a commit'
      case 'sync':
        return 'Synced notes'
      case 'merge':
        return 'Merged origin'
      default:
        return message
    }
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
  <div class="panel" aria-busy={loading}>
    <div class="summary">
      <span class="count">
        {loading ? 'Loading…' : `${commits.length} commit${commits.length === 1 ? '' : 's'}`}
      </span>
      <button
        class="refresh"
        class:busy={loading}
        disabled={loading}
        use:tip={'Refresh'}
        aria-label="Refresh history"
        onclick={() => void load()}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M14 8a6 6 0 1 1-6-6c1.68 0 3.29.67 4.49 1.83L14 5.33" />
          <path d="M14 2v4h-4" />
        </svg>
      </button>
    </div>

    {#if filePath}
      <p class="filter">
        History for <code>{filePath.split('/').pop()}</code>
      </p>
    {/if}
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    {#if commits.length === 0 && !loading && !error}
      <p class="empty">
        {filePath ? 'No commits touch this file yet.' : 'No commits yet.'}
      </p>
    {:else if groups.length > 0}
      <ul class="days">
        {#each groups as group (group.key)}
          <li class="day-group">
            <div class="day">
              <span class="day-name">
                <span class="dot" aria-hidden="true"></span>
                {group.label}
              </span>
              <span
                class="day-count"
                aria-label={`${group.commits.length} commit${group.commits.length === 1 ? '' : 's'}`}
              >
                {group.commits.length}
              </span>
            </div>
            <ul class="card">
              {#each group.commits as commit (commit.hash)}
                {@const kind = kindOf(commit.message)}
                <li
                  class="commit {kind}"
                  class:open={expanded === commit.hash}
                  class:active={ui.timelineCommit?.hash === commit.hash}
                >
                  <div class="head-row">
                    <button class="row" onclick={() => open(commit)}>
                      <span class="msg">{humanize(commit.message)}</span>
                      <span class="meta">
                        <code>{commit.shortHash}</code>
                        <span>{timeLabel(commit.timestamp)}</span>
                        {#if commit.author && commit.author !== 'GitPad'}
                          <span class="author">{commit.author}</span>
                        {/if}
                        <span class="files-count">{fileCount(commit.filesChanged)}</span>
                      </span>
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
                    <div class="details">
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
                      <div class="actions">
                        <button disabled={busy} onclick={() => revert(commit)}>
                          {allDeleted
                            ? `Restore ${files.length} deleted item${files.length === 1 ? '' : 's'}`
                            : 'Revert commit'}
                        </button>
                      </div>
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-25) var(--sp-1);
  }
  .count {
    padding-left: var(--sp-0);
    color: var(--muted);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    line-height: 1.2;
  }
  .refresh {
    display: grid;
    flex: 0 0 26px;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    transition:
      background var(--t-fast),
      color var(--t-fast);
  }
  .refresh:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }
  .refresh:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .refresh svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .refresh.busy svg {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .refresh.busy svg {
      animation: none;
    }
  }
  .filter {
    margin: 0;
    padding: 0 var(--sp-25) var(--sp-1);
    color: var(--muted);
    font-size: var(--fs-2xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .filter code {
    color: var(--accent-fg);
  }
  .error {
    margin: 0;
    padding: 0 var(--sp-25) var(--sp-1);
    color: var(--err);
    font-size: var(--fs-sm);
    overflow-wrap: anywhere;
  }
  .empty {
    margin: 0;
    padding: var(--sp-6) var(--sp-25);
    color: var(--muted);
    font-size: var(--fs-sm);
    text-align: center;
    overflow-wrap: anywhere;
  }
  .days,
  .card,
  .files {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .days {
    padding-bottom: var(--sp-25);
  }
  /* One card per day: the day label stays outside it, stuck to the top. */
  .day {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-25) var(--sp-1);
    background: var(--surface-1);
    color: var(--accent-fg);
    font-size: 12px;
    line-height: 1.2;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .day-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-right: var(--sp-1);
    border-radius: var(--r-pill);
    background: var(--accent);
    vertical-align: middle;
  }
  .day-count {
    flex: 0 0 auto;
    padding-right: var(--sp-25);
    color: var(--muted);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }
  .card {
    margin: 0 var(--sp-25) var(--sp-25);
    border: 1px solid color-mix(in srgb, var(--border) 65%, var(--surface-1));
    border-radius: var(--r-lg);
    background: color-mix(in srgb, var(--surface-2) 62%, var(--surface-1));
    overflow: hidden;
  }
  .commit {
    --kind: var(--border-strong);
    --kind-ink: var(--muted);
    border: 1px solid transparent;
    border-left-color: var(--kind);
    background: transparent;
  }
  .commit.delete {
    --kind: var(--err);
    --kind-ink: var(--err);
  }
  .commit.revert {
    --kind: var(--warn);
    --kind-ink: var(--warn);
  }
  .commit.import {
    --kind: var(--ok);
    --kind-ink: var(--ok);
  }
  .commit.update,
  .commit.sync {
    --kind: var(--accent-fg);
    --kind-ink: var(--accent-fg);
  }
  .commit + .commit {
    border-top-color: color-mix(in srgb, var(--divider) 70%, transparent);
  }
  .commit:hover,
  .commit.open {
    background: var(--surface-3);
  }
  .commit.active {
    border-width: 2px;
    border-color: var(--accent);
    background: transparent;
  }
  /* A 2px outline must not change the row's total height: give the extra 2px
     back to the head and the expand strip. */
  .commit.active .head-row,
  .commit.active .expand {
    min-height: 46px;
  }
  /* The card's 1px border sits outside this radius, so the accent follows the
     curve instead of squaring off against it. */
  .commit.active:first-child {
    border-top-left-radius: calc(var(--r-lg) - 1px);
    border-top-right-radius: calc(var(--r-lg) - 1px);
  }
  .commit.active:last-child {
    border-bottom-left-radius: calc(var(--r-lg) - 1px);
    border-bottom-right-radius: calc(var(--r-lg) - 1px);
  }
  .commit.active:only-child {
    border-radius: calc(var(--r-lg) - 1px);
  }
  /* The active row's 2px bottom border replaces the next divider; reclaim its
     1px so the row below keeps the same height. */
  .commit.active + .commit {
    border-top-width: 0;
  }
  .commit.active + .commit .head-row,
  .commit.active + .commit .expand {
    min-height: 49px;
  }
  .head-row {
    display: flex;
    align-items: stretch;
    min-height: 48px;
  }
  .row {
    display: flex;
    flex: 1;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    min-width: 0;
    padding: var(--sp-1) var(--sp-25);
    border: 0;
    border-radius: 0;
    background: transparent;
    text-align: left;
  }
  .row:focus-visible,
  .expand:focus-visible {
    outline-offset: -2px;
  }
  .msg {
    overflow: hidden;
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: baseline;
    gap: var(--sp-1);
    min-width: 0;
    color: var(--muted);
    font-size: 11px;
    line-height: 1.2;
  }
  .meta code {
    flex: 0 0 auto;
    color: var(--kind-ink);
    font-size: var(--fs-xs);
    font-weight: 600;
  }
  .meta > span {
    flex: 0 0 auto;
    white-space: nowrap;
  }
  .meta .author {
    flex: 0 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta .files-count {
    margin-left: auto;
  }
  .expand {
    display: grid;
    flex: 0 0 28px;
    place-items: center;
    width: 28px;
    min-height: 48px;
    padding: 0;
    border: 0;
    border-left: 1px solid color-mix(in srgb, var(--divider) 70%, transparent);
    border-radius: 0;
    background: transparent;
    color: var(--muted);
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
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .expand.open .chev {
    transform: rotate(90deg);
  }
  /* Files sit on their own surface so the row above stays the primary line. */
  .details {
    border-top: 1px solid color-mix(in srgb, var(--divider) 70%, transparent);
    background: var(--surface-1);
  }
  .files {
    padding: var(--sp-1) 0;
  }
  .files li {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-height: 32px;
    padding: 0 var(--sp-25) 0 var(--sp-3);
  }
  .files li:hover {
    background: var(--surface-2);
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: var(--fs-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: 0 0 auto;
    width: 14px;
    font-size: var(--fs-xs);
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
    flex: 0 0 auto;
    min-height: 28px;
    padding: 0 var(--sp-1);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--accent-fg);
    font-size: 12px;
    font-weight: 500;
  }
  .files button:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    padding: var(--sp-1) var(--sp-25) var(--sp-25);
  }
  .actions button {
    min-height: 32px;
    padding: 0 var(--sp-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-sm);
    font-weight: 500;
  }
  .actions button:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .actions button:disabled,
  .files button:disabled {
    cursor: default;
    opacity: 0.5;
  }
  /* High contrast is a legibility guarantee, not a place to soften surfaces. */
  :global(:root[data-theme='hc-black']) .card {
    background: var(--surface-2);
    border-color: var(--border);
  }
  :global(:root[data-theme='hc-black']) .commit + .commit,
  :global(:root[data-theme='hc-black']) .expand,
  :global(:root[data-theme='hc-black']) .details {
    border-color: var(--divider);
  }
</style>
