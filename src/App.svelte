<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import Sidebar from './lib/components/Sidebar.svelte'
  import ActivityBar from './lib/components/ActivityBar.svelte'
  import TabBar from './lib/components/TabBar.svelte'
  import Breadcrumb from './lib/components/Breadcrumb.svelte'
  import StatusBar from './lib/components/StatusBar.svelte'
  import type { Component } from 'svelte'
  import { flushPendingSave, getEditor, reloadOpenTabs } from './lib/editor/bridge'
  import Onboarding from './lib/components/Onboarding.svelte'
  import CommandPalette from './lib/components/CommandPalette.svelte'
  import Settings from './lib/components/Settings.svelte'
  import Shortcuts from './lib/components/Shortcuts.svelte'
  import Toasts from './lib/components/Toasts.svelte'
  import ConfirmDialog from './lib/components/ConfirmDialog.svelte'
  import TitleBar from './lib/components/TitleBar.svelte'
  import { pushToast } from './lib/stores/toasts.svelte'
  import { confirmDialog, confirmState } from './lib/stores/confirm.svelte'
  import { tip } from './lib/utils/tooltip'
  import { saveAndSync } from './lib/utils/save-and-sync'
  import { ui, cyclePreview, THEMES, KEYBINDINGS, type Keybinding } from './lib/stores/ui.svelte'
  import { tabs, closeTab } from './lib/stores/tabs.svelte'
  import { setCustomLanguages } from './lib/utils/langmap'
  import { applyTheme, applyAccent, applyUiFont, cleanFamily } from './lib/utils/themes'
  import {
    applyGitStatus,
    gitStatus,
    errorMessage,
    isIgnorableSyncError,
    isNoRepoError,
    type FileChange,
    type GitStatus,
  } from './lib/stores/git.svelte'
  import { loadConfig, saveConfig, type Config } from './lib/utils/config'
  import { clampTimelineHeight } from './lib/utils/timeline'

  let configLoaded = $state(false)
  // Monaco is heavy: load it after the shell paints.
  let EditorPane = $state<Component | null>(null)
  let FileTimeline = $state<Component | null>(null)
  let ConflictModal = $state<Component | null>(null)
  let editorColumn = $state<HTMLDivElement>()
  let timelineSized = false

  // Preload the editor chunk right away: the window is still hidden, so parsing
  // Monaco does not compete with the first paint. Opening a tab does the same
  // import, so a race just resolves to the same module.
  $effect(() => {
    if (EditorPane) return
    void import('./lib/components/EditorPane.svelte')
      .then((m) => {
        sessionStorage.removeItem('gitpad:chunk-reload')
        EditorPane = m.default
      })
      .catch(() => reloadOnce())
  })

  // The history panel is its own frame: load it only when it opens.
  $effect(() => {
    if (!ui.timelineOpen || FileTimeline) return
    void import('./lib/components/FileTimeline.svelte')
      .then((m) => {
        sessionStorage.removeItem('gitpad:chunk-reload')
        FileTimeline = m.default
      })
      .catch(() => reloadOnce())
  })

  // 40% on the first open; after that the dragged height is kept.
  $effect(() => {
    if (!ui.timelineOpen || timelineSized || !editorColumn) return
    timelineSized = true
    ui.timelineHeight = clampTimelineHeight(
      editorColumn.clientHeight,
      Math.round(editorColumn.clientHeight * 0.4),
    )
  })

  // The stored height is in pixels: keep it inside the column when it shrinks.
  $effect(() => {
    const column = editorColumn
    if (!column) return
    const observer = new ResizeObserver(() => {
      ui.timelineHeight = clampTimelineHeight(column.clientHeight, ui.timelineHeight)
    })
    observer.observe(column)
    return () => observer.disconnect()
  })
  let saveTimer: ReturnType<typeof setTimeout> | undefined
  let pendingConfig: Config | null = null

  // Dragging the gap between the left panel and the editor resizes the sidebar.
  const MIN_SIDEBAR = 150
  /// The editor always keeps this much room, so the sidebar cannot swallow it.
  const MIN_EDITOR = 260
  const maxSidebar = () => Math.max(MIN_SIDEBAR, window.innerWidth - MIN_EDITOR)
  // The wrapper declares its size: WebKit does not recompute a flex-basis:auto
  // wrapper when the panel inside it changes width, so the panel was clipped.
  const leftWidth = $derived(
    ui.sidebarOpen
      ? `calc(var(--w-activitybar) + ${ui.sidebarWidth}px)`
      : 'var(--w-activitybar)',
  )
  let resizing = $state(false)
  let resizeX = 0
  let resizeWidth = 0

  function startResize(e: PointerEvent) {
    if (!ui.sidebarOpen || e.button !== 0) return
    // Without this the drag selects the text of the panel under the cursor.
    e.preventDefault()
    resizeX = e.clientX
    resizeWidth = ui.sidebarWidth
    resizing = true
  }
  function moveResize(e: PointerEvent) {
    if (!resizing) return
    // Rounded: config_save takes a u32, and a fractional width fails the save.
    ui.sidebarWidth = Math.round(
      Math.min(maxSidebar(), Math.max(MIN_SIDEBAR, resizeWidth + e.clientX - resizeX)),
    )
  }
  function endResize() {
    resizing = false
  }
  /// Shrinking the window must not let the saved width swallow the editor.
  function clampSidebar() {
    if (ui.sidebarWidth > maxSidebar()) ui.sidebarWidth = Math.round(maxSidebar())
  }

  // Persist user-level settings whenever they change; skip the initial load.
  $effect(() => {
    const config: Config = {
      onboardingComplete: ui.onboardingComplete,
      theme: ui.editorTheme,
      previewMode: ui.previewMode,
      fontSize: ui.fontSize,
      fontFamily: ui.fontFamily,
      sidebarWidth: ui.sidebarWidth,
      sidebarOpen: ui.sidebarOpen,
      autoSync: ui.autoSync,
      saveDebounceMs: ui.saveDebounceMs,
      pushIdleMs: ui.pushIdleMs,
      keybindings: ui.keybindings,
      extensions: { ...ui.extensions },
      accent: ui.accent,
      sortMode: ui.sortMode,
      sortDesc: ui.sortDesc,
      foldersFirst: ui.foldersFirst,
      showHidden: ui.showHidden,
      repoPath: ui.repoPath,
    }
    if (!configLoaded) return
    pendingConfig = config
    clearTimeout(saveTimer)
    saveTimer = setTimeout(() => void saveConfig(config).catch((e) => pushToast(`save settings: ${errorMessage(e)}`, 'error', 'app')), 300)
  })

  // Settings > Supported extensions override the built-in language map.
  $effect(() => {
    setCustomLanguages(ui.extensions)
  })

  $effect(() => {
    applyTheme(ui.editorTheme)
    // The accent tokens depend on the theme's surface, so re-derive on both.
    applyAccent(ui.accent)
  })

  $effect(() => {
    applyUiFont(ui.fontFamily)
  })

  // A resolved merge re-arms the conflict modal for the next one.
  $effect(() => {
    if (gitStatus.conflicts.length === 0) ui.conflictsHidden = false
    else if (!ConflictModal) {
      void import('./lib/components/ConflictModal.svelte')
        .then((m) => {
          sessionStorage.removeItem('gitpad:chunk-reload')
          ConflictModal = m.default
        })
        .catch(() => reloadOnce())
    }
  })

  // A failed chunk import leaves the pane blank. In dev a rebuild renames the
  // chunk while the page still asks for the old one: reload once, then stop.
  function reloadOnce() {
    if (sessionStorage.getItem('gitpad:chunk-reload')) return
    sessionStorage.setItem('gitpad:chunk-reload', '1')
    location.reload()
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, Math.round(value)))
  }

  let booted = false

  async function boot() {
    try {
      let repoReady = false
      try {
        applyGitStatus(await invoke<GitStatus>('git_status'))
        repoReady = true
      } catch (e) {
        const message = errorMessage(e)
        if (isNoRepoError(message)) gitStatus.configured = false
        else gitStatus.error = message
      }
      if (!repoReady) return
      // Network work waits for first paint; the local status is enough to render.
      setTimeout(() => void remoteSync(), 1200)
    } finally {
      booted = true
    }
  }

  async function remoteSync() {
    if (gitStatus.conflicts.length > 0) return
    const hasToken = await invoke<boolean>('auth_has_token').catch((e) => {
      const message = errorMessage(e)
      if (!isIgnorableSyncError(message)) gitStatus.error = message
      return false
    })
    if (!hasToken) return
    try {
      applyGitStatus(await invoke<GitStatus>('git_pull'))
    } catch (e) {
      const message = errorMessage(e)
      if (!isIgnorableSyncError(message)) gitStatus.error = message
    }
  }

  /// In Vim/Emacs the editor keeps Ctrl+N and Ctrl+O while it has focus.
  function editorOwnsKey(): boolean {
    return ui.keybindings !== 'vscode' && !!document.activeElement?.closest('.monaco-editor')
  }

  function findWidgetOpen(): boolean {
    return !!document.querySelector('.find-widget.visible')
  }

  function onKeydown(e: KeyboardEvent) {
    // A confirmation owns the keyboard: let it handle Escape/Enter/Tab, but block
    // global shortcuts and WebKit defaults (Ctrl+W closes the window).
    if (confirmState.current) {
      if (e.ctrlKey || e.metaKey || e.altKey) e.preventDefault()
      return
    }
    // Ctrl+S is always global: it must never reach the webview's save-page dialog.
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 's') {
      e.preventDefault()
      e.stopPropagation()
      if (e.repeat) return
      void saveAndSync()
      return
    }
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'p') {
      e.preventDefault()
      cyclePreview()
      return
    }
    // WebKit would close the window on Ctrl+W, so this must never reach it.
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'w') {
      e.preventDefault()
      if (e.repeat) return
      if (tabs.activeTabId) closeTab(tabs.activeTabId)
      return
    }
    if (e.ctrlKey && e.shiftKey && !e.altKey && e.key.toLowerCase() === 'f') {
      e.preventDefault()
      if (e.repeat) return
      ui.paletteSearch = true
      ui.commandPaletteOpen = true
      return
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'f') {
      // Monaco's own Ctrl+F only refocuses the widget; this toggles it.
      e.preventDefault()
      // Vim and Emacs own Ctrl+F (page down / forward char) in the editor.
      if (editorOwnsKey()) return
      e.stopPropagation()
      if (e.repeat) return
      if (ui.settingsOpen || ui.shortcutsOpen || ui.commandPaletteOpen) return
      if (!tabs.activeTabId) return
      const editor = getEditor()
      if (findWidgetOpen()) {
        editor?.trigger('menu', 'closeFindWidget', null)
        return
      }
      editor?.focus()
      editor?.trigger('menu', 'actions.find', null)
      return
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'r') {
      // WebKit would reload the webview, so this must never reach it.
      e.preventDefault()
      // Vim and Emacs own Ctrl+R (redo / isearch) while the editor has focus.
      if (editorOwnsKey()) return
      e.stopPropagation()
      if (e.repeat) return
      if (ui.settingsOpen || ui.shortcutsOpen) return
      if (!tabs.activeTabId) return
      const editor = getEditor()
      if (findWidgetOpen()) {
        editor?.trigger('menu', 'closeFindWidget', null)
        return
      }
      editor?.focus()
      editor?.trigger('menu', 'editor.action.startFindReplaceAction', null)
      return
    }
    if (ui.settingsOpen) return
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'k') {
      e.preventDefault()
      if (e.repeat) return
      // Never stack the two modals.
      ui.commandPaletteOpen = false
      ui.shortcutsOpen = !ui.shortcutsOpen
      return
    }
    if (ui.shortcutsOpen) return
    // Escape closes the file history, but not while the palette is on top of it.
    if (e.key === 'Escape' && ui.timelineOpen && !ui.commandPaletteOpen) {
      ui.timelineOpen = false
      ui.timelineCommit = null
      return
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'h') {
      e.preventDefault()
      // Monaco's own Ctrl+H opens replace; stop it in the capture phase so the
      // history shortcut never opens the find widget behind it.
      e.stopPropagation()
      if (e.repeat) return
      // Ctrl+H always means file history: from a commit diff it switches panels
      // instead of closing one, so the key never feels like a no-op.
      const fromCommit = ui.timelineCommit !== null
      ui.timelineCommit = null
      // The panel only exists for an open file, so the key does nothing without one.
      if (tabs.activeTabId) ui.timelineOpen = fromCommit || !ui.timelineOpen
      else ui.timelineOpen = false
      return
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'n') {
      // Vim and Emacs own Ctrl+N (completion / next line) while the editor has focus.
      if (editorOwnsKey()) return
      e.preventDefault()
      if (e.repeat) return
      ui.palettePrompt = 'file'
      ui.commandPaletteOpen = true
      return
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'o') {
      // Vim and Emacs own Ctrl+O (jump back / open line) while the editor has focus.
      if (editorOwnsKey()) return
      // Always preventDefault: WebKit would otherwise offer to open a local file.
      e.preventDefault()
      if (e.repeat) return
      ui.paletteQuery = '>import'
      ui.commandPaletteOpen = true
      return
    }
    // Comment toggles: Monaco only ships them as Ctrl+K chords, which the
    // shortcut list already owns, so they get their own keys here.
    if (
      (e.ctrlKey && !e.shiftKey && !e.altKey && e.key === '/') ||
      (e.shiftKey && e.altKey && !e.ctrlKey && e.key.toLowerCase() === 'a')
    ) {
      // Typing in a field must not toggle a comment behind the dialog.
      if (document.activeElement?.closest('input, textarea, .find-widget')) return
      e.preventDefault()
      if (e.repeat) return
      const editor = getEditor()
      editor?.focus()
      editor?.trigger(
        'menu',
        e.key === '/' ? 'editor.action.commentLine' : 'editor.action.blockComment',
        null,
      )
      return
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'p') {
      e.preventDefault()
      if (e.repeat) return
      // Ctrl+P is the palette's own toggle; the other keys open it with content.
      ui.commandPaletteOpen = !ui.commandPaletteOpen
    }
  }

  onMount(() => {
    const unlistens: Array<() => void> = []
    void listen<GitStatus>('sync-status', (e) => applyGitStatus(e.payload)).then((fn) => {
      unlistens.push(fn)
    })
    void listen<FileChange[]>('pull-changed', (e) => void reloadOpenTabs(e.payload)).then((fn) => {
      unlistens.push(fn)
    })
    void listen('app-closing', async () => {
      // Ack first: proves this webview is alive, so the 5 s watchdog stands down.
      void invoke('app_closing_ack').catch(() => {})
      const saved = await flushPendingSave()
      if (!saved) {
        const exit = await confirmDialog({
          title: 'Unsaved changes',
          message: 'Some changes could not be saved. Exit anyway?',
          kind: 'warning',
          confirmLabel: 'Exit',
        })
        if (!exit) return
      }
      // The window disappears once the local writes are done; the exit commit
      // and push (up to 3 s) finish behind a hidden window, then the app quits.
      ui.quitting = true
      await getCurrentWindow().hide().catch(() => {})
      clearTimeout(saveTimer)
      if (pendingConfig) await saveConfig(pendingConfig).catch(() => {})
      await invoke('app_exit').catch(async () => {
        // If the exit command fails, do not leave an invisible process behind.
        ui.quitting = false
        await getCurrentWindow().show().catch(() => {})
      })
    }).then((fn) => {
      unlistens.push(fn)
    })
    const onFocus = () => {
      // The focus fired by the first show() would duplicate boot()'s work.
      if (!booted) return
      ui.treeVersion += 1
      void invoke<GitStatus>('git_status')
        .then(applyGitStatus)
        .catch((e) => {
          const message = errorMessage(e)
          if (isNoRepoError(message)) gitStatus.configured = false
          else pushToast(message, 'error', 'git')
        })
    }
    window.addEventListener('focus', onFocus)
    // Hidden until the first render; the timer keeps a JS failure from hiding the app forever.
    const showTimer = setTimeout(() => void getCurrentWindow().show().catch(() => {}), 1500)
    // A token kept outside the keyring is worth one warning, at startup.
    void invoke<boolean>('auth_keyring_ok')
      .then((ok) => {
        if (!ok) {
          pushToast(
            'Keyring unavailable: the GitHub token is kept in a protected file.',
            'warning',
            'app',
          )
        }
      })
      .catch(() => {})
    void (async () => {
      const config = await loadConfig().catch((e) => {
        pushToast(`load settings: ${errorMessage(e)}`, 'error', 'app')
        return null
      })
      if (config) {
        if (THEMES.includes(config.theme)) ui.editorTheme = config.theme
        if (config.previewMode === 'off' || config.previewMode === 'split' || config.previewMode === 'full') {
          ui.previewMode = config.previewMode
        }
        ui.onboardingComplete = config.onboardingComplete
        if (Number.isFinite(config.fontSize)) ui.fontSize = clamp(config.fontSize, 8, 32)
        if (config.fontFamily) ui.fontFamily = cleanFamily(config.fontFamily)
        if (Number.isFinite(config.sidebarWidth))
          ui.sidebarWidth = clamp(config.sidebarWidth, MIN_SIDEBAR, maxSidebar())
        if (typeof config.sidebarOpen === 'boolean') ui.sidebarOpen = config.sidebarOpen
        ui.autoSync = config.autoSync
        ui.accent =
          typeof config.accent === 'string' && /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(config.accent)
            ? config.accent
            : ''
        if (Number.isFinite(config.saveDebounceMs)) {
          ui.saveDebounceMs = clamp(config.saveDebounceMs, 500, 10_000)
        }
        if (Number.isFinite(config.pushIdleMs)) ui.pushIdleMs = clamp(config.pushIdleMs, 5000, 300_000)
        if (KEYBINDINGS.includes(config.keybindings as Keybinding)) {
          ui.keybindings = config.keybindings as Keybinding
        }
        if (config.extensions) ui.extensions = config.extensions
        if (typeof config.foldersFirst === 'boolean') ui.foldersFirst = config.foldersFirst
        const mode = config.sortMode
        // Configs written before the split stored folders-first as a sort mode.
        if (mode === 'folders') ui.foldersFirst = true
        if (mode === 'name' || mode === 'folders' || mode === 'modified' || mode === 'type') {
          ui.sortMode = mode === 'folders' ? 'name' : mode
        }
        if (typeof config.sortDesc === 'boolean') ui.sortDesc = config.sortDesc
        if (typeof config.showHidden === 'boolean') ui.showHidden = config.showHidden
        if (typeof config.repoPath === 'string') ui.repoPath = config.repoPath
      }
      configLoaded = true
      clearTimeout(showTimer)
      await tick()
      await getCurrentWindow().show().catch(() => {})
      await boot()
    })()
    return () => {
      clearTimeout(showTimer)
      window.removeEventListener('focus', onFocus)
      unlistens.forEach((fn) => fn())
    }
  })
