<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick, onMount } from 'svelte'
  import { ui, cyclePreview, THEMES, type FileNode } from '../stores/ui.svelte'
  import { tabs, openTab } from '../stores/tabs.svelte'
  import { languageFor } from '../utils/langmap'
  import { debounce } from '../utils/debounce'
  import { errorMessage, gitStatus, syncNow } from '../stores/git.svelte'
  import { saveAndSync } from '../utils/save-and-sync'
  import { newFileError } from '../utils/new-file'
  import { importSource, pickImportSource } from '../utils/import'
  import { openFileInTab } from '../utils/open-file'
  import { pushToast } from '../stores/toasts.svelte'
  import { revealLine } from '../editor/bridge'

  interface SearchResult {
    file: string
    line: number
    column: number
    snippet: string
  }

  type Item =
    | { kind: 'file'; path: string; label: string }
    | { kind: 'result'; path: string; line: number; column: number; label: string; snippet: string }
    | { kind: 'command'; id: string; label: string; hint?: string }

  type Prompt = 'file' | 'folder' | null

  let text = $state('')
  let selected = $state(0)
  let prompt = $state<Prompt>(null)
  let searching = $state(false)
  let results = $state<SearchResult[]>([])
  let error = $state<string | null>(null)
  let busy = $state(false)
  let listEl: HTMLDivElement
  let inputEl: HTMLInputElement
  // Esc closes the palette by unmounting it; the focus goes back where it was.
  let previousFocus: HTMLElement | null = null

  onMount(() => {
    previousFocus = document.activeElement as HTMLElement | null
    inputEl?.focus()
    // A global shortcut asked for the prompt: consume it once.
    if (ui.palettePrompt) {
      prompt = ui.palettePrompt
      ui.palettePrompt = null
    }
    if (ui.paletteQuery !== null) {
      text = ui.paletteQuery
      ui.paletteQuery = null
    }
    return () => previousFocus?.focus()
  })

  const commands = $derived([
    { id: 'save-sync', label: 'Save and sync', hint: 'Ctrl+S' },
    { id: 'sync', label: 'Sync' },
    { id: 'preview', label: 'Toggle markdown preview', hint: 'Ctrl+Shift+P' },
    { id: 'theme', label: `Theme: cycle (${ui.editorTheme})` },
    { id: 'new-file', label: 'New file', hint: 'Ctrl+N' },
    { id: 'new-folder', label: 'New folder' },
    { id: 'import-file', label: 'Import file…', hint: 'Ctrl+O' },
    { id: 'import-folder', label: 'Import folder…' },
    { id: 'search', label: 'Search in files' },
    { id: 'settings', label: 'Open settings' },
  ])

  function flatten(nodes: FileNode[]): string[] {
    const out: string[] = []
    for (const node of nodes) {
      if (node.type === 'dir') out.push(...flatten(node.children ?? []))
      else out.push(node.path)
    }
    return out
  }

  const filePaths = $derived(flatten(ui.fileTree))

  /// Ctrl+N must not overwrite a folder either, so collect files and dirs.
  function allPaths(nodes: FileNode[]): string[] {
    const out: string[] = []
    for (const node of nodes) {
      out.push(node.path)
      if (node.type === 'dir') out.push(...allPaths(node.children ?? []))
    }
    return out
  }

  const existingPaths = $derived(allPaths(ui.fileTree))

  const extensionHint = $derived(
    Object.keys(ui.extensions)
      .slice(0, 6)
      .map((ext) => `.${ext}`)
      .join(' '),
  )

  const commandMode = $derived(prompt === null && !searching && text.startsWith('>'))
  const needle = $derived((commandMode ? text.slice(1) : text).trim().toLowerCase())

  const items = $derived.by<Item[]>(() => {
    if (prompt) return []
    if (searching) {
      return results.map((r) => ({
        kind: 'result',
        path: r.file,
        line: r.line,
        column: r.column,
        label: `${r.file}:${r.line}`,
        snippet: r.snippet,
      }))
    }
    if (commandMode) {
      return commands
        .filter((c) => c.label.toLowerCase().includes(needle))
        .map((c) => ({ kind: 'command', id: c.id, label: c.label, hint: c.hint }))
    }
    return filePaths
      .filter((p) => p.toLowerCase().includes(needle))
      .slice(0, 100)
      .map((p) => ({ kind: 'file', path: p, label: p }))
  })

  async function openPath(path: string, line?: number, column?: number) {
    try {
      if (!tabs.tabs.some((t) => t.id === path)) {
        const content = await invoke<string>('fs_read_file', { path })
        openTab({ id: path, path, content, modified: false, language: languageFor(path) })
      } else {
        tabs.activeTabId = path
      }
      close()
      if (line !== undefined) {
        await tick()
        revealLine(line, column ?? 1)
      }
    } catch (e) {
      error = String(e)
    }
  }

  function close() {
    ui.commandPaletteOpen = false
  }

  async function sync() {
    busy = true
    try {
      if (await syncNow(null)) close()
      else error = gitStatus.error ?? 'sync failed'
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }

  function cycleTheme() {
    const next = THEMES[(THEMES.indexOf(ui.editorTheme) + 1) % THEMES.length]
    ui.editorTheme = next
  }

  async function run(id: string) {
    if (id === 'save-sync') {
      close()
      void saveAndSync()
      return
    }
    if (id === 'sync') return void sync()
    if (id === 'preview') {
      cyclePreview()
      return close()
    }
    if (id === 'theme') return cycleTheme()
    if (id === 'new-file' || id === 'new-folder') {
      prompt = id === 'new-file' ? 'file' : 'folder'
      text = ''
      return
    }
    if (id === 'import-file' || id === 'import-folder') return void importFrom(id === 'import-folder')
    if (id === 'settings') {
      ui.settingsOpen = true
      return close()
    }
    if (id === 'search') {
      searching = true
      text = ''
      results = []
      return
    }
  }

  /// Imports into the repo root. A subfolder destination stays in the tree's context menu.
  async function importFrom(directory: boolean) {
    // Close first: the native dialog needs the focus out of the palette.
    close()
    try {
      const source = await pickImportSource(directory)
      if (!source) return
      pushToast('Importing…', 'info')
      const path = await importSource(source, null)
      ui.treeVersion += 1
      if (!directory) await openFileInTab(path)
      pushToast(`Imported ${path}`, 'success')
    } catch (e) {
      pushToast(errorMessage(e), 'error')
    }
  }

  async function create() {
    const name = text.trim()
    if (!prompt || !name || busy) return
    busy = true
    try {
      if (prompt === 'file') {
        const invalid = newFileError(name, existingPaths)
        if (invalid) {
          error = invalid
          return
        }
        await invoke('fs_write_file', { path: name, content: '' })
        ui.treeVersion += 1
        await openPath(name)
        return
      }
      await invoke('fs_create_dir', { path: name })
      ui.treeVersion += 1
      close()
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }

  const runSearch = debounce(() => void doSearch(), 250)

  async function doSearch() {
    const query = text.trim()
    if (!query) {
      results = []
      return
    }
    try {
      const found = await invoke<SearchResult[]>('search_global', { query })
      // A slower search for an older query must not overwrite the current one.
      if (text.trim() !== query) return
      results = found
      error = null
      selected = 0
    } catch (e) {
      if (text.trim() !== query) return
      results = []
      error = String(e)
    }
  }

  function onInput() {
    selected = 0
    if (searching) runSearch.run()
  }

  function activate(item: Item | undefined = items[selected]) {
    if (prompt) return void create()
    if (!item) return
    if (item.kind === 'command') return void run(item.id)
    if (item.kind === 'result') return void openPath(item.path, item.line, item.column)
    void openPath(item.path)
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault()
      return close()
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (items.length) selected = (selected + 1) % items.length
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (items.length) selected = (selected - 1 + items.length) % items.length
      return
    }
    if (e.key === 'Enter') {
      e.preventDefault()
      activate()
    }
  }

  // Keep the keyboard selection inside the scrollable list.
  $effect(() => {
    selected
    listEl?.querySelector('.item.active')?.scrollIntoView({ block: 'nearest' })
  })
</script>

<div class="overlay" role="dialog" aria-label="Command palette">
  <button class="backdrop" aria-label="Close" onclick={close}></button>
  <div class="palette">
    <input
      bind:this={inputEl}
      bind:value={text}
      oninput={onInput}
      onkeydown={onKeydown}
      placeholder={prompt
        ? `New ${prompt} name`
        : searching
          ? 'Search in files'
          : 'Search files by name, or > for commands'}
      spellcheck="false"
    />
    <div class="list" bind:this={listEl}>
      {#if prompt}
        <p class="hint">
          {text.trim()
            ? `Enter to create ${text.trim()}`
            : prompt === 'file'
              ? `Type a name with extension, e.g. notes.md — ${extensionHint}`
              : 'Type a path, e.g. notes/today'}
        </p>
      {:else if items.length === 0}
        <p class="hint">
          {searching && needle ? 'No matches' : searching ? 'Type to search' : 'No results'}
        </p>
      {:else}
        {#each items as item, i (item.kind === 'file' ? item.path : item.kind === 'result' ? `${item.path}:${item.line}` : item.id)}
          <button
            class="item"
            class:active={i === selected}
            onmouseenter={() => (selected = i)}
            onclick={() => activate(item)}
          >
            {#if item.kind === 'command'}
              <span class="label">{item.label}</span>
              {#if item.hint}<span class="hint-key">{item.hint}</span>{/if}
            {:else if item.kind === 'result'}
              <span class="label">{item.label}</span>
              <span class="snippet">{item.snippet}</span>
            {:else}
              <span class="label">{item.label}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    /* Anchored under the 32px title bar, like the VS Code quick pick. */
    padding: 40px 16px 0;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
  }
  .palette {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 620px;
    max-width: calc(100vw - 32px);
    max-height: 60vh;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    animation: drop 150ms ease-out;
  }
  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .palette {
      animation: none;
    }
  }
  input {
    padding: 10px 12px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    font: inherit;
  }
  input:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -2px;
  }
  .list {
    overflow: auto;
  }
  .item {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    padding: 6px 12px;
    text-align: left;
  }
  .item.active {
    background: var(--accent);
    color: #fff;
  }
  .item:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -2px;
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .snippet {
    flex: 0 1 50%;
    overflow: hidden;
    color: var(--muted);
    font-family: ui-monospace, monospace;
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item.active .snippet {
    color: inherit;
    opacity: 0.8;
  }
  .hint-key {
    color: var(--muted);
    font-size: 11px;
  }
  .item.active .hint-key {
    color: inherit;
  }
  .hint,
  .error {
    margin: 10px 12px;
    font-size: 12px;
  }
  .hint {
    color: var(--muted);
  }
  .error {
    color: var(--err);
    overflow-wrap: anywhere;
  }
</style>
