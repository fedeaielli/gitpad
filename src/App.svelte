<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Sidebar from './lib/components/Sidebar.svelte'
  import TabBar from './lib/components/TabBar.svelte'
  import StatusBar from './lib/components/StatusBar.svelte'
  import EditorPane from './lib/components/EditorPane.svelte'
  import { ui } from './lib/stores/ui.svelte'
  import { applyGitStatus, gitStatus, type GitStatus } from './lib/stores/git.svelte'

  async function boot() {
    let repoReady = false
    try {
      applyGitStatus(await invoke<GitStatus>('git_status'))
      repoReady = true
    } catch (e) {
      gitStatus.error = String(e)
    }
    if (!repoReady) return
    const token = await invoke<string | null>('auth_get', { key: 'github_token' }).catch(() => null)
    if (!token) return
    try {
      applyGitStatus(await invoke<GitStatus>('git_pull'))
    } catch (e) {
      gitStatus.error = String(e)
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined
    void listen<GitStatus>('sync-status', (e) => applyGitStatus(e.payload)).then((fn) => {
      unlisten = fn
    })
    void boot()
    return () => unlisten?.()
  })
</script>

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
