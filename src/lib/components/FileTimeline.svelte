<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from '@tauri-apps/plugin-dialog'
  import * as monaco from 'monaco-editor'
  import { closeIfClean, discarded, openTab, tabs } from '../stores/tabs.svelte'
  import { languageFor } from '../utils/langmap'
  import { ui } from '../stores/ui.svelte'
  import { applyGitStatus, errorMessage, gitStatus, type GitStatus } from '../stores/git.svelte'
  import { pushToast } from '../stores/toasts.svelte'
  import { monacoTheme } from '../utils/themes'
  import { tip } from '../utils/tooltip'
  import { isCurrentVersion } from '../utils/file-history'
  import FileIcon from './FileIcon.svelte'

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
  const name = $derived(path.split('/').pop() ?? path)
  const dirty = $derived(!!active?.modified)
  const language = $derived(active?.language ?? 'plaintext')
  // Any working-tree change, or a dirty buffer, means the disk matches no commit.
  const pending = $derived(
    dirty || gitStatus.changes.some((change) => change.path === path),
  )

  let commits = $state<CommitEntry[]>([])
  let selected = $state('')
  let diff = $state<FileDiff | null>(null)
  let busy = $state(false)
  let loading = $state(false)
  let error = $state<string | null>(null)
  let dragging = $state(false)

  let host: HTMLDivElement
  let diffEditor: monaco.editor.IStandaloneDiffEditor | undefined
  let models: monaco.editor.ITextModel[] = []
  let startY = 0
  let startHeight = 0
  let loadedKey = ''

  // Reload on another file, on a tree refresh and whenever this file's status
  // changes: the list goes stale after a commit, a pull or a restore elsewhere.
  const refreshKey = $derived(
    `${path}|${ui.treeVersion}|${pending}|${gitStatus.changes.map((c) => c.path + c.status).join(',')}`,
  )

  $effect(() => {
    const key = ui.timelineOpen ? refreshKey : ''
    if (!key || key === loadedKey) return
    loadedKey = key
    selected = ''
    setDiff(null, '')
    void load()
  })

  async function load() {
    const target = path
    if (!target) return
    loading = true
    error = null
    commits = await invoke<CommitEntry[]>('git_file_log', { path: target }).catch((e) => {
      error = errorMessage(e)
      return []
    })
    if (target === path) loading = false
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
    // Snapshot: closing the tab clears the derived path.
    const target = path
    if (dirty) {
      error = 'save or close the open file first'
      return
    }
    if (!(await ask(`Restore ${target} from ${commit.shortHash}?`, { title: 'Restore file' }))) return
    busy = true
    error = null
    try {
      applyGitStatus(
        await invoke<GitStatus>('git_restore_file', { hash: commit.hash, path: target }),
      )
      // Read before closing: a failure here leaves the tab exactly as it was.
      const content = await invoke<string>('fs_read_file', { path: target })
      if (!closeIfClean(target)) return
      // Let EditorPane drop the stale model before the tab comes back.
      await tick()
      openTab({
        id: target,
        path: target,
        content,
        modified: false,
        language: languageFor(target),
      })
      discarded.delete(target)
      selected = ''
      setDiff(null, '')
      ui.treeVersion += 1
      pushToast(`Restored ${target} from ${commit.shortHash}`, 'success')
      await load()
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  function close() {
    ui.timelineOpen = false
  }

  function when(timestamp: number) {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    })
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

  let splitting = $state(false)
  let splitX = 0
  let splitWidth = 0

  function startSplit(e: PointerEvent) {
    e.preventDefault()
    splitting = true
    splitX = e.clientX
    splitWidth = ui.timelineListWidth
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }

  function moveSplit(e: PointerEvent) {
    if (!splitting) return
    ui.timelineListWidth = Math.round(
      Math.min(560, Math.max(180, splitWidth + e.clientX - splitX)),
    )
  }

  function endSplit() {
    splitting = false
  }

  onMount(() => {
    diffEditor = monaco.editor.createDiffEditor(host, {
      theme: monacoTheme(ui.editorTheme),
      readOnly: true,
      automaticLayout: true,
      renderSideBySide: true,
      minimap: { enabled: false },
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

<section class="timeline" style="height: {ui.timelineHeight}px" aria-label="File history">
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

  <header class="head">
    <FileIcon name={path} size={13} />
    <span class="name">{name}</span>
    {#if !loading}{commits.length} commit{commits.length === 1 ? '' : 's'}{/if}
    <button class="close" aria-label="Close file history" onclick={close}>×</button>
  </header>

  {#if error}
    <div class="error">
      <span>{error}</span>
      <button onclick={() => void load()}>Retry</button>
    </div>
  {/if}

  <div class="body">
    <ul class="commits" style="flex: 0 0 {ui.timelineListWidth}px">
      {#if loading && commits.length === 0}
        <li class="skeleton"></li>
        <li class="skeleton"></li>
        <li class="skeleton"></li>
      {:else}
        {#each commits as commit, index (commit.hash)}
          <li class="row" class:active={selected === commit.hash}>
            <button class="commit" use:tip={commit.author} onclick={() => void select(commit)}>
              <code>{commit.shortHash}</code>
              <span class="msg">{commit.message}</span>
              <span class="meta">{when(commit.timestamp)}</span>
            </button>
            {#if isCurrentVersion(index, path, gitStatus.changes, dirty)}
              <span class="badge">Current</span>
            {/if}
            <button
              class="restore"
              disabled={busy || isCurrentVersion(index, path, gitStatus.changes, dirty)}
              use:tip={isCurrentVersion(index, path, gitStatus.changes, dirty)
                ? 'This is the current version'
                : `Restore ${name} from ${commit.shortHash}`}
              onclick={() => void restore(commit)}
            >
              Restore
            </button>
          </li>
        {:else}
          <li class="empty">No commits touch this file.</li>
        {/each}
      {/if}
    </ul>
    <div
      class="split"
      class:active={splitting}
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize file history list"
      aria-valuemin="180"
      aria-valuemax="560"
      aria-valuenow={ui.timelineListWidth}
      onpointerdown={startSplit}
      onpointermove={moveSplit}
      onpointerup={endSplit}
      onpointercancel={endSplit}
    ></div>
    <div class="diff">
      <div class="editor" bind:this={host}></div>
      {#if !diff}
        <p class="hint">Pick a commit to see the diff.</p>
      {/if}
    </div>
  </div>
</section>

<style>
  .timeline {
    position: relative;
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    gap: var(--sp-1);
    min-height: 0;
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
  .resize:hover {
    background: var(--border-strong);
  }
  .resize.active {
    background: var(--accent);
  }
  .head {
    display: flex;
    align-items: center;
    flex: 0 0 26px;
    gap: var(--sp-2);
    padding: 0 var(--sp-2);
    /* Rounded like every other panel: the container clips nothing, so each
       child carries its own corners and the gutter shows the pane behind. */
    border-radius: var(--r-lg);
    background: var(--surface-1);
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    font-size: var(--fs-sm);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    margin-left: auto;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 15px;
    line-height: 1;
  }
  .close:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .error {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: var(--sp-2);
    padding: 3px var(--sp-2);
    border-radius: var(--r-md);
    background: var(--surface-2);
    color: var(--err);
    font-size: var(--fs-xs);
  }
  .error span {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .error button {
    flex: 0 0 auto;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-xs);
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .commits {
    flex: 0 0 auto;
    margin: 0;
    padding: 4px 0;
    overflow: auto;
    border-radius: var(--r-lg);
    background: var(--surface-1);
    list-style: none;
  }
  /* The gap between the two panels, made grabbable, like the sidebar gutter. */
  .split {
    flex: 0 0 var(--sp-1);
    cursor: col-resize;
    touch-action: none;
  }
  .split:hover {
    background: var(--border-strong);
  }
  .split.active {
    background: var(--accent);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    padding-right: 4px;
  }
  .commit {
    position: relative;
    display: flex;
    flex: 1;
    align-items: baseline;
    gap: var(--sp-2);
    min-width: 0;
    padding: 3px var(--sp-2);
    text-align: left;
  }
  .commit:hover,
  .row.active .commit {
    background: var(--surface-2);
  }
  /* Inset past the corners: a box-shadow would follow the button radius. */
  .row.active .commit::before {
    position: absolute;
    top: var(--r-md);
    bottom: var(--r-md);
    left: 0;
    width: 2px;
    background: var(--accent);
    content: '';
  }
  code {
    flex: 0 0 auto;
    color: var(--accent-fg);
    font-size: 11px;
  }
  .msg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    flex: 0 0 auto;
    color: var(--muted);
    font-size: 11px;
  }
  .badge {
    flex: 0 0 auto;
    color: var(--accent-fg);
    font-size: 10px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .restore {
    flex: 0 0 auto;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font-size: 11px;
    opacity: 0.55;
  }
  .row:hover .restore,
  .restore:focus-visible {
    opacity: 1;
  }
  .restore:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .restore:disabled {
    opacity: 0.3;
  }
  .skeleton {
    height: 10px;
    margin: 6px var(--sp-2);
    border-radius: var(--r-sm);
    background: var(--surface-2);
  }
  .empty {
    padding: 6px var(--sp-2);
    color: var(--muted);
  }
  .diff {
    position: relative;
    flex: 1;
    min-width: 0;
    border-radius: var(--r-lg);
    overflow: hidden;
    background: var(--surface-0);
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
</style>
