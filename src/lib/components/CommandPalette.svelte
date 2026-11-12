<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick, onMount } from 'svelte'
  import { ui, cyclePreview, THEMES, type FileNode } from '../stores/ui.svelte'
  import { tabs } from '../stores/tabs.svelte'
  import { debounce } from '../utils/debounce'
  import { focusTrap } from '../utils/focus'
  import { errorMessage, gitStatus, syncNow } from '../stores/git.svelte'
  import { saveAndSync } from '../utils/save-and-sync'
  import { newFileError } from '../utils/new-file'
  import { importSource, pickImportSource } from '../utils/import'
  import { openFileInTab } from '../utils/open-file'
  import { pushToast } from '../stores/toasts.svelte'
  import { revealLine } from '../editor/bridge'
  import { formatActiveDocument } from '../editor/format'
  import { keyOf } from '../utils/shortcuts'

  interface SearchResult {
    file: string
    line: number
    column: number
    snippet: string
  }

  type Item =
    | { kind: 'file'; path: string; dir: string; name: string }
    | { kind: 'result'; path: string; line: number; column: number; dir: string; name: string; snippet: string }
    | { kind: 'command'; id: string; label: string; hint?: string }

  /// The directory is context, the file name is what you are looking for.
  function splitPath(path: string) {
    const cut = path.lastIndexOf('/')
    return { dir: path.slice(0, cut + 1), name: path.slice(cut + 1) }
  }

  type Prompt = 'file' | 'folder' | null

  let text = $state('')
  let selected = $state(0)
  let prompt = $state<Prompt>(null)
  let searching = $state(false)
  let results = $state<SearchResult[]>([])
  let error = $state<string | null>(null)
  let busy = $state(false)
  // In flight, not just in search mode: the row reserves its height to avoid a shift.
  let pending = $state(false)
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
    if (ui.paletteSearch) {
      searching = true
      ui.paletteSearch = false
    }
    return () => previousFocus?.focus()
  })

  const commands = $derived([
    { id: 'save-sync', label: 'Save and sync', hint: keyOf('saveSync') },
    { id: 'format', label: 'Format document', hint: keyOf('format') },
    { id: 'sync', label: 'Sync' },
    { id: 'preview', label: 'Toggle markdown preview', hint: keyOf('preview') },
    { id: 'theme', label: `Theme: cycle (${ui.editorTheme})` },
    { id: 'new-file', label: 'New file', hint: keyOf('newFile') },
    { id: 'new-folder', label: 'New folder' },
    { id: 'import-file', label: 'Import file…', hint: keyOf('importFile') },
    { id: 'import-folder', label: 'Import folder…' },
    { id: 'search', label: 'Search in files' },
    { id: 'history', label: 'File history', hint: keyOf('history') },
    { id: 'shortcuts', label: 'Keyboard shortcuts', hint: keyOf('shortcuts') },
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
        ...splitPath(r.file),
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
      .map((p) => ({ kind: 'file', path: p, ...splitPath(p) }))
  })

  async function openPath(path: string, line?: number, column?: number) {
    try {
      await openFileInTab(path)
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
    if (id === 'format') {
      close()
      void formatActiveDocument()
      return
    }
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
    if (id === 'history') {
      if (tabs.activeTabId) ui.timelineOpen = !ui.timelineOpen
      return close()
    }
    if (id === 'shortcuts') {
      ui.shortcutsOpen = true
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

  const runSearch = debounce(() => void doSearch(), 400)

  async function doSearch() {
    const query = text.trim()
    if (!query) {
      results = []
      return
    }
    pending = true
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
    } finally {
      // A newer query owns the flag until it finishes.
      if (text.trim() === query) pending = false
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

<div class="overlay" role="dialog" aria-modal="true" aria-label="Command palette">
  <button class="backdrop" tabindex="-1" aria-label="Close" onclick={close}></button>
  <div class="palette" use:focusTrap>
    <input
      bind:this={inputEl}
      bind:value={text}
      oninput={onInput}
      onkeydown={onKeydown}
      aria-label="Search files or run a command"
      placeholder={prompt
        ? `New ${prompt} name`
        : searching
          ? 'Search in files'
          : 'Search files by name, or > for commands'}
      spellcheck="false"
    />
    <p class="searching" role="status">{pending ? 'Searching…' : ''}</p>
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
          <!-- Move, not enter: rows appearing under a still pointer must not steal the selection. -->
          <button
            class="item"
            class:active={i === selected}
            onmousemove={() => (selected = i)}
            onclick={() => activate(item)}
          >
            {#if item.kind === 'command'}
              <span class="label">{item.label}</span>
              {#if item.hint}<span class="hint-key">{item.hint}</span>{/if}
            {:else if item.kind === 'result'}
              <span class="label">
                <span class="dir">{item.dir}</span><span class="name">{item.name}:{item.line}</span>
              </span>
              <span class="snippet">{item.snippet}</span>
            {:else}
              <span class="label">
                <span class="dir">{item.dir}</span><span class="name">{item.name}</span>
              </span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
    {#if error}
      <p class="error" role="alert">{error}</p>
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
    /* Anchored under the title bar, like the VS Code quick pick. */
    padding: calc(var(--h-titlebar) + var(--sp-2)) 16px 0;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    /* The panel floats over the editor, like VS Code's quick pick: no dimming. */
    background: transparent;
  }
  .palette {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 600px;
    max-width: calc(100vw - 32px);
    max-height: 60vh;
    overflow: hidden;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--r-overlay);
    box-shadow: var(--shadow-2);
    animation: gp-pop-in var(--t-fast) var(--ease-out);
  }
  input {
    margin: 8px;
    padding: 8px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .searching {
    min-height: 18px;
    margin: 0 16px 4px;
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  .list {
    padding: 0 6px 6px;
    overflow: auto;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 6px 8px;
    border-radius: var(--r-md);
    text-align: left;
  }
  .item:hover {
    background: var(--surface-2);
  }
  .item.active {
    background: var(--accent);
    color: var(--on-accent);
  }
  .item:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -2px;
  }
  .label {
    display: flex;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }
  /* Both shrink, so a long path loses characters instead of the file name. */
  .dir,
  .name {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dir {
    color: var(--muted);
  }
  .name {
    color: var(--text);
    font-weight: 500;
  }
  /* On the accent row keep the split: white for the name, a light one for the path. */
  .item.active .dir {
    color: inherit;
    opacity: 0.72;
  }
  .item.active .name {
    color: inherit;
  }
  .snippet {
    flex: 0 1 50%;
    overflow: hidden;
    color: var(--muted);
    font-family: ui-monospace, monospace;
    font-size: var(--fs-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item.active .snippet {
    color: inherit;
    opacity: 0.85;
  }
  .hint-key {
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  .item.active .hint-key {
    color: inherit;
    opacity: 0.85;
  }
  .hint,
  .error {
    margin: 12px 16px;
    font-size: var(--fs-sm);
  }
  .hint {
    color: var(--muted);
  }
  .error {
    color: var(--err);
    overflow-wrap: anywhere;
  }
</style>
