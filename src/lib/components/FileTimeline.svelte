<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from '@tauri-apps/plugin-dialog'
  import * as monaco from 'monaco-editor'
  import { closeIfClean, tabs } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
  import { applyGitStatus, errorMessage, type GitStatus } from '../stores/git.svelte'
  import { monacoTheme } from '../utils/themes'

  interface CommitEntry {
    hash: string
    shortHash: string
    message: string
    author: string
    timestamp: number
    filesChanged: number
  }

  interface FileDiff {
    original: string
    modified: string
  }

  const active = $derived(tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null)
  const path = $derived(active?.path ?? '')
  const language = $derived(active?.language ?? 'plaintext')

  let commits = $state<CommitEntry[]>([])
  let selected = $state('')
  let diff = $state<FileDiff | null>(null)
  let busy = $state(false)
  let error = $state<string | null>(null)
  let dragging = $state(false)

  let host: HTMLDivElement
  let diffEditor: monaco.editor.IStandaloneDiffEditor | undefined
  let models: monaco.editor.ITextModel[] = []
  let startY = 0
  let startHeight = 0
  let loadedFor = ''

  $effect(() => {
    const target = ui.timelineOpen ? path : ''
    if (!target || target === loadedFor) return
    loadedFor = target
    selected = ''
    setDiff(null, '')
    void load(target)
  })

  async function load(target: string) {
    error = null
    commits = await invoke<CommitEntry[]>('git_file_log', { path: target }).catch((e) => {
      error = errorMessage(e)
      return []
    })
  }

  function setDiff(next: FileDiff | null, hash: string) {
    diff = next
    for (const model of models) model.dispose()
    models = []
    if (!next || !diffEditor) return
    const original = monaco.editor.createModel(
      next.original,
      language,
      monaco.Uri.parse(`gitpad-diff://old/${hash}`),
    )
    const modified = monaco.editor.createModel(
      next.modified,
      language,
      monaco.Uri.parse(`gitpad-diff://new/${hash}`),
    )
    models = [original, modified]
    diffEditor.setModel({ original, modified })
  }

  async function select(commit: CommitEntry) {
    if (selected === commit.hash) {
      selected = ''
      setDiff(null, '')
      return
    }
    selected = commit.hash
    busy = true
    error = null
    try {
      setDiff(await invoke<FileDiff>('git_file_diff', { hash: commit.hash, path }), commit.hash)
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  async function restore(commit: CommitEntry) {
    if (!closeIfClean(path)) {
      error = 'save or close the open file first'
      return
    }
    if (!(await ask(`Restore ${path} from ${commit.shortHash}?`, { title: 'Restore file' }))) return
    busy = true
    error = null
    try {
      applyGitStatus(await invoke<GitStatus>('git_restore_file', { hash: commit.hash, path }))
      ui.treeVersion += 1
      await load(path)
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  function startResize(e: PointerEvent) {
    dragging = true
    startY = e.clientY
    startHeight = ui.timelineHeight
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }

  function moveResize(e: PointerEvent) {
    if (!dragging) return
    ui.timelineHeight = Math.min(600, Math.max(120, startHeight + (startY - e.clientY)))
  }

  function endResize() {
    dragging = false
  }

  onMount(() => {
    diffEditor = monaco.editor.createDiffEditor(host, {
      theme: monacoTheme(ui.editorTheme),
      readOnly: true,
      renderSideBySide: false,
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: ui.fontSize,
      fontFamily: ui.fontFamily,
      scrollBeyondLastLine: false,
      scrollbar: { vertical: 'auto', verticalScrollbarSize: 6, horizontalScrollbarSize: 6 },
    })
    if (diff) setDiff(diff, selected)
    return () => {
      for (const model of models) model.dispose()
      diffEditor?.dispose()
    }
  })
</script>

<section class="timeline" style="height: {ui.timelineHeight}px">
  <div
    class="resize"
    class:active={dragging}
    role="separator"
    aria-orientation="horizontal"
    aria-label="Resize file history"
    aria-valuemin="120"
    aria-valuemax="600"
    aria-valuenow={ui.timelineHeight}
    onpointerdown={startResize}
    onpointermove={moveResize}
    onpointerup={endResize}
    onpointercancel={endResize}
  ></div>
  <div class="body">
    <ul class="commits">
      {#each commits as commit (commit.hash)}
        <li class="row">
          <button
            class="commit"
            class:active={selected === commit.hash}
            title={commit.author}
            onclick={() => void select(commit)}
          >
            <code>{commit.shortHash}</code>
            <span class="msg">{commit.message}</span>
            <span class="meta">{new Date(commit.timestamp * 1000).toLocaleDateString()}</span>
          </button>
          <button class="restore" disabled={busy} onclick={() => void restore(commit)}>
            Restore
          </button>
        </li>
      {:else}
        <li class="empty">No commits touch this file.</li>
      {/each}
    </ul>
    <div class="diff">
      <div class="editor" bind:this={host}></div>
      {#if !diff}
        <p class="hint">Pick a commit to see the diff.</p>
      {/if}
    </div>
  </div>
  {#if error}<p class="error">{error}</p>{/if}
</section>

<style>
  .timeline {
    position: relative;
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    min-height: 0;
    border-top: 1px solid var(--border);
    background: var(--bg-panel);
  }
  .resize {
    position: absolute;
    top: -2px;
    left: 0;
    z-index: 2;
    width: 100%;
    height: 4px;
    cursor: row-resize;
  }
  .resize:hover,
  .resize.active {
    background: var(--accent);
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .commits {
    flex: 0 0 220px;
    margin: 0;
    padding: 4px 0;
    overflow: auto;
    list-style: none;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .commit {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    padding: 3px 8px;
    text-align: left;
  }
  .commit:hover {
    background: var(--bg-raised);
  }
  .commit.active {
    background: var(--bg-raised);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  code {
    color: var(--muted);
    font-size: 11px;
  }
  .msg,
  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--muted);
    font-size: 11px;
  }
  .restore {
    flex: 0 0 auto;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-size: 11px;
    opacity: 0;
  }
  .row:hover .restore {
    opacity: 1;
  }
  .restore:hover {
    border-color: var(--accent);
  }
  .empty {
    padding: 6px 8px;
    color: var(--muted);
  }
  .diff {
    position: relative;
    flex: 1;
    min-width: 0;
    border-left: 1px solid var(--border);
  }
  .editor {
    position: absolute;
    inset: 0;
  }
  .hint {
    position: absolute;
    top: 8px;
    left: 10px;
    margin: 0;
    color: var(--muted);
    pointer-events: none;
  }
  .error {
    margin: 0;
    padding: 2px 8px;
    color: var(--err);
  }
</style>
