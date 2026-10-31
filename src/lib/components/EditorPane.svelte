<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import * as monaco from 'monaco-editor'
  // editor.main registers every grammar; load only the languages langmap.ts maps.
  import 'monaco-editor/languages/definitions/markdown/register.js'
  import 'monaco-editor/languages/definitions/rust/register.js'
  import 'monaco-editor/languages/definitions/python/register.js'
  import 'monaco-editor/languages/definitions/shell/register.js'
  import 'monaco-editor/languages/definitions/yaml/register.js'
  import 'monaco-editor/languages/definitions/cpp/register.js'
  import 'monaco-editor/languages/definitions/go/register.js'
  import 'monaco-editor/languages/definitions/java/register.js'
  // Rich language services (hover, validation, completions) for the rest.
  import 'monaco-editor/languages/features/json/register.js'
  import 'monaco-editor/languages/features/css/register.js'
  import 'monaco-editor/languages/features/html/register.js'
  import 'monaco-editor/languages/features/typescript/register.js'
  import editorWorker from 'monaco-editor/editor/editor.worker?worker'
  import jsonWorker from 'monaco-editor/language/json/json.worker?worker'
  import cssWorker from 'monaco-editor/language/css/css.worker?worker'
  import htmlWorker from 'monaco-editor/language/html/html.worker?worker'
  import tsWorker from 'monaco-editor/language/typescript/ts.worker?worker'
  import { tabs, discarded, closeTab, type Tab } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
  import { getEditor, registerFlush, registerReload, setEditorInstance } from '../editor/bridge'
  import { planPullReload, type UpstreamChange } from '../utils/pull-reload'
  import { pushToast } from '../stores/toasts.svelte'
  import { monacoTheme } from '../utils/themes'
  import { errorMessage, gitStatus, syncNow } from '../stores/git.svelte'
  import { debounce } from '../utils/debounce'
  import { saveDecision } from '../utils/save-guard'
  import { initVimMode } from 'monaco-vim'
  import { EmacsExtension } from 'monaco-emacs'
  import MarkdownPreview from './MarkdownPreview.svelte'
  import FileTimeline from './FileTimeline.svelte'

  self.MonacoEnvironment = {
    getWorker(_workerId, label) {
      if (label === 'json') return new jsonWorker()
      if (label === 'css' || label === 'scss' || label === 'less') return new cssWorker()
      if (label === 'html' || label === 'handlebars' || label === 'razor') return new htmlWorker()
      if (label === 'typescript' || label === 'javascript') return new tsWorker()
      return new editorWorker()
    },
  }

  monaco.editor.defineTheme('gitpad-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': '#1e1e1e',
      'editor.foreground': '#d4d4d4',
      'editorLineNumber.foreground': '#6e6e6e',
      'editorLineNumber.activeForeground': '#d4d4d4',
      'editor.selectionBackground': '#2f6fa866',
      'editorCursor.foreground': '#d4d4d4',
      'editorIndentGuide.background1': '#3c3c3c',
    },
  })

  let host: HTMLDivElement
  let editor: monaco.editor.IStandaloneCodeEditor | undefined
  let saveError = $state<string | null>(null)
  let pendingId = ''
  let currentPath = ''
  // setValue() fires onDidChangeModelContent: ignore those while reloading.
  let applyingExternal = false

  const models = new Map<string, monaco.editor.ITextModel>()
  const saved = new Map<string, string>()
  const inFlight = new Map<string, Promise<boolean>>()
  const viewStates = new Map<string, monaco.editor.ICodeEditorViewState | null>()

  const persist = debounce(() => void savePending(), () => ui.saveDebounceMs)
  /// Close path: past this wait, report instead of risking a duplicate write.
  const FLUSH_WAIT_MS = 4000
  let pushTimer: ReturnType<typeof setTimeout> | undefined
  let keymap: { dispose(): void } | undefined
  let vimStatus = $state<HTMLDivElement>()

  function activeTab() {
    return tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null
  }

  const isMarkdown = $derived(activeTab()?.language === 'markdown')
  // Preview is a markdown-only view: other tabs always render the editor alone.
  const previewMode = $derived<'off' | 'split' | 'full'>(isMarkdown ? ui.previewMode : 'off')
  const previewVisible = $derived(previewMode !== 'off')

  /// Write one tab and track it so flushAll can wait for a debounce save.
  function writeTab(tab: Tab, content: string): Promise<boolean> {
    const write = (async () => {
      try {
        await invoke('fs_write_file', { path: tab.path, content })
        saved.set(tab.path, content)
        tab.savedContent = content
        if (tab.content === content) tab.modified = false
        saveError = null
        return true
      } catch (e) {
        tab.modified = true
        saveError = `save failed: ${errorMessage(e)}`
        return false
      }
    })()
    inFlight.set(tab.path, write)
    void write.finally(() => {
      if (inFlight.get(tab.path) === write) inFlight.delete(tab.path)
    })
    return write
  }

  async function savePending(): Promise<boolean> {
    const id = pendingId
    pendingId = ''
    const tab = tabs.tabs.find((t) => t.id === id)
    if (!tab) return true
    const decision = saveDecision(tab)
    if (decision === 'skip') {
      tab.modified = false
      return true
    }
    if (decision === 'stale') {
      tab.modified = true
      saveError = 'file changed upstream — autosave paused'
      return false
    }
    if (decision === 'refuse') {
      tab.modified = true
      saveError = 'refusing to save an empty file — reopen the tab'
      return false
    }
    const ok = await writeTab(tab, tab.content)
    if (ok) armPush()
    // Keep it pending so the next flush retries instead of losing the buffer.
    else if (!pendingId) pendingId = id
    return ok
  }

  /// Exit path: no debounce left, so write every tab that is still dirty.
  async function flushAll(): Promise<boolean> {
    persist.cancel()
    // A save started by the debounce may still be writing: wait for it so the
    // loop below sees the flag it sets instead of writing the same file twice.
    let timer: ReturnType<typeof setTimeout> | undefined
    const timedOut = await Promise.race([
      Promise.allSettled([...inFlight.values()]).then(() => false),
      new Promise<boolean>((resolve) => {
        timer = setTimeout(() => resolve(true), FLUSH_WAIT_MS)
      }),
    ])
    clearTimeout(timer)
    if (timedOut) {
      // The first write is still running: a second one on the same path would
      // race it, so let the caller ask the user instead.
      saveError = 'a save is still in progress — try again in a moment'
      return false
    }
    const attempted = pendingId
    let ok = await savePending()
    for (const tab of tabs.tabs) {
      if (!tab.modified || tab.id === attempted) continue
      const decision = saveDecision(tab)
      if (decision === 'skip') {
        tab.modified = false
        continue
      }
      if (decision === 'stale') {
        tab.modified = true
        saveError = 'file changed upstream — autosave paused'
        ok = false
        break
      }
      if (decision === 'refuse') {
        tab.modified = true
        saveError = 'refusing to save an empty file — reopen the tab'
        ok = false
        break
      }
      if (!(await writeTab(tab, tab.content))) {
        ok = false
        break
      }
    }
    return ok
  }

  function armPush() {
    clearTimeout(pushTimer)
    if (!ui.autoSync || gitStatus.conflicts.length > 0) return
    pushTimer = setTimeout(() => void syncNow(null), ui.pushIdleMs)
  }

  function onEdit() {
    if (applyingExternal) return
    const tab = activeTab()
    const model = editor?.getModel()
    if (!tab || !model) return
    tab.content = model.getValue()
    tab.modified = true
    pendingId = tab.id
    persist.run()
  }

  function setEditor(next: monaco.editor.IStandaloneCodeEditor | null) {
    setEditorInstance(next)
  }

  function applyActive() {
    const tab = activeTab()
    const path = tab?.path ?? ''
    // onMount re-runs this once Monaco exists; setting currentPath here would
    // make that call return early and leave the first tab without a model.
    if (!editor) return
    if (path === currentPath) return
    if (editor && currentPath) viewStates.set(currentPath, editor.saveViewState())
    if (pendingId && pendingId !== path) {
      persist.cancel()
      void savePending()
    }
    currentPath = path
    if (!tab) {
      editor.setModel(null)
      return
    }
    let model = models.get(path)
    if (!model) {
      model = monaco.editor.createModel(tab.content, tab.language, monaco.Uri.file(path))
      models.set(path, model)
      saved.set(path, tab.content)
    }
    editor.setModel(model)
    const state = viewStates.get(path)
    if (state) editor.restoreViewState(state)
    editor.focus()
  }

  $effect(() => {
    applyActive()
  })

  // Closed tabs: keep the last edits and drop the model.
  $effect(() => {
    const open = new Set(tabs.tabs.map((t) => t.path))
    for (const [path, model] of models) {
      if (open.has(path)) continue
      if (
        !discarded.has(path) &&
        !inFlight.has(path) &&
        model.getValue() !== saved.get(path)
      ) {
        void invoke('fs_write_file', { path, content: model.getValue() }).catch((e) => {
          saveError = `save failed: ${errorMessage(e)}`
        })
      }
      discarded.delete(path)
      if (editor?.getModel() === model) editor.setModel(null)
      model.dispose()
      models.delete(path)
      saved.delete(path)
      viewStates.delete(path)
    }
  })

  $effect(() => {
    monaco.editor.setTheme(monacoTheme(ui.editorTheme))
  })

  $effect(() => {
    editor?.updateOptions({ fontSize: ui.fontSize, fontFamily: ui.fontFamily })
  })

  // Vim/Emacs keymaps wrap the single editor instance; swapping presets disposes the old one.
  $effect(() => {
    const mode = ui.keybindings
    keymap?.dispose()
    keymap = undefined
    if (!editor) return
    if (mode === 'vim' && vimStatus) {
      keymap = initVimMode(editor, vimStatus)
    } else if (mode === 'emacs') {
      const emacs = new EmacsExtension(editor)
      emacs.start()
      keymap = emacs
    }
  })

  async function onBlur() {
    const hadEdits = pendingId !== '' || tabs.tabs.some((t) => t.modified)
    if (pendingId) {
      persist.cancel()
      await savePending()
    }
    // Avoid pushing just because the window lost focus.
    if (ui.autoSync && gitStatus.conflicts.length === 0 && (hadEdits || gitStatus.ahead > 0)) {
      await syncNow(null)
    }
  }

  function isMissing(e: unknown) {
    return !!e && typeof e === 'object' && (e as { code?: unknown }).code === 'NOT_FOUND'
  }

  function closeRemoved(path: string) {
    const tab = tabs.tabs.find((candidate) => candidate.path === path)
    if (!tab) return
    discarded.add(tab.path)
    closeTab(tab.id)
    pushToast(`${path} removed upstream`, 'info')
  }

  function markStale(path: string) {
    const tab = tabs.tabs.find((candidate) => candidate.path === path)
    if (!tab || tab.staleUpstream) return
    tab.staleUpstream = true
    tab.modified = true
    pushToast(`${path} changed upstream — autosave paused for this tab, copy your text and reopen it`)
  }

  async function reloadTab(path: string) {
    const tab = tabs.tabs.find((candidate) => candidate.path === path)
    if (!tab || tab.modified) return markStale(path)
    const before = tab.content
    let content: string
    try {
      content = await invoke<string>('fs_read_file', { path })
    } catch (e) {
      if (isMissing(e)) closeRemoved(path)
      else markStale(path)
      return
    }
    // The user may have typed during the read.
    if (tab.modified || tab.content !== before || inFlight.has(path) || pendingId === tab.id) {
      markStale(path)
      return
    }
    if (content === tab.content) return
    tab.content = content
    tab.savedContent = content
    tab.modified = false
    saved.set(path, content)
    const model = models.get(path)
    if (!model) return
    applyingExternal = true
    try {
      if (editor?.getModel() === model) {
        const state = editor.saveViewState()
        model.setValue(content)
        if (state) editor.restoreViewState(state)
      } else {
        model.setValue(content)
      }
    } finally {
      applyingExternal = false
    }
  }

  async function applyPullChanges(changes: UpstreamChange[]) {
    const plan = planPullReload(
      changes,
      tabs.tabs,
      (path) => inFlight.has(path) || pendingId === path,
    )
    for (const path of plan.close) closeRemoved(path)
    for (const path of plan.stale) markStale(path)
    for (const path of plan.reload) await reloadTab(path)
  }

  onMount(() => {
    editor = monaco.editor.create(host, {
      theme: monacoTheme(ui.editorTheme),
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: ui.fontSize,
      fontFamily: ui.fontFamily,
      scrollBeyondLastLine: false,
      renderWhitespace: 'selection',
      scrollbar: { vertical: 'auto', verticalScrollbarSize: 6, horizontalScrollbarSize: 6 },
    })
    setEditor(editor)
    const contentSub = editor.onDidChangeModelContent(onEdit)
    const cursorSub = editor.onDidChangeCursorPosition((e) => {
      ui.cursor = { line: e.position.lineNumber, column: e.position.column }
    })
    editor.addAction({
      id: 'gitpad.findReplace',
      label: 'Find and Replace',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyH],
      run: () => {
        void editor?.getAction('editor.action.startFindReplaceAction')?.run()
      },
    })
    window.addEventListener('blur', onBlur)
    registerFlush(flushAll)
    registerReload(applyPullChanges)
    applyActive()

    return () => {
      registerFlush(null)
      registerReload(null)
      setEditor(null)
      window.removeEventListener('blur', onBlur)
      contentSub.dispose()
      cursorSub.dispose()
      persist.cancel()
      clearTimeout(pushTimer)
      models.forEach((m) => m.dispose())
      models.clear()
      keymap?.dispose()
      editor?.dispose()
    }
  })
