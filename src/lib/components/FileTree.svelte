<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ui, type FileNode } from '../stores/ui.svelte'
  import { tabs, discardUnder, retarget } from '../stores/tabs.svelte'
  import { openFileInTab } from '../utils/open-file'
  import { pickAndImport } from '../utils/import'
  import FileIcon from './FileIcon.svelte'
  import { sortNodes, visibleNodes } from '../utils/file-sort'
  import { errorMessage, isNoRepoError } from '../stores/git.svelte'

  type MenuMode = 'menu' | 'new-file' | 'new-folder' | 'delete'

  let error = $state<string | null>(null)
  let loading = $state(true)
  /// The backend stops at 20 000 entries instead of sending a payload that
  /// can take the window down; the tree is real, just partial.
  let truncated = $state(false)
  let noRepo = $state(false)
  let menu = $state<{ x: number; y: number; node: FileNode | null; mode: MenuMode } | null>(null)
  let name = $state('')
  let renaming = $state<string | null>(null)
  let renameValue = $state('')
  let creating = $state<'file' | 'folder' | null>(null)

  /// Opened from the header buttons: the new entry always lands at the root.
  export function createAtRoot(kind: 'file' | 'folder') {
    if (noRepo) return
    menu = null
    name = ''
    error = null
    creating = kind
  }

  /// Dot files stay hidden unless the options menu enables them.
  function visible(nodes: FileNode[]): FileNode[] {
    return visibleNodes(nodes, ui.showHidden)
  }

  function sorted(nodes: FileNode[]): FileNode[] {
    return sortNodes(visible(nodes), ui.sortMode, ui.sortDesc, ui.foldersFirst)
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
      const listing = await invoke<{ entries: FileNode[]; truncated: boolean }>('fs_list_dir', {
        path: null,
      })
      ui.fileTree = listing.entries
      truncated = listing.truncated
      error = null
      noRepo = false
    } catch (e) {
      ui.fileTree = []
      truncated = false
      noRepo = isNoRepoError(e)
      error = noRepo ? null : errorMessage(e)
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

  async function createIn(base: string, kind: MenuMode, value: string) {
    if (kind === 'new-file') {
      const target = join(base, value)
      await invoke('fs_write_file', { path: target, content: '' })
      await loadTree()
      await openFile({ name: value, path: target, type: 'file' })
      return
    }
    await invoke('fs_create_dir', { path: join(base, value) })
    await loadTree()
  }

  async function submitName(event: SubmitEvent) {
    event.preventDefault()
    if (!menu || !name.trim()) return
    error = null
    const value = name.trim()
    const kind = menu.mode
    const base = menu.node ? (menu.node.type === 'dir' ? menu.node.path : parentOf(menu.node.path)) : ''
    try {
      await createIn(base, kind, value)
      menu = null
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function submitCreate(event: SubmitEvent) {
    event.preventDefault()
    if (!creating || !name.trim()) return
    error = null
    const kind = creating === 'file' ? 'new-file' : 'new-folder'
    try {
      await createIn('', kind, name.trim())
      creating = null
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
            <span class="icon"><FileIcon name={node.name} dir size={16} /></span>
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
            <span class="icon"><FileIcon name={node.name} size={16} /></span>
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
    {#if creating}
      <form class="create" onsubmit={submitCreate}>
        <input
          bind:value={name}
          use:focusInput
          placeholder={creating === 'file' ? 'File name' : 'Folder name'}
          aria-label={creating === 'file' ? 'New file name' : 'New folder name'}
          onkeydown={(e) => {
            if (e.key === 'Escape') {
              e.preventDefault()
              creating = null
            }
          }}
        />
      </form>
    {/if}
    {@render tree(ui.fileTree)}
    {#if truncated}
      <p class="hint">Folder too large: only part of it is listed.</p>
    {/if}
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
    padding: 6px 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding-left: 12px;
  }
  .tree > :global(ul) {
    padding-left: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: 26px;
    padding: 0 8px;
    text-align: left;
    border-radius: var(--r-sm);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.active {
    /* Follows the accent, at a strength that keeps --text above 4.5:1. */
    background: var(--selection);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
    color: var(--text);
  }
  .rename {
    padding: 0 8px;
  }
  .rename input {
    width: 100%;
    padding: 1px 4px;
  }
  .create {
    padding: 2px 8px;
  }
  .create input {
    box-sizing: border-box;
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
    width: 16px;
    color: var(--muted);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .error {
    margin: 4px 8px;
    color: var(--err);
    font-size: var(--fs-xs);
  }
  .hint {
    margin: 4px 8px;
    color: var(--muted);
    font-size: var(--fs-xs);
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
    box-shadow: var(--shadow-1);
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
