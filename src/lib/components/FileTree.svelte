<script lang="ts">
  import { slide } from 'svelte/transition'
  import { prefersReducedMotion } from 'svelte/motion'
  import { invoke } from '@tauri-apps/api/core'
  import { ui, type FileNode } from '../stores/ui.svelte'
  import { tabs, discardUnder, retarget } from '../stores/tabs.svelte'
  import { openFileInTab } from '../utils/open-file'
  import { pickAndImport } from '../utils/import'
  import FileIcon from './FileIcon.svelte'
  import { sortNodes, visibleNodes } from '../utils/file-sort'
  import { errorMessage, isNoRepoError } from '../stores/git.svelte'
  import { pushToast } from '../stores/toasts.svelte'

  type MenuMode = 'menu' | 'delete'

  let loading = $state(true)
  /// The backend stops at 5 000 entries instead of sending a payload that
  /// can take the window down; the tree is real, just partial.
  let truncated = $state(false)
  let noRepo = $state(false)
  let menu = $state<{ x: number; y: number; node: FileNode | null; mode: MenuMode } | null>(null)
  let renaming = $state<string | null>(null)
  let renameValue = $state('')

  /// Creation runs through the palette, so the tree has one flow and not two.
  /// An optional base folder is written into the prompt as a path prefix.
  function createAt(base: string, kind: 'file' | 'folder') {
    menu = null
    // Expanded up front, like the import flow: the new entry must be visible.
    if (base && !ui.expanded.includes(base)) ui.expanded = [...ui.expanded, base]
    ui.palettePrompt = kind
    ui.paletteQuery = base ? `${base}/` : ''
    ui.commandPaletteOpen = true
  }

  /// Opened from the header buttons: the new entry always lands at the root.
  export function createAtRoot(kind: 'file' | 'folder') {
    if (noRepo) return
    createAt('', kind)
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
      pushToast(`Rename failed: ${errorMessage(e)}`)
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
      noRepo = false
    } catch (e) {
      ui.fileTree = []
      truncated = false
      noRepo = isNoRepoError(e)
      if (!noRepo) pushToast(`Could not read the notes folder: ${errorMessage(e)}`)
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
      pushToast(`Could not open ${node.name}: ${errorMessage(e)}`)
    }
  }

  function openMenu(event: MouseEvent, node: FileNode | null) {
    event.preventDefault()
    if (noRepo) return
    menu = { x: event.clientX, y: event.clientY, node, mode: 'menu' }
  }

  /// The folder a context-menu action applies to: the node itself when it is a
  /// directory, otherwise its parent. Root context menus have no node.
  function menuBase(): string {
    const node = menu?.node
    if (!node) return ''
    return node.type === 'dir' ? node.path : parentOf(node.path)
  }

  async function importFromPicker(directory: boolean) {
    if (!menu) return
    const base = menuBase()
    menu = null
    try {
      const path = await pickAndImport(directory, base || null)
      if (!path) return
      if (base && !ui.expanded.includes(base)) ui.expanded = [...ui.expanded, base]
      await loadTree()
      if (!directory) {
        await openFile({ name: path.split('/').pop() ?? path, path, type: 'file' })
      }
    } catch (e) {
      pushToast(`Import failed: ${errorMessage(e)}`)
    }
  }

  async function reveal() {
    if (!menu) return
    const path = menu.node?.path ?? ''
    menu = null
    try {
      await invoke('reveal_in_filemanager', { path })
    } catch (e) {
      pushToast(`Could not reveal the file: ${errorMessage(e)}`)
    }
  }

  async function remove() {
    if (!menu?.node) return
    const path = menu.node.path
    const label = menu.node.name
    try {
      await invoke('fs_delete', { path })
      discardUnder(path)
      menu = null
      await loadTree()
    } catch (e) {
      pushToast(`Could not delete ${label}: ${errorMessage(e)}`)
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
            <span class="chevron" class:open={ui.expanded.includes(node.path)}>
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M6 3.5 10.5 8 6 12.5" />
              </svg>
            </span>
            <span class="icon"><FileIcon name={node.name} dir size={16} /></span>
            <span class="name">{node.name}</span>
          </button>
          {#if ui.expanded.includes(node.path)}
            <div transition:slide={{ duration: prefersReducedMotion.current ? 0 : 120 }}>
              {@render tree(node.children ?? [])}
            </div>
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

<div class="tree" role="group" aria-label="Files" oncontextmenu={(e) => openMenu(e, null)}>
  {#if loading}
    <div class="skeleton" aria-label="Loading files">
      {#each { length: 5 } as _, i (i)}
        <div class="bone" style="width: {65 - i * 7}%"></div>
      {/each}
    </div>
  {:else}
    {#if noRepo}
      <p class="hint">No repository yet. Clone one from the Git panel.</p>
    {/if}
    {@render tree(ui.fileTree)}
    {#if truncated}
      <p class="hint">Showing the first 5 000 entries. Use search to reach the rest.</p>
    {/if}
  {/if}
</div>

{#if menu}
  <button class="backdrop" type="button" aria-label="Close menu" onclick={() => (menu = null)}></button>
  <div class="menu" style="left: {menu.x}px; top: {menu.y}px">
    {#if menu.mode === 'menu'}
      <button onclick={() => createAt(menuBase(), 'file')}>New File</button>
      <button onclick={() => createAt(menuBase(), 'folder')}>New Folder</button>
      <button onclick={() => importFromPicker(false)}>Import File…</button>
      <button onclick={() => importFromPicker(true)}>Import Folder…</button>
      <button onclick={reveal}>Reveal in File Manager</button>
      {#if menu.node}
        <button class="danger" onclick={() => (menu = { ...menu!, mode: 'delete' })}>Delete</button>
      {/if}
    {:else if menu.mode === 'delete'}
      <p>Delete {menu.node?.name}?</p>
      <button class="danger" onclick={remove}>Delete</button>
      <button onclick={() => (menu = null)}>Cancel</button>
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
    /* Inset by 4px a side: the hover/selected fill stops short of the panel
       edge, and margin + padding still add up to the old 8px label offset. */
    width: calc(100% - 8px);
    margin: 0 4px;
    min-height: 24px;
    padding: 0 4px;
    text-align: left;
    border-radius: var(--r-sm);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.active {
    /* A raised surface marks the open file. No rail: the fill alone carries
       the selection, so the label sits at the same x as every other row. */
    background: var(--surface-3);
    color: var(--text);
  }
  /* hc-black aliases surface-3 onto surface-1, which is darker than the hover
     fill, so the open file would read as the least prominent row. */
  :global(:root[data-theme='hc-black']) .row.active {
    background: var(--surface-2);
  }
  .rename {
    padding: 0 8px;
  }
  .rename input {
    width: 100%;
    padding: 1px 4px;
  }
  .chevron {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 12px;
    color: var(--muted);
    transition: transform var(--t-fast);
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .chevron svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
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
    animation: gp-pop-in var(--t-fast) var(--ease-out);
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
</style>
