<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import * as monaco from 'monaco-editor'
  import { errorMessage, gitStatus, syncNow } from '../stores/git.svelte'
  import { ui } from '../stores/ui.svelte'
  import { tabs, openTab } from '../stores/tabs.svelte'
  import { languageFor } from '../utils/langmap'
  import { monacoTheme } from '../utils/themes'

  interface Versions {
    ours: string
    theirs: string
  }

  let files = $state(gitStatus.conflicts.map((c) => c.path))
  let selected = $state(gitStatus.conflicts[0]?.path ?? '')
  let oursModel: monaco.editor.ITextModel | undefined
  let theirsModel: monaco.editor.ITextModel | undefined
  let diff: monaco.editor.IStandaloneDiffEditor | undefined
  let host = $state<HTMLDivElement>()
  let busy = $state(false)
  let error = $state<string | null>(null)

  onMount(() => {
    if (!host) return
    diff = monaco.editor.createDiffEditor(host, {
      readOnly: true,
      automaticLayout: true,
      renderSideBySide: true,
      theme: monacoTheme(ui.editorTheme),
    })
  })

  onDestroy(() => {
    oursModel?.dispose()
    theirsModel?.dispose()
    diff?.dispose()
  })

  $effect(() => {
    monaco.editor.setTheme(monacoTheme(ui.editorTheme))
  })

  $effect(() => {
    const path = selected
    if (!diff) return
    if (!path) {
      diff.setModel(null)
      return
    }
    void load(path)
  })

  async function load(path: string) {
    try {
      const versions = await invoke<Versions>('git_conflict_versions', { path })
      oursModel?.dispose()
      theirsModel?.dispose()
      oursModel = monaco.editor.createModel(versions.ours, languageFor(path))
      theirsModel = monaco.editor.createModel(versions.theirs, languageFor(path))
      diff?.setModel({ original: oursModel, modified: theirsModel })
      error = null
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function accept(resolution: 'accept_ours' | 'accept_theirs') {
    if (!selected || busy) return
    busy = true
    try {
      await invoke('git_resolve_conflict', { path: selected, resolution })
      files = files.filter((f) => f !== selected)
      selected = files[0] ?? ''
      error = null
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  async function editManually() {
    const path = selected
    if (!path) return
    try {
      if (!tabs.tabs.some((t) => t.id === path)) {
        const content = await invoke<string>('fs_read_file', { path })
        openTab({ id: path, path, content, modified: false, language: languageFor(path) })
      } else {
        tabs.activeTabId = path
      }
      // The editor needs the full window; conflicts stay listed in the status bar.
      ui.conflictsHidden = true
    } catch (e) {
      error = errorMessage(e)
    }
  }

  async function confirm() {
    if (files.length > 0 || busy) return
    busy = true
    try {
      if (!(await syncNow(null))) throw new Error(gitStatus.error ?? 'push failed')
      ui.conflictsHidden = false
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  function close() {
    ui.conflictsHidden = true
  }
</script>

<div class="overlay" role="dialog" aria-label="Merge conflicts">
  <div class="modal">
    <header>
      <span>Merge conflicts ({files.length})</span>
      <button class="close" aria-label="Close" onclick={close}>×</button>
    </header>

    <div class="body">
      <nav>
        {#each files as file (file)}
          <button
            class="file"
            class:active={file === selected}
            onclick={() => (selected = file)}
          >
            {file}
          </button>
        {/each}
        {#if files.length === 0}
          <p class="hint">All conflicts resolved.</p>
        {/if}
      </nav>

      <section class="detail">
        {#if selected}
          <div class="labels">
            <span>Local</span>
            <span>Remote</span>
          </div>
          <div class="diff" bind:this={host}></div>
          <div class="actions">
            <button disabled={busy} onclick={() => accept('accept_ours')}>Accept Local</button>
            <button disabled={busy} onclick={() => accept('accept_theirs')}>Accept Remote</button>
            <button disabled={busy} onclick={editManually}>Edit Manually</button>
          </div>
        {:else}
          <div class="done">
            <p>Every conflicted file has been resolved.</p>
            <button class="primary" disabled={busy} onclick={confirm}>
              {busy ? 'Merging…' : 'Confirm Merge'}
            </button>
          </div>
        {/if}
        {#if error}
          <p class="error">{error}</p>
        {/if}
      </section>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
  }
  .modal {
    display: flex;
    flex-direction: column;
    width: 1000px;
    max-width: calc(100vw - 48px);
    height: 80vh;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
  }
  .close {
    padding: 0 6px;
    color: var(--muted);
    font-size: 16px;
  }
  .close:hover {
    color: var(--text);
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  nav {
    display: flex;
    flex: 0 0 240px;
    flex-direction: column;
    gap: 2px;
    overflow: auto;
    padding: 6px;
    border-right: 1px solid var(--border);
  }
  .file {
    padding: 4px 8px;
    border-radius: 3px;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file:hover {
    background: var(--bg-raised);
  }
  .file.active {
    background: var(--accent);
    color: #fff;
  }
  .detail {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    padding: 8px;
    gap: 8px;
  }
  .labels {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .diff {
    flex: 1;
    min-height: 0;
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .actions button,
  .done button {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-raised);
  }
  .actions button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary {
    color: var(--ok);
  }
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 12px;
  }
  .done p,
  .hint {
    margin: 0;
    color: var(--muted);
  }
  .error {
    margin: 0;
    color: var(--err);
    font-size: 11px;
    overflow-wrap: anywhere;
  }
</style>
