<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { confirmDialog } from '../stores/confirm.svelte'
  import * as monaco from 'monaco-editor'
  import { tabs } from '../stores/tabs.svelte'
  import { languageFor } from '../utils/langmap'
  import { ui } from '../stores/ui.svelte'
  import { applyGitStatus, errorMessage, gitStatus, type FileChange, type GitStatus } from '../stores/git.svelte'
  import { logLine, pushToast } from '../stores/toasts.svelte'
  import { monacoTheme } from '../utils/themes'
  import { tip } from '../utils/tooltip'
  import { reloadTabFromDisk } from '../editor/bridge'
  import { isCurrentVersion } from '../utils/file-history'
  import { clampTimelineHeight } from '../utils/timeline'
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
  /// Non-null moves the panel from one file's history to one commit's diff.
  const commit = $derived(ui.timelineCommit)
  const path = $derived(active?.path ?? '')
  const name = $derived(path.split('/').pop() ?? path)
  const dirty = $derived(!!active?.modified)
  const language = $derived(active?.language ?? 'plaintext')
  // Any working-tree change, or a dirty buffer, means the disk matches no commit.
  const pending = $derived(
    dirty || gitStatus.changes.some((change) => change.path === path),
  )

  let commits = $state<CommitEntry[]>([])
  let files = $state<FileChange[]>([])
  let selected = $state('')
  let diff = $state<FileDiff | null>(null)
  let busy = $state(false)
  let loading = $state(false)
  let error = $state<string | null>(null)
  let dragging = $state(false)

  let host: HTMLDivElement
  let frame = $state<HTMLElement>()
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
  // A commit's diff never changes, so its hash alone is the reload key.
  const loadKey = $derived(commit ? `commit:${commit.hash}` : refreshKey)

  $effect(() => {
    const key = ui.timelineOpen ? loadKey : ''
    if (!key || key === loadedKey) return
    loadedKey = key
    selected = ''
    loggedFailures.clear()
    setDiff(null, '')
    if (commit) void loadCommit()
    else void load()
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

  // Monaco throws when a model disappears under it: one log line per distinct
  // failure, so a broken panel cannot flood the log.
  let loggedFailures = new Set<string>()
  function diffFailed(e: unknown) {
    const message = errorMessage(e)
    error = message
    if (loggedFailures.has(message)) return
    loggedFailures.add(message)
    logLine('error', `diff: ${message}`, 'editor')
  }

  function setDiff(next: FileDiff | null, key: string, lang = language) {
    diff = next
    // An exception must never escape into a Svelte flush: that aborts the DOM
    // update and leaves the whole panel half-painted.
    try {
      applyDiff(next, key, lang)
    } catch (e) {
      diffFailed(e)
    }
  }

  function applyDiff(next: FileDiff | null, key: string, lang: string) {
    // Detach before disposing: the diff editor throws if its models vanish
    // under it, which used to abort the effect flush and break the layout.
    diffEditor?.setModel(null)
    for (const model of models) model.dispose()
    models = []
    if (!next) return
    ensureDiff()
    if (!diffEditor) return
    // A model with this URI may survive a failed switch: createModel would throw.
    for (const side of ['old', 'new']) {
      monaco.editor.getModel(monaco.Uri.parse(`gitpad-diff://${side}/${key}`))?.dispose()
    }
    const original = monaco.editor.createModel(
      next.original,
      lang,
      monaco.Uri.parse(`gitpad-diff://old/${key}`),
    )
    const modified = monaco.editor.createModel(
      next.modified,
      lang,
      monaco.Uri.parse(`gitpad-diff://new/${key}`),
    )
    models = [original, modified]
    diffEditor.setModel({ original, modified })
  }

  async function loadCommit() {
    const target = ui.timelineCommit
    if (!target) return
    loading = true
    error = null
    files = await invoke<FileChange[]>('git_commit_files', { hash: target.hash }).catch((e) => {
      error = errorMessage(e)
      return []
    })
    loading = false
    if (files.length) void pick(files[0])
  }

  /// Commit mode: the list picks which file of the commit to diff.
  async function pick(file: FileChange) {
    const target = ui.timelineCommit
    if (!target || selected === file.path) return
    selected = file.path
    busy = true
    error = null
    try {
      setDiff(
        await invoke<FileDiff>('git_file_diff', { hash: target.hash, path: file.path }),
        `${target.hash}/${file.path}`,
        languageFor(file.path),
      )
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
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
    const restored = await confirmDialog({
      title: 'Restore file',
      message: `Restore ${target} from ${commit.shortHash}?`,
      confirmLabel: 'Restore',
    })
    if (!restored) return
    busy = true
    error = null
    try {
      applyGitStatus(
        await invoke<GitStatus>('git_restore_file', { hash: commit.hash, path: target }),
      )
      // The tab stays put: EditorPane re-reads the file into its own model.
      await reloadTabFromDisk(target)
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
    ui.timelineCommit = null
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
    const column = frame?.parentElement?.clientHeight ?? window.innerHeight
    ui.timelineHeight = clampTimelineHeight(column, startHeight + (startY - e.clientY))
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

  /// Created on first use: an empty diff editor draws a sash between two blank
  /// panes, which reads as a black seam on some compositors.
  function ensureDiff() {
    if (diffEditor || !host) return
    diffEditor = monaco.editor.createDiffEditor(host, {
      theme: monacoTheme(ui.editorTheme),
      readOnly: true,
      automaticLayout: true,
      renderSideBySide: true,
      minimap: { enabled: false },
      overviewRulerBorder: false,
      scrollBeyondLastLine: false,
      scrollbar: { vertical: 'hidden', verticalScrollbarSize: 8, horizontalScrollbarSize: 8 },
      // Inline diffs keep the original gutter next to the modified one; one
      // column of numbers is enough in a panel this narrow. Not in the 0.57
      // typings yet, hence the cast.
      hideOriginalLineNumbers: true,
    } as monaco.editor.IStandaloneDiffEditorConstructionOptions & {
      hideOriginalLineNumbers: boolean
    })
  }

  onMount(() => {
    return () => {
      try {
        // Same order as setDiff: the editor must let go before its models die.
        diffEditor?.setModel(null)
        diffEditor?.dispose()
        for (const model of models) model.dispose()
      } catch (e) {
        diffFailed(e)
      }
    }
  })
</script>

<section
  class="timeline"
  style="height: {ui.timelineHeight}px"
  aria-label="File history"
  bind:this={frame}
>
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
    {#if commit}
      <code class="sha">{commit.shortHash}</code>
      <span class="name">{commit.message}</span>
      {#if !loading}{files.length} file{files.length === 1 ? '' : 's'}{/if}
    {:else}
      <FileIcon name={path} size={13} />
      <span class="name">{name}</span>
      {#if !loading}{commits.length} commit{commits.length === 1 ? '' : 's'}{/if}
    {/if}
    <button class="close" aria-label={commit ? 'Close commit diff' : 'Close file history'} onclick={close}>×</button>
  </header>

  {#if error}
    <div class="error" role="alert">
      <span>{error}</span>
      <button onclick={() => void (commit ? loadCommit() : load())}>Retry</button>
    </div>
  {/if}

  <div class="body">
    <ul class="commits" style="flex: 0 0 {ui.timelineListWidth}px">
      {#if commit}
        {#each files as file (file.path)}
          <li class="row" class:active={selected === file.path}>
            <button class="commit" use:tip={file.path} onclick={() => void pick(file)}>
              <span class="badge {file.status}">{file.status}</span>
              <span class="msg">{file.path}</span>
            </button>
          </li>
        {:else}
          <li class="empty">{loading ? 'Loading…' : 'No files in this commit.'}</li>
        {/each}
      {:else if loading && commits.length === 0}
        <li class="skeleton"></li>
        <li class="skeleton"></li>
        <li class="skeleton"></li>
      {:else}
        {#each commits as commit, index (commit.hash)}
          <li class="row" class:active={selected === commit.hash}>
            <button class="commit entry" use:tip={commit.author} onclick={() => void select(commit)}>
              <span class="id">
                <code>{commit.shortHash}</code>
                <span class="meta">{when(commit.timestamp)}</span>
              </span>
              <span class="msg">{commit.message}</span>
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
        <p class="hint">Pick a {commit ? 'file' : 'commit'} to see the diff.</p>
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
    box-sizing: border-box;
    min-height: 0;
    /* Safety net for the JS clamp: leave 160px for the editor. */
    max-height: calc(100% - 160px);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: var(--surface-base);
  }
  .resize {
    position: absolute;
    top: calc(-1 * var(--sp-2) - 1px);
    left: 0;
    z-index: 2;
    width: 100%;
    height: var(--sp-2);
    cursor: row-resize;
  }
  /* Kept visible at rest: unlike the two side gutters, nothing else marks this
     edge as draggable. */
  .resize::before {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 32px;
    height: 4px;
    border-radius: var(--r-pill);
    background: var(--border);
    transform: translate(-50%, -50%);
    transition: background var(--t-fast);
    content: '';
  }
  .resize:hover::before {
    background: var(--border-strong);
  }
  .resize.active::before {
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
    width: 24px;
    height: 24px;
    margin-left: auto;
    border: 0;
    /* 24px square: the shared 9px radius would read as a circle here. */
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-lg);
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
    border-radius: var(--r-md);
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
  /* The gap between the two panels, made grabbable, like the sidebar gutter. */
  .split {
    position: relative;
    flex: 0 0 var(--sp-1);
    cursor: col-resize;
    touch-action: none;
  }
  .split::after {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 4px;
    border-radius: var(--r-pill);
    background: transparent;
    transform: translateX(-50%);
    transition: background var(--t-fast);
    content: '';
    pointer-events: none;
  }
  .split:hover::after {
    background: var(--border-strong);
  }
  .split.active::after {
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
  /* Two lines: each field keeps its own width, the message takes the rest. */
  .entry {
    flex-direction: column;
    align-items: stretch;
    gap: 2px;
  }
  .id {
    display: flex;
    align-items: baseline;
    gap: 4px;
    width: fit-content;
    max-width: 100%;
    min-width: 0;
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
    font-size: var(--fs-xs);
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
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: var(--fs-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: 0 0 auto;
    color: var(--accent-fg);
    font-size: var(--fs-2xs);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .badge.A {
    color: var(--ok);
  }
  .badge.D {
    color: var(--err);
  }
  .sha {
    flex: 0 0 auto;
    font-size: var(--fs-2xs);
  }
  .restore {
    flex: 0 0 auto;
    /* Desktop target, not touch: 32px is the smallest that still reads as one. */
    min-height: 32px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-xs);
  }
  .restore:hover:not(:disabled) {
    border-color: var(--border-strong);
    background: var(--surface-3);
  }
  /* Dimming alone still leaves a pill outline and a bright label: at 0.3 the
     text would read as a live control on the Current row. Drop the chrome. */
  .restore:disabled {
    opacity: 0.3;
    cursor: default;
    border-color: transparent;
    color: var(--muted);
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
