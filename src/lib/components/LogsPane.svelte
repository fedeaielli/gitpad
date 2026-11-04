<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { errorMessage } from '../stores/git.svelte'
  import {
    buildSessions,
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
  let query = $state('')
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
  const total = $derived(visible.reduce((sum, session) => sum + session.entries.length, 0))

  type Row =
    | { kind: 'session'; key: string; startedAt: number; running: boolean }
    | { kind: 'line'; key: string; entry: LogEntry }

  // One flat list so the render cap counts lines, not sessions.
  const rows = $derived.by(() => {
    const list: Row[] = []
    let count = 0
    for (const session of visible) {
      if (count >= limit) break
      list.push({
        kind: 'session',
        key: session.key,
        startedAt: session.startedAt,
        running: session.current && session === visible[0],
      })
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

<div class="bar">
  <select aria-label="Log session" bind:value={selected} onchange={() => void load()}>
    {#if files.length === 0}
      <option value="">No log files</option>
    {/if}
    {#each files as file (file.name)}
      <option value={file.name}>{fileLabel(file)} · {bytes(file.size)}</option>
    {/each}
  </select>
  <select aria-label="Log level" bind:value={level}>
    {#each LEVELS as item (item.id)}
      <option value={item.id}>{item.label}</option>
    {/each}
  </select>
  <div class="chips" role="group" aria-label="Log category">
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
  <input type="search" placeholder="Search messages" aria-label="Search logs" bind:value={query} />
  <button type="button" class="refresh" disabled={loading} onclick={() => void refresh()}>
    {loading ? 'Loading…' : 'Refresh'}
  </button>
</div>

{#if error}
  <p class="note">{error}</p>
{/if}

<p class="note">
  {total === 0
    ? 'No lines match.'
    : `${Math.min(total, limit)} of ${total} lines — newest session first.`}
</p>

<div class="log" role="log" aria-label="Log lines">
  {#each rows as row (row.key)}
    {#if row.kind === 'session'}
      <div class="session">
        <span>{new Date(row.startedAt).toLocaleString()}</span>
        {#if row.running}
          <span class="now">running</span>
        {/if}
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

{#if total > limit}
  <button type="button" class="more" onclick={() => (limit += 300)}>
    Show more
  </button>
{/if}

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
  }
  select,
  input {
    padding: var(--sp-1) var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-0);
    color: var(--text);
  }
  select:focus,
  input:focus {
    border-color: var(--accent-fg);
    outline: none;
  }
  input {
    flex: 1;
    min-width: 140px;
  }
  .chips {
    display: flex;
    gap: 4px;
  }
  .chip {
    padding: 3px 8px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
  }
  .chip[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
  .refresh,
  .more {
    padding: var(--sp-1) var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-0);
    color: var(--text);
  }
  .refresh:hover:not(:disabled),
  .more:hover {
    border-color: var(--accent-fg);
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
  }
  .more {
    align-self: flex-start;
  }
  .log {
    max-height: 46vh;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-0);
    font-family: ui-monospace, monospace;
    font-size: 11px;
    line-height: 1.5;
  }
  .session {
    position: sticky;
    top: 0;
    display: flex;
    gap: 8px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--muted);
  }
  .now {
    color: var(--ok);
  }
  .line {
    display: grid;
    grid-template-columns: 62px 46px 52px 1fr;
    gap: 8px;
    padding: 1px 8px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .line:hover {
    background: var(--surface-2);
  }
  .time,
  .cat {
    color: var(--muted);
  }
  .level {
    text-transform: uppercase;
    color: var(--muted);
  }
  .line.warn .level {
    color: var(--warn);
  }
  .line.error .level,
  .line.error .msg {
    color: var(--err);
  }
  .empty {
    margin: 0;
    padding: 12px;
    color: var(--muted);
  }
</style>
