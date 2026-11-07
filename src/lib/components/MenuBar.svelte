<script lang="ts">
  import { ui } from '../stores/ui.svelte'
  import { tabs, closeTab } from '../stores/tabs.svelte'
  import { getEditor } from '../editor/bridge'
  import { saveAndSync } from '../utils/save-and-sync'

  interface Item {
    label: string
    keys?: string
    run: () => void
    disabled?: boolean
  }

  interface Menu {
    label: string
    groups: Item[][]
  }

  let open = $state<string | null>(null)
  let list = $state<HTMLDivElement>()

  const hasTab = $derived(tabs.tabs.length > 0)

  /// Monaco actions need the editor focused, or the find widget opens unfocused.
  function trigger(action: string) {
    const editor = getEditor()
    editor?.focus()
    editor?.trigger('menu', action, null)
  }

  function openPalette(setup: () => void) {
    setup()
    ui.commandPaletteOpen = true
  }

  const menus = $derived<Menu[]>([
    {
      label: 'File',
      groups: [
        [
          { label: 'New File', keys: 'Ctrl+N', run: () => openPalette(() => (ui.palettePrompt = 'file')) },
          { label: 'New Folder', run: () => openPalette(() => (ui.paletteQuery = '>new folder')) },
          { label: 'Import…', keys: 'Ctrl+O', run: () => openPalette(() => (ui.paletteQuery = '>import')) },
        ],
        [
          { label: 'Save and Sync', keys: 'Ctrl+S', run: () => void saveAndSync(), disabled: !hasTab },
          {
            label: 'Close Tab',
            keys: 'Ctrl+W',
            run: () => closeTab(tabs.activeTabId),
            disabled: !hasTab,
          },
        ],
      ],
    },
    {
      label: 'Edit',
      groups: [
        [
          { label: 'Undo', keys: 'Ctrl+Z', run: () => trigger('undo'), disabled: !hasTab },
          { label: 'Redo', keys: 'Ctrl+Y', run: () => trigger('redo'), disabled: !hasTab },
        ],
        [
          {
            label: 'Cut',
            keys: 'Ctrl+X',
            run: () => trigger('editor.action.clipboardCutAction'),
            disabled: !hasTab,
          },
          {
            label: 'Copy',
            keys: 'Ctrl+C',
            run: () => trigger('editor.action.clipboardCopyAction'),
            disabled: !hasTab,
          },
          {
            label: 'Paste',
            keys: 'Ctrl+V',
            run: () => trigger('editor.action.clipboardPasteAction'),
            disabled: !hasTab,
          },
        ],
        [
          { label: 'Find', keys: 'Ctrl+F', run: () => trigger('actions.find'), disabled: !hasTab },
          {
            label: 'Replace',
            keys: 'Ctrl+R',
            run: () => trigger('editor.action.startFindReplaceAction'),
            disabled: !hasTab,
          },
        ],
        [
          {
            label: 'Find in Files',
            keys: 'Ctrl+Shift+F',
            run: () => openPalette(() => (ui.paletteSearch = true)),
          },
        ],
        [
          {
            label: 'Toggle Line Comment',
            keys: 'Ctrl+/',
            run: () => trigger('editor.action.commentLine'),
            disabled: !hasTab,
          },
          {
            label: 'Toggle Block Comment',
            keys: 'Shift+Alt+A',
            run: () => trigger('editor.action.blockComment'),
            disabled: !hasTab,
          },
        ],
      ],
    },
  ])

  // Focus the first item when a menu opens, so Escape and the arrows work next.
  let focusedFor: string | null = null
  $effect(() => {
    if (!open) {
      focusedFor = null
      return
    }
    if (focusedFor === open) return
    focusedFor = open
    list?.querySelector<HTMLButtonElement>('.item:not(:disabled)')?.focus()
  })

  function close(refocus = false) {
    const label = open
    open = null
    if (refocus && label) {
      const trigger = document.querySelector<HTMLButtonElement>(`[data-menu="${label}"]`)
      trigger?.focus()
    }
  }

  function pick(item: Item) {
    if (item.disabled) return
    close()
    item.run()
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.stopPropagation()
      close(true)
      return
    }
    // Left/right switch menus the way a real menu bar does.
    if ((e.key === 'ArrowRight' || e.key === 'ArrowLeft') && open) {
      e.preventDefault()
      const index = menus.findIndex((menu) => menu.label === open)
      const next = (index + (e.key === 'ArrowRight' ? 1 : -1) + menus.length) % menus.length
      open = menus[next].label
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<nav class="bar" aria-label="Application menu">
  {#each menus as menu (menu.label)}
    <div class="entry">
      <button
        type="button"
        class="trigger"
        class:open={open === menu.label}
        data-menu={menu.label}
        aria-haspopup="menu"
        aria-expanded={open === menu.label}
        onclick={() => (open = open === menu.label ? null : menu.label)}
        onpointerenter={() => {
          if (open) open = menu.label
        }}
      >
        {menu.label}
      </button>
      {#if open === menu.label}
        <!-- Click-anywhere-else closes the menu without stealing the click. -->
        <button class="scrim" aria-label="Close menu" onclick={() => close()}></button>
        <div class="menu" role="menu" aria-label={menu.label} bind:this={list}>
          {#each menu.groups as group, i (i)}
            {#if i > 0}<hr />{/if}
            {#each group as item (item.label)}
              <button
                type="button"
                class="item"
                role="menuitem"
                disabled={item.disabled}
                onclick={() => pick(item)}
              >
                <span class="label">{item.label}</span>
                {#if item.keys}<span class="keys">{item.keys}</span>{/if}
              </button>
            {/each}
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</nav>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: 10px;
  }
  /* Narrow windows: the centred search needs the room more than the menus do. */
  @media (max-width: 560px) {
    .bar {
      display: none;
    }
  }
  .entry {
    position: relative;
  }
  .trigger {
    height: 22px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .trigger:hover,
  .trigger.open {
    background: var(--surface-2);
    color: var(--text);
  }
  .trigger:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    border: 0;
    background: transparent;
  }
  .menu {
    position: absolute;
    top: 26px;
    left: 0;
    z-index: 51;
    min-width: 240px;
    padding: var(--sp-1);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-3);
    box-shadow: var(--shadow-1);
    font-size: var(--fs-sm);
  }
  .menu hr {
    margin: var(--sp-1) 0;
    border: 0;
    border-top: 1px solid var(--border);
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    width: 100%;
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-sm);
    text-align: left;
  }
  .item:hover:not(:disabled),
  .item:focus-visible:not(:disabled) {
    background: var(--accent);
    color: var(--on-accent);
    outline: none;
  }
  .item:disabled {
    color: var(--muted);
    cursor: default;
  }
  .label {
    flex: 1;
  }
  .keys {
    color: var(--muted);
    font-size: var(--fs-xs);
    white-space: nowrap;
  }
  .item:hover:not(:disabled) .keys,
  .item:focus-visible:not(:disabled) .keys {
    color: var(--on-accent);
  }
</style>