</script>

<div class="pane">
  <div class="content">
    <div class="editor-wrap" class:hidden={previewMode === 'full'}>
      <div class="editor" bind:this={host}></div>
      {#if ui.keybindings === 'vim'}
        <div class="vim" bind:this={vimStatus}></div>
      {/if}
      {#if !activeTab()}
        <p class="empty">No file open</p>
      {/if}
    </div>
    {#if previewVisible}
      <div class="preview"><MarkdownPreview /></div>
    {/if}
  </div>
  {#if ui.timelineOpen && activeTab()}
    <FileTimeline />
  {/if}
  {#if saveError}
    <p class="error">{saveError}</p>
  {/if}
</div>

<style>
  .pane {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .content {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .editor-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  .editor-wrap.hidden {
    display: none;
  }
  .editor {
    position: absolute;
    inset: 0;
  }
  .vim {
    position: absolute;
    right: 8px;
    bottom: 6px;
    z-index: 5;
    padding: 0 4px;
    border-radius: 3px;
    background: var(--bg-panel);
    color: var(--muted);
    font: 12px ui-monospace, monospace;
    pointer-events: none;
  }
  .preview {
    flex: 1;
    min-width: 0;
    border-left: 1px solid var(--border);
  }
  .empty {
    position: absolute;
    top: 40%;
    right: 0;
    left: 0;
    margin: 0;
    color: var(--muted);
    text-align: center;
    pointer-events: none;
  }
  .error {
    position: absolute;
    right: 8px;
    bottom: 8px;
    margin: 0;
    padding: 4px 8px;
    border: 1px solid var(--err);
    border-radius: 3px;
    background: var(--bg-panel);
    color: var(--err);
    font-size: 12px;
  }
</style>
