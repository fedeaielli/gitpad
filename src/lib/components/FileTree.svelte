<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ui, type FileNode } from '../stores/ui.svelte'
  import { tabs, openTab, activateTab, closeTab } from '../stores/tabs.svelte'

  type MenuMode = 'menu' | 'new-file' | 'new-folder' | 'rename' | 'delete'

  let expanded = $state<string[]>([])
  let error = $state<string | null>(null)
  let menu = $state<{ x: number; y: number; node: FileNode | null; mode: MenuMode } | null>(null)
  let name = $state('')

  const ICONS: Record<string, string> = {
    md: 'M',
    markdown: 'M',
    ts: 'TS',
    js: 'JS',
    json: '{}',
    css: '#',
    html: '<>',
    svelte: 'S',
    txt: 'T',
    png: '▣',
    jpg: '▣',
    svg: '◈',
  }

  function iconFor(file: string): string {
    const ext = file.split('.').pop()?.toLowerCase() ?? ''
    return ICONS[ext] ?? '·'
  }

  function toggle(path: string) {
    expanded = expanded.includes(path) ? expanded.filter((p) => p !== path) : [...expanded, path]
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
    } catch (e) {
      ui.fileTree = []
      error = String(e)
    }
  }

  $effect(() => {
    ui.treeVersion
    void loadTree()
  })

  async function openFile(node: FileNode) {
    if (tabs.tabs.some((t) => t.id === node.path)) {
      activateTab(node.path)
      return
    }
    try {
      const content = await invoke<string>('fs_read_file', { path: node.path })
      // language stays plaintext until T3.5 maps extensions to Monaco modes
      openTab({ id: node.path, path: node.path, content, modified: false, language: 'plaintext' })
    } catch (e) {
      error = String(e)
    }
  }

  function openMenu(event: MouseEvent, node: FileNode | null) {
    event.preventDefault()
    menu = { x: event.clientX, y: event.clientY, node, mode: 'menu' }
  }

  function start(mode: MenuMode) {
    if (!menu) return
    name = mode === 'rename' ? (menu.node?.name ?? '') : ''
    menu = { ...menu, mode }
  }

  async function submitName(event: SubmitEvent) {
    event.preventDefault()
    if (!menu || !name.trim()) return
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
      } else if (menu.mode === 'rename' && menu.node) {
        await invoke('fs_rename', { from: menu.node.path, to: join(parentOf(menu.node.path), value) })
      }
      menu = null
      await loadTree()
    } catch (e) {
      error = String(e)
    }
  }

  async function remove() {
    if (!menu?.node) return
    const path = menu.node.path
    try {
      await invoke('fs_delete', { path })
      const tab = tabs.tabs.find((t) => t.path === path || t.path.startsWith(`${path}/`))
      if (tab) closeTab(tab.id)
      menu = null
      await loadTree()
    } catch (e) {
      error = String(e)
    }
  }
</script>

{#snippet tree(nodes: FileNode[])}
  <ul>
    {#each nodes as node (node.path)}
      <li>
        {#if node.type === 'dir'}
          <button
            class="row"
            oncontextmenu={(e) => {
              e.stopPropagation()
              openMenu(e, node)
            }}
            onclick={() => toggle(node.path)}
          >
            <span class="chevron">{expanded.includes(node.path) ? '▾' : '▸'}</span>
            <span class="icon">▤</span>
            <span class="name">{node.name}</span>
          </button>
          {#if expanded.includes(node.path)}
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
            onclick={() => openFile(node)}
          >
            <span class="icon">{iconFor(node.name)}</span>
            <span class="name">{node.name}</span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div class="tree" role="tree" tabindex="0" aria-label="Files" oncontextmenu={(e) => openMenu(e, null)}>
  {#if error}
    <p class="error">{error}</p>
  {/if}
  {@render tree(ui.fileTree)}
</div>

{#if menu}
  <button class="backdrop" type="button" aria-label="Close menu" onclick={() => (menu = null)}></button>
  <div class="menu" style="left: {menu.x}px; top: {menu.y}px">
    {#if menu.mode === 'menu'}
      <button onclick={() => start('new-file')}>New File</button>
      <button onclick={() => start('new-folder')}>New Folder</button>
      {#if menu.node}
        <button onclick={() => start('rename')}>Rename</button>
        <button onclick={() => (menu = { ...menu!, mode: 'delete' })}>Delete</button>
      {/if}
    {:else if menu.mode === 'delete'}
      <p>Delete {menu.node?.name}?</p>
      <button class="danger" onclick={remove}>Delete</button>
      <button onclick={() => (menu = null)}>Cancel</button>
    {:else}
      <form onsubmit={submitName}>
        <input bind:value={name} placeholder={menu.mode === 'rename' ? 'New name' : 'Name'} />
        <button type="submit">OK</button>
      </form>
    {/if}
  </div>
{/if}

<style>
  .tree {
    min-height: 100%;
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
    border-radius: 3px;
  }
  .row:hover {
    background: var(--bg-raised);
  }
  .row.active {
    background: var(--bg-raised);
    color: #fff;
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
    font-size: 10px;
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
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }
  .menu button {
    display: block;
    width: 100%;
    padding: 5px 8px;
    text-align: left;
    border-radius: 3px;
  }
  .menu button:hover {
    background: var(--accent);
    color: #fff;
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
