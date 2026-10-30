<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Sidebar from './lib/components/Sidebar.svelte'
  import TabBar from './lib/components/TabBar.svelte'
  import StatusBar from './lib/components/StatusBar.svelte'
  import EditorPane, { flushPendingSave } from './lib/components/EditorPane.svelte'
  import Onboarding from './lib/components/Onboarding.svelte'
  import CommandPalette from './lib/components/CommandPalette.svelte'
  import ConflictModal from './lib/components/ConflictModal.svelte'
  import { ui, cyclePreview, THEMES } from './lib/stores/ui.svelte'
  import {
    applyGitStatus,
    gitStatus,
    isIgnorableSyncError,
    type GitStatus,
  } from './lib/stores/git.svelte'
  import { loadConfig, saveConfig, type Config } from './lib/utils/config'

  let configLoaded = $state(false)
  let onboardingSkipped = $state(false)

  // Persist user-level settings whenever they change; skip the initial load.
  $effect(() => {
    const config: Config = {
      onboardingComplete: ui.onboardingComplete,
      theme: ui.editorTheme,
      previewMode: ui.previewMode,
    }
    if (!configLoaded) return
    void saveConfig(config).catch(() => {})
  })

  // A resolved merge re-arms the conflict modal for the next one.
  $effect(() => {
    if (gitStatus.conflicts.length === 0) ui.conflictsHidden = false
  })

  async function boot() {
    let repoReady = false
    try {
      applyGitStatus(await invoke<GitStatus>('git_status'))
      repoReady = true
    } catch (e) {
      const message = String(e)
      if (isIgnorableSyncError(message)) gitStatus.configured = false
      else gitStatus.error = message
    }
    if (!repoReady) return
    const token = await invoke<string | null>('auth_get', { key: 'github_token' }).catch(() => null)
    if (!token) return
    try {
      applyGitStatus(await invoke<GitStatus>('git_pull'))
    } catch (e) {
      const message = String(e)
      if (!isIgnorableSyncError(message)) gitStatus.error = message
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'p') {
      e.preventDefault()
      cyclePreview()
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
    void listen('app-closing', async () => {
      await flushPendingSave()
      await invoke('app_exit').catch(() => {})
    }).then((fn) => {
      unlistens.push(fn)
    })
    const onFocus = () => {
      ui.treeVersion += 1
      void invoke<GitStatus>('git_status')
        .then(applyGitStatus)
        .catch((e) => {
          if (isIgnorableSyncError(String(e))) gitStatus.configured = false
        })
    }
    window.addEventListener('focus', onFocus)
    void (async () => {
      const config = await loadConfig().catch(() => null)
      if (config) {
        if (THEMES.includes(config.theme)) ui.editorTheme = config.theme
        if (config.previewMode === 'off' || config.previewMode === 'split' || config.previewMode === 'full') {
          ui.previewMode = config.previewMode
        }
        ui.onboardingComplete = config.onboardingComplete
      }
      configLoaded = true
      await boot()
    })()
    return () => {
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
      {#if ui.sidebarOpen}
        <Sidebar />
      {/if}
      <section class="editor">
        <TabBar />
        <EditorPane />
      </section>
    </main>
    <StatusBar />
  </div>
{/if}

{#if ui.commandPaletteOpen}
  <CommandPalette />
{/if}

{#if gitStatus.conflicts.length > 0 && !ui.conflictsHidden}
  <ConflictModal />
{/if}

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
