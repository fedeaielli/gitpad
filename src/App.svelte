<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { ask } from '@tauri-apps/plugin-dialog'
  import Sidebar from './lib/components/Sidebar.svelte'
  import ActivityBar from './lib/components/ActivityBar.svelte'
  import TabBar from './lib/components/TabBar.svelte'
  import StatusBar from './lib/components/StatusBar.svelte'
  import type { Component } from 'svelte'
  import { flushPendingSave, reloadOpenTabs } from './lib/editor/bridge'
  import Onboarding from './lib/components/Onboarding.svelte'
  import CommandPalette from './lib/components/CommandPalette.svelte'
  import Settings from './lib/components/Settings.svelte'
  import Toasts from './lib/components/Toasts.svelte'
  import TitleBar from './lib/components/TitleBar.svelte'
  import { pushToast } from './lib/stores/toasts.svelte'
  import { saveAndSync } from './lib/utils/save-and-sync'
  import { ui, cyclePreview, THEMES, KEYBINDINGS, type Keybinding } from './lib/stores/ui.svelte'
  import { tabs } from './lib/stores/tabs.svelte'
  import { setCustomLanguages } from './lib/utils/langmap'
  import { applyTheme, applyAccent } from './lib/utils/themes'
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

  let configLoaded = $state(false)
  // Monaco is heavy: load it after the shell paints.
  let EditorPane = $state<Component | null>(null)
  let ConflictModal = $state<Component | null>(null)

  // Preload the editor chunk right away: the window is still hidden, so parsing
  // Monaco does not compete with the first paint. Opening a tab does the same
  // import, so a race just resolves to the same module.
  $effect(() => {
    if (EditorPane) return
    void import('./lib/components/EditorPane.svelte').then((m) => (EditorPane = m.default))
  })
  let saveTimer: ReturnType<typeof setTimeout> | undefined
  let pendingConfig: Config | null = null
  let onboardingSkipped = $state(false)

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
      showHidden: ui.showHidden,
    }
    if (!configLoaded) return
    pendingConfig = config
    clearTimeout(saveTimer)
    saveTimer = setTimeout(() => void saveConfig(config).catch((e) => pushToast(`save settings: ${errorMessage(e)}`)), 300)
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

  // A resolved merge re-arms the conflict modal for the next one.
  $effect(() => {
    if (gitStatus.conflicts.length === 0) ui.conflictsHidden = false
    else if (!ConflictModal) {
      void import('./lib/components/ConflictModal.svelte').then((m) => (ConflictModal = m.default))
    }
  })

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

  function onKeydown(e: KeyboardEvent) {
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
    if (ui.settingsOpen) return
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
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'p') {
      e.preventDefault()
      ui.commandPaletteOpen = true
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
        const exit = await ask('Some changes could not be saved. Exit anyway?', {
          title: 'Unsaved changes',
          kind: 'warning',
        }).catch(() => false)
        if (!exit) return
      }
      clearTimeout(saveTimer)
      if (pendingConfig) await saveConfig(pendingConfig).catch(() => {})
      await invoke('app_exit').catch(() => {})
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
          else pushToast(message)
        })
    }
    window.addEventListener('focus', onFocus)
    // Hidden until the first render; the timer keeps a JS failure from hiding the app forever.
    const showTimer = setTimeout(() => void getCurrentWindow().show().catch(() => {}), 1500)
    void (async () => {
      const config = await loadConfig().catch((e) => {
        pushToast(`load settings: ${errorMessage(e)}`)
        return null
      })
      if (config) {
        if (THEMES.includes(config.theme)) ui.editorTheme = config.theme
        if (config.previewMode === 'off' || config.previewMode === 'split' || config.previewMode === 'full') {
          ui.previewMode = config.previewMode
        }
        ui.onboardingComplete = config.onboardingComplete
        if (Number.isFinite(config.fontSize)) ui.fontSize = clamp(config.fontSize, 8, 32)
        if (config.fontFamily) ui.fontFamily = config.fontFamily
        if (Number.isFinite(config.sidebarWidth)) ui.sidebarWidth = clamp(config.sidebarWidth, 150, 500)
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
        if (config.sortMode === 'name' || config.sortMode === 'folders' || config.sortMode === 'modified') {
          ui.sortMode = config.sortMode
        }
        if (typeof config.sortDesc === 'boolean') ui.sortDesc = config.sortDesc
        if (typeof config.showHidden === 'boolean') ui.showHidden = config.showHidden
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

<svelte:window onkeydowncapture={onKeydown} />

<div class="shell">
  <TitleBar />
  <div class="app">
    <main class="body">
      <ActivityBar />
      {#if ui.sidebarOpen}
        <Sidebar />
      {/if}
      <section class="editor">
        <TabBar />
        {#if EditorPane}
          <EditorPane />
        {/if}
        {#if tabs.tabs.length === 0}
          <div class="empty">
            <p class="empty-title">No file open</p>
            <p class="empty-hint">
              <kbd>Ctrl</kbd>+<kbd>N</kbd> to create a file · <kbd>Ctrl</kbd>+<kbd>O</kbd> to import
            </p>
          </div>
        {/if}
      </section>
    </main>
    <StatusBar />
  </div>
  {#if configLoaded && !ui.onboardingComplete && !onboardingSkipped}
    <!-- Gated on config.json so an onboarded user doesn't see the wizard flash. -->
    <Onboarding onskip={() => (onboardingSkipped = true)} />
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

<Toasts />

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
    gap: var(--sp-1);
    min-height: 0;
    padding: var(--sp-1);
  }
  .editor {
    position: relative;
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
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
