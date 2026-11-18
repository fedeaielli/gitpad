<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { errorMessage } from '../stores/git.svelte'
  import { debounce } from '../utils/debounce'
  import {
    buildSessions,
    isStartOnly,
    matches,
    type LogCategory,
    type LogEntry,
    type LogLevel,
  } from '../utils/logs'

  interface LogFile {
    name: string
    current: boolean
    modified: number
    size: number
  }

  const CATEGORIES: Array<{ id: LogCategory; label: string }> = [
    { id: 'editor', label: 'Editor' },
    { id: 'git', label: 'Git' },
    { id: 'app', label: 'App' },
  ]
  const LEVELS: Array<{ id: LogLevel | 'all'; label: string }> = [
    { id: 'all', label: 'All levels' },
    { id: 'info', label: 'Info and up' },
    { id: 'warn', label: 'Warnings and up' },
    { id: 'error', label: 'Errors only' },
  ]

  let files = $state<LogFile[]>([])
  let selected = $state('')
  let text = $state('')
  let loading = $state(false)
  let error = $state('')
  let level = $state<LogLevel | 'all'>('all')
  let categories = $state<LogCategory[]>([])
  // The typed value and the filter it feeds: filtering every keystroke rebuilds
  // the whole list, so the two are separated by a short debounce.
  let queryInput = $state('')
  let query = $state('')
  const applyQuery = debounce(() => (query = queryInput), 150)
  let limit = $state(300)

  const sessions = $derived(
    buildSessions(files.filter((f) => f.name === selected).map((f) => ({ ...f, text }))),
  )
  const visible = $derived(
    sessions
      .map((session) => ({
        ...session,
        entries: session.entries.filter((entry) =>
          matches(entry, level, categories, query),
        ),
      }))
      .filter((session) => session.entries.length > 0),
  )
  // Quiet sessions collapse to their header, so they do not count a line.
  const total = $derived(
    visible.reduce((sum, session) => sum + session.entries.length - (isStartOnly(session) ? 1 : 0), 0),
  )

  type Row =
    | { kind: 'session'; key: string; startedAt: number; running: boolean; quiet: boolean }
    | { kind: 'line'; key: string; entry: LogEntry }

  // One flat list so the render cap counts lines, not sessions.
  const rows = $derived.by(() => {
    const list: Row[] = []
    let count = 0
    for (const session of visible) {
      if (count >= limit) break
      const quiet = isStartOnly(session)
      list.push({
        kind: 'session',
        key: session.key,
        startedAt: session.startedAt,
        running: session.current && session === visible[0],
        quiet,
      })
      // A session that only recorded its own start needs no second line.
      if (quiet) continue
      for (const [index, entry] of session.entries.entries()) {
        if (count >= limit) break
        list.push({ kind: 'line', key: `${session.key}:${index}`, entry })
        count++
      }
    }
    return list
  })

  // A new filter is a new list: start from the top again.
  $effect(() => {
    void level
    void categories
    void query
    limit = 300
  })

  const fileLabel = (file: LogFile) =>
    file.current
      ? 'Current session'
      : `Session · ${new Date(file.modified).toLocaleString()}`

  const bytes = (size: number) =>
    size > 1024 * 1024 ? `${(size / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(size / 1024))} KB`

  async function load() {
    if (!selected) return
    loading = true
    error = ''
    try {
      text = await invoke<string>('logs_read', { name: selected })
    } catch (e) {
      text = ''
      error = errorMessage(e)
    } finally {
      loading = false
    }
  }

  async function refresh() {
    try {
      files = await invoke<LogFile[]>('logs_list')
      if (!files.some((file) => file.name === selected)) {
        selected = (files.find((file) => file.current) ?? files[0])?.name ?? ''
      }
      await load()
    } catch (e) {
      error = errorMessage(e)
    }
  }

  function toggle(category: LogCategory) {
    categories = categories.includes(category)
      ? categories.filter((item) => item !== category)
      : [...categories, category]
  }

  onMount(() => {
    void refresh()
  })
</script>

