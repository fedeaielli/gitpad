<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ui, type FileNode } from '../stores/ui.svelte'
  import { tabs, discardUnder, retarget } from '../stores/tabs.svelte'
  import { openFileInTab } from '../utils/open-file'
  import { pickAndImport } from '../utils/import'
  import { sortNodes, visibleNodes } from '../utils/file-sort'
  import { errorMessage, isNoRepoError } from '../stores/git.svelte'

  type MenuMode = 'menu' | 'new-file' | 'new-folder' | 'delete'

  let error = $state<string | null>(null)
  let loading = $state(true)
  let noRepo = $state(false)
  let menu = $state<{ x: number; y: number; node: FileNode | null; mode: MenuMode } | null>(null)
  let name = $state('')
  let renaming = $state<string | null>(null)
  let renameValue = $state('')

  /// Dot files stay hidden unless the options menu enables them.
  function visible(nodes: FileNode[]): FileNode[] {
    return visibleNodes(nodes, ui.showHidden)
  }

  function sorted(nodes: FileNode[]): FileNode[] {
    return sortNodes(visible(nodes), ui.sortMode, ui.sortDesc)
  }

  function focusInput(node: HTMLElement) {
    node.focus()
    if (node instanceof HTMLInputElement) node.select()
  }

  function startRename(node: FileNode) {
    renaming = node.path
    renameValue = node.name
  }

  async function commitRename(node: FileNode) {
    if (renaming !== node.path) return
    const value = renameValue.trim()
    renaming = null
    const to = join(parentOf(node.path), value)
    if (!value || to === node.path) return
    error = null
    try {
      await invoke('fs_rename', { from: node.path, to })
      retarget(node.path, to)
      ui.expanded = ui.expanded.map((p) =>
        p === node.path || p.startsWith(`${node.path}/`)
          ? to + p.slice(node.path.length)
          : p,
      )
      await loadTree()
    } catch (e) {
      error = errorMessage(e)
    }
  }

  // Keyed by the language in ui.extensions so a custom extension inherits its
  // icon; the extension keys cover files the default list does not map.
  const ICONS: Record<string, string> = {
    markdown: '📝',
    plaintext: '📄',
    python: '🐍',
    rust: '⚙️',
    javascript: '⚡',
    typescript: '⚡',
    json: '{}',
    yaml: '⚙',
    toml: '⚙',
    shell: '$',
    css: '🎨',
    html: '🌐',
  }

  function iconFor(file: string): string {
    const ext = file.split('.').pop()?.toLowerCase() ?? ''
    return ICONS[ui.extensions[ext] ?? ext] ?? '📄'
  }

  function toggle(path: string) {
    ui.expanded = ui.expanded.includes(path)
      ? ui.expanded.filter((p) => p !== path)
      : [...ui.expanded, path]
  }

  function parentOf(path: string): string {
    return path.split('/').slice(0, -1).join('/')
  }

  function join(base: string, child: string): string {
    return base ? `${base}/${child}` : child
  }

  async function loadTree() {
    try {
      ui.fileTree = await invoke<FileNode[]>('fs_list_dir', { path: null })
      error = null
      noRepo = false
    } catch (e) {
      ui.fileTree = []
      const message = errorMessage(e)
      noRepo = isNoRepoError(message)
      error = noRepo ? null : message
    } finally {
      loading = false
    }
  }

  $effect(() => {
    ui.treeVersion
    void loadTree()
  })

  async function openFile(node: FileNode) {
    try {
      await openFileInTab(node.path)
    } catch (e) {
      error = errorMessage(e)
    }
  }

  function openMenu(event: MouseEvent, node: FileNode | null) {
    event.preventDefault()
    if (noRepo) return
    menu = { x: event.clientX, y: event.clientY, node, mode: 'menu' }
  }

  function start(mode: MenuMode) {
    if (!menu) return
    name = ''
    menu = { ...menu, mode }
  }

  async function importFromPicker(directory: boolean) {
    if (!menu) return
    const base = menu.node ? (menu.node.type === 'dir' ? menu.node.path : parentOf(menu.node.path)) : ''
    menu = null
    error = null
    try {
      const path = await pickAndImport(directory, base || null)
      if (!path) return
      if (base && !ui.expanded.includes(base)) ui.expanded = [...ui.expanded, base]
      await loadTree()
      if (!directory) {
        await openFile({ name: path.split('/').pop() ?? path, path, type: 'file' })
      }
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function submitName(event: SubmitEvent) {
    event.preventDefault()
    if (!menu || !name.trim()) return
    error = null
    const value = name.trim()
    const base = menu.node ? (menu.node.type === 'dir' ? menu.node.path : parentOf(menu.node.path)) : ''
    try {
      if (menu.mode === 'new-file') {
        const target = join(base, value)
        await invoke('fs_write_file', { path: target, content: '' })
        menu = null
        await loadTree()
        await openFile({ name: value, path: target, type: 'file' })
        return
      }
      if (menu.mode === 'new-folder') {
        await invoke('fs_create_dir', { path: join(base, value) })
      }
      menu = null
      await loadTree()
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function reveal() {
    if (!menu) return
    const path = menu.node?.path ?? ''
    menu = null
    error = null
    try {
      await invoke('reveal_in_filemanager', { path })
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function remove() {
    if (!menu?.node) return
    error = null
    const path = menu.node.path
    try {
      await invoke('fs_delete', { path })
      discardUnder(path)
      menu = null
      await loadTree()
    } catch (e) {
      error = errorMessage(e)
    }
  }
</script>

{#snippet tree(nodes: FileNode[])}
  <ul>
    {#each sorted(nodes) as node (node.path)}
      <li>
        {#if renaming === node.path}
          <form
            class="rename"
            onsubmit={(e) => {
              e.preventDefault()
              void commitRename(node)
            }}
          >
            <input
              bind:value={renameValue}
              use:focusInput
              aria-label="New name for {node.name}"
              onblur={() => commitRename(node)}
              onkeydown={(e) => {
                if (e.key === 'Escape') {
                  e.preventDefault()
                  renaming = null
                }
              }}
            />
          </form>
        {:else if node.type === 'dir'}
          <button
            class="row"
            oncontextmenu={(e) => {
              e.stopPropagation()
              openMenu(e, node)
            }}
            ondblclick={() => startRename(node)}
            onkeydown={(e) => {
              if (e.key === 'F2') {
                e.preventDefault()
                startRename(node)
              }
            }}
            onclick={() => toggle(node.path)}
          >
            <span class="chevron">{ui.expanded.includes(node.path) ? '▾' : '▸'}</span>
            <span class="icon" aria-hidden="true">▤</span>
            <span class="name">{node.name}</span>
          </button>
          {#if ui.expanded.includes(node.path)}
            {@render tree(node.children ?? [])}
          {/if}
        {:else}
          <button
            class="row"
            class:active={tabs.activeTabId === node.path}
            oncontextmenu={(e) => {
              e.stopPropagation()
              openMenu(e, node)
            }}
            ondblclick={() => startRename(node)}
            onkeydown={(e) => {
              if (e.key === 'F2') {
                e.preventDefault()
                startRename(node)
              }
            }}
            onclick={() => openFile(node)}
          >
            <span class="icon" aria-hidden="true">{iconFor(node.name)}</span>
            <span class="name">{node.name}</span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div class="tree" role="tree" tabindex="0" aria-label="Files" oncontextmenu={(e) => openMenu(e, null)}>
  {#if loading}
    <div class="skeleton" aria-label="Loading files">
      {#each { length: 5 } as _, i (i)}
        <div class="bone" style="width: {65 - i * 7}%"></div>
      {/each}
    </div>
  {:else}
    {#if noRepo}
      <p class="hint">No repository yet. Clone one from the Git panel.</p>
    {:else if error}
      <p class="error">{error}</p>
    {/if}
    {@render tree(ui.fileTree)}
  {/if}
</div>

{#if menu}
  <button class="backdrop" type="button" aria-label="Close menu" onclick={() => (menu = null)}></button>
  <div class="menu" style="left: {menu.x}px; top: {menu.y}px">
    {#if menu.mode === 'menu'}
      <button onclick={() => start('new-file')}>New File</button>
      <button onclick={() => start('new-folder')}>New Folder</button>
      <button onclick={() => importFromPicker(false)}>Import File…</button>
      <button onclick={() => importFromPicker(true)}>Import Folder…</button>
      <button onclick={reveal}>Reveal in File Manager</button>
      {#if menu.node}
        <button onclick={() => (menu = { ...menu!, mode: 'delete' })}>Delete</button>
      {/if}
    {:else if menu.mode === 'delete'}
      <p>Delete {menu.node?.name}?</p>
      <button class="danger" onclick={remove}>Delete</button>
      <button onclick={() => (menu = null)}>Cancel</button>
    {:else}
      <form onsubmit={submitName}>
        <input bind:value={name} placeholder="Name" />
        <button type="submit">OK</button>
      </form>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>
{/if}

<style>
  .tree {
    box-sizing: border-box;
    height: 100%;
    overflow-y: auto;
    padding: 4px 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding-left: 10px;
  }
  .tree > :global(ul) {
    padding-left: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 2px 8px;
    text-align: left;
    border-radius: var(--r-sm);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.active {
    background: var(--selection);
    color: var(--text);
  }
  .rename {
    padding: 0 8px;
  }
  .rename input {
    width: 100%;
    padding: 1px 4px;
  }
  .chevron {
    width: 8px;
    color: var(--muted);
  }
  .icon {
    display: inline-flex;
    justify-content: center;
    width: 18px;
    color: var(--muted);
    font-family: ui-monospace, monospace;
    font-size: 12px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .error {
    margin: 4px 8px;
    color: var(--err);
    font-size: 11px;
  }
  .hint {
    margin: 4px 8px;
    color: var(--muted);
    font-size: 11px;
  }
  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 6px 10px;
  }
  .bone {
    height: 10px;
    border-radius: var(--r-sm);
    background: var(--surface-2);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }
  .backdrop {
    position: fixed;
    inset: 0;
    background: transparent;
  }
  .menu {
    position: fixed;
    z-index: 10;
    min-width: 160px;
    padding: 4px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }
  .menu button {
    display: block;
    width: 100%;
    padding: 5px 8px;
    text-align: left;
    border-radius: var(--r-sm);
  }
  .menu button:hover {
    background: var(--accent);
    color: var(--on-accent);
  }
  .menu .danger {
    color: var(--err);
  }
  .menu p {
    margin: 4px 8px;
    white-space: nowrap;
  }
  .menu form {
    display: flex;
    gap: 4px;
    padding: 4px;
  }
</style>
