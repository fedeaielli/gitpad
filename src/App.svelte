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
  import { flushPendingSave } from './lib/editor/bridge'
  import Onboarding from './lib/components/Onboarding.svelte'
  import CommandPalette from './lib/components/CommandPalette.svelte'
  import Settings from './lib/components/Settings.svelte'
  import Toasts from './lib/components/Toasts.svelte'
  import { pushToast } from './lib/stores/toasts.svelte'
  import { ui, cyclePreview, THEMES, KEYBINDINGS, type Keybinding } from './lib/stores/ui.svelte'
  import { setCustomLanguages } from './lib/utils/langmap'
  import { applyTheme } from './lib/utils/themes'
  import {
    applyGitStatus,
    gitStatus,
    errorMessage,
    isIgnorableSyncError,
    isNoRepoError,
    type GitStatus,
  } from './lib/stores/git.svelte'
  import { loadConfig, saveConfig, type Config } from './lib/utils/config'

  let configLoaded = $state(false)
  // Monaco is heavy: load it after the shell paints.
  let EditorPane = $state<Component | null>(null)
  let ConflictModal = $state<Component | null>(null)
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
      autoSync: ui.autoSync,
      saveDebounceMs: ui.saveDebounceMs,
      pushIdleMs: ui.pushIdleMs,
      keybindings: ui.keybindings,
      extensions: { ...ui.extensions },
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

  async function boot() {
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
  }

  async function remoteSync() {
    const token = await invoke<string | null>('auth_get', { key: 'github_token' }).catch(() => null)
    if (!token) return
    try {
      applyGitStatus(await invoke<GitStatus>('git_pull'))
    } catch (e) {
      const message = errorMessage(e)
      if (!isIgnorableSyncError(message)) gitStatus.error = message
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'p') {
      e.preventDefault()
      cyclePreview()
      return
    }
    if (ui.settingsOpen) return
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'p') {
      e.preventDefault()
      ui.commandPaletteOpen = true
    }
  }

  onMount(() => {
    void import('./lib/components/EditorPane.svelte').then((m) => (EditorPane = m.default))
    const unlistens: Array<() => void> = []
    void listen<GitStatus>('sync-status', (e) => applyGitStatus(e.payload)).then((fn) => {
      unlistens.push(fn)
    })
    void listen('app-closing', async () => {
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
        ui.autoSync = config.autoSync
        if (Number.isFinite(config.saveDebounceMs)) {
          ui.saveDebounceMs = clamp(config.saveDebounceMs, 500, 10_000)
        }
        if (Number.isFinite(config.pushIdleMs)) ui.pushIdleMs = clamp(config.pushIdleMs, 5000, 300_000)
        if (KEYBINDINGS.includes(config.keybindings as Keybinding)) {
          ui.keybindings = config.keybindings as Keybinding
        }
        if (config.extensions) ui.extensions = config.extensions
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

{#if !configLoaded}
  <!-- wait for config.json so an onboarded user doesn't see the wizard flash -->
{:else if !ui.onboardingComplete && !onboardingSkipped}
  <Onboarding onskip={() => (onboardingSkipped = true)} />
{:else}
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
      </section>
    </main>
    <StatusBar />
  </div>
{/if}

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
  :global(html, body, #app) {
    height: 100%;
    margin: 0;
  }
  :global(:root) {
    --bg: #1e1e1e;
    --bg-panel: #252526;
    --bg-raised: #2d2d30;
    --border: #3c3c3c;
    --text: #d4d4d4;
    --muted: #8a8a8a;
    --accent: #2f6fa8;
    --ok: #4ec9b0;
    --warn: #d7ba7d;
    --err: #f14c4c;
  }
  :global(:root[data-theme='vs']) {
    color-scheme: light;
    --bg: #ffffff;
    --bg-panel: #f3f3f3;
    --bg-raised: #e7e7e7;
    --border: #d4d4d4;
    --text: #1f1f1f;
    --muted: #6b6b6b;
    --accent: #005fb8;
    --ok: #107c10;
    --warn: #9a6700;
    --err: #d13438;
  }
  :global(:root[data-theme='hc-black']) {
    color-scheme: dark;
    --bg: #000000;
    --bg-panel: #0a0a0a;
    --bg-raised: #1a1a1a;
    --border: #6fc3df;
    --text: #ffffff;
    --muted: #d4d4d4;
    --accent: #f38518;
    --ok: #3ff23f;
    --warn: #fff200;
    --err: #ff5252;
  }
  :global(body) {
    overflow: hidden;
    background: var(--bg);
    color: var(--text);
    font: 13px/1.4 system-ui, sans-serif;
  }
  :global(button) {
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  :global(input) {
    padding: 3px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-raised);
    color: inherit;
    font: inherit;
  }
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .editor {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }
</style>