</script>

<svelte:window
  onkeydowncapture={onKeydown}
  onpointermove={moveResize}
  onpointerup={endResize}
  onpointercancel={endResize}
  onresize={clampSidebar}
/>

<div class="shell">
  <TitleBar />
  <div class="app">
    <main class="body">
      <div class="left" style="flex: 0 0 {leftWidth}">
        <ActivityBar />
        {#if ui.sidebarOpen}
          <Sidebar />
        {/if}
      </div>
      <div
        class="gutter"
        class:open={ui.sidebarOpen}
        class:active={resizing}
        role={ui.sidebarOpen ? 'separator' : 'presentation'}
        aria-orientation={ui.sidebarOpen ? 'vertical' : undefined}
        aria-label="Resize sidebar"
        aria-valuemin={MIN_SIDEBAR}
        aria-valuemax={Math.round(maxSidebar())}
        aria-valuenow={Math.round(ui.sidebarWidth)}
        use:tip={`${Math.round(ui.sidebarWidth)}px`}
        onpointerdown={startResize}
      ></div>
      <div class="editor-column" bind:this={editorColumn}>
        <section class="editor">
          <TabBar />
          <Breadcrumb />
          {#if EditorPane}
            <EditorPane />
          {/if}
          {#if tabs.tabs.length === 0 && !ui.timelineCommit}
            <div class="empty">
              <p class="empty-title">No file open</p>
              <p class="empty-hint">
                <kbd>Ctrl</kbd>+<kbd>N</kbd> to create a file · <kbd>Ctrl</kbd>+<kbd>O</kbd> to
                import
              </p>
            </div>
          {/if}
        </section>
        {#if ui.timelineOpen && (ui.timelineCommit || tabs.activeTabId)}
          {#if FileTimeline}
            <FileTimeline />
          {/if}
        {/if}
      </div>
    </main>
    <StatusBar />
  </div>
  {#if configLoaded && !ui.onboardingComplete}
    <!-- Gated on config.json so an onboarded user doesn't see the wizard flash. -->
    <!-- Skipping is a choice: it is saved, and Settings is where to change it. -->
    <Onboarding onskip={() => (ui.onboardingComplete = true)} />
  {/if}
</div>

{#if ui.commandPaletteOpen}
  <CommandPalette />
{/if}

{#if gitStatus.conflicts.length > 0 && !ui.conflictsHidden && ConflictModal}
  <ConflictModal />
{/if}

{#if ui.settingsOpen}
  <Settings />
{/if}

{#if ui.shortcutsOpen}
  <Shortcuts />
{/if}

<Toasts />
<ConfirmDialog />

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-base);
  }
  .app {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }
  .body {
    display: flex;
    flex: 1;
    gap: 0;
    min-height: 0;
    padding: var(--sp-1);
  }
  .left {
    display: flex;
    box-sizing: border-box;
    /* The inline flex-basis carries the sidebar width; see leftWidth. */
    flex: 0 0 auto;
    min-width: 0;
    /* One panel: the rail and the sidebar share the outer rounding. */
    border-radius: var(--r-lg);
    border: 1px solid var(--border);
    border-right: 0;
    overflow: hidden;
    background: var(--surface-1);
  }
  /* The gap between the panels, made grabbable: it is the only place where the
     sidebar edge can be dragged without fighting the panel content. */
  .gutter {
    position: relative;
    z-index: 1;
    flex: 0 0 var(--sp-1);
    touch-action: none;
  }
  /* 6px is too thin to hit, so the grab area leans into the sidebar. */
  .gutter::before {
    position: absolute;
    inset: 0 -4px 0 -10px;
    content: '';
  }
  .gutter.open {
    cursor: col-resize;
  }
  .gutter.open:hover {
    background: var(--border-strong);
  }
  /* Accent only while dragging, so the two states are told apart at a glance.
     Two classes to match :hover, which is still on during the drag. */
  .gutter.open.active {
    background: var(--accent);
  }
  .editor-column {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--sp-2);
    min-width: 0;
    min-height: 0;
  }
  .editor {
    position: relative;
    display: flex;
    box-sizing: border-box;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    /* Clipped so Monaco cannot paint a square corner over the radius. */
    border-radius: var(--r-lg);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .empty {
    position: absolute;
    /* Below the tab bar, so the strip stays visible. */
    inset: var(--h-tabbar) 0 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-2);
    background: var(--surface-0);
    color: var(--muted);
  }
  .empty-title {
    margin: 0;
    color: var(--text);
    font-size: var(--fs-lg);
  }
  .empty-hint {
    margin: 0;
    font-size: var(--fs-sm);
  }
  .empty kbd {
    padding: 1px 5px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-1);
    color: var(--text);
    font-family: inherit;
    font-size: var(--fs-xs);
  }
</style>