<div class="pane">
  <div class="filters">
    <div class="row">
      <div class="group grow">
        <label class="label" for="log-session">Session</label>
        <select id="log-session" bind:value={selected} onchange={() => void load()}>
          {#if files.length === 0}
            <option value="">No log files</option>
          {/if}
          {#each files as file (file.name)}
            <option value={file.name}>{fileLabel(file)} · {bytes(file.size)}</option>
          {/each}
        </select>
      </div>
      <button type="button" class="refresh sweep" disabled={loading} onclick={() => void refresh()}>
        {loading ? 'Loading…' : 'Refresh'}
      </button>
    </div>
    <div class="row">
      <div class="group level">
        <label class="label" for="log-level">Level</label>
        <select id="log-level" bind:value={level}>
          {#each LEVELS as item (item.id)}
            <option value={item.id}>{item.label}</option>
          {/each}
        </select>
      </div>
      <div class="group">
        <span class="label" id="log-category">Category</span>
        <div class="chips" role="group" aria-labelledby="log-category">
          {#each CATEGORIES as item (item.id)}
            <button
              type="button"
              class="chip"
              aria-pressed={categories.includes(item.id)}
              onclick={() => toggle(item.id)}
            >
              {item.label}
            </button>
          {/each}
        </div>
      </div>
      <div class="group grow">
        <label class="label" for="log-search">Search</label>
        <input
          id="log-search"
          type="search"
          placeholder="Search messages"
          bind:value={queryInput}
          oninput={applyQuery.run}
        />
      </div>
    </div>
  </div>

  {#if error}
    <p class="note error" role="alert">{error}</p>
  {/if}

  <div class="meta">
    <span class="note">
      {total === 0
        ? '0 lines'
        : `${Math.min(total, limit)} of ${total} ${total === 1 ? 'line' : 'lines'}`}
    </span>
    {#if total > limit}
      <button type="button" class="more" onclick={() => (limit += 300)}>Show more</button>
    {/if}
  </div>

  <!-- A scrollable region needs focus to be reachable and scrolled by keyboard. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="log" role="log" aria-label="Log lines" tabindex="0">
    {#each rows as row (row.key)}
      {#if row.kind === 'session'}
        <div class="session">
          <span>{new Date(row.startedAt).toLocaleString()}</span>
          <span class="session-state">
            {#if row.running}
              <span class="now">running</span>
            {/if}
            {#if row.quiet}
              <span class="idle">app started</span>
            {/if}
          </span>
        </div>
      {:else}
        <div class="line {row.entry.level}">
          <span class="time">{row.entry.time || '—'}</span>
          <span class="level">{row.entry.level}</span>
          <span class="cat">{row.entry.category}</span>
          <span class="msg">{row.entry.message}</span>
        </div>
      {/if}
    {/each}
    {#if total === 0 && !loading}
      <p class="empty">
        {files.length === 0
          ? 'GitPad has not written a log file yet.'
          : 'Nothing here with these filters.'}
      </p>
    {/if}
  </div>
</div>

<style>
  /* Fills the tab: only the log scrolls, never the settings body. */
  .pane {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--sp-2);
    min-height: 0;
  }
  .filters {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--sp-2);
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--sp-05);
    min-width: 0;
  }
  .group.grow {
    flex: 1 1 160px;
  }
  .group.level {
    flex: 0 0 140px;
  }
  .label {
    color: var(--accent-fg);
    font-size: var(--fs-xs);
  }
  select,
  input {
    box-sizing: border-box;
    width: 100%;
    height: 32px;
    padding: 0 var(--sp-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-1);
    color: var(--text);
    font: inherit;
  }
  .refresh,
  .more {
    flex: 0 0 auto;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-1);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .refresh {
    box-sizing: border-box;
    min-width: 96px;
    height: 32px;
    padding: 0 var(--sp-3);
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
  .refresh:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 88%, black);
  }
  .refresh:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .chips {
    display: flex;
    gap: var(--sp-05);
  }
  .chip {
    height: 32px;
    padding: 0 var(--sp-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-1);
    color: var(--muted);
    font: inherit;
    font-size: var(--fs-md);
    cursor: pointer;
  }
  .chip[aria-pressed='true'] {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface-1));
    color: var(--accent-fg);
  }
  .meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    min-height: 24px;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  .note.error {
    color: var(--err);
  }
  .more {
    height: 24px;
    padding: 0 var(--sp-2);
    border-color: var(--border);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  .more:hover {
    color: var(--text);
    border-color: var(--border-strong);
    background: transparent;
  }
  .log {
    flex: 1 1 auto;
    min-height: 96px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
    font-family: ui-monospace, monospace;
    font-size: 13px;
    line-height: 1.55;
  }
  .session {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-2);
    border-bottom: 1px solid var(--border);
    border-left: 2px solid transparent;
    background: var(--surface-2);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .session-state {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    white-space: nowrap;
  }
  .now {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
    color: var(--ok);
  }
  .now::before {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    content: '';
  }
  .idle {
    opacity: 0.8;
  }
  .line {
    display: grid;
    grid-template-columns: 68px 48px 56px 1fr;
    gap: var(--sp-2);
    padding: 2px var(--sp-2);
    border-left: 2px solid transparent;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .line:hover {
    background: var(--surface-2);
  }
  .line.warn {
    border-left-color: var(--warn);
  }
  .line.error {
    border-left-color: var(--err);
  }
  .time,
  .cat {
    color: var(--muted);
  }
  .level {
    color: var(--muted);
    text-transform: uppercase;
  }
  .line.warn .level {
    color: var(--warn);
  }
  .line.error .level {
    color: var(--err);
  }
  .empty {
    margin: 0;
    padding: var(--sp-4);
    color: var(--muted);
  }
</style>
