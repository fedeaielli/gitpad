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
  import { getEditor, registerFlush, registerReload, registerReloadTab, registerSaveAll, setEditorInstance } from '../editor/bridge'
  import { resetUndoState, trackChange } from '../editor/history.svelte'
  import { registerSlashCommands } from '../editor/slash-register'
  import { registerTableEditing } from '../editor/table-register'
  import { registerListEditing } from '../editor/list'

  import { planPullReload, decideReload, type UpstreamChange } from '../utils/pull-reload'
  import { pushToast } from '../stores/toasts.svelte'
  import { monacoTheme } from '../utils/themes'
  import { errorMessage, gitStatus, syncNow } from '../stores/git.svelte'
  import { auth } from '../stores/auth.svelte'
  import { debounce } from '../utils/debounce'
  import { withTimeout } from '../utils/timeout'
  import { saveDecision } from '../utils/save-guard'
  import { initVimMode } from 'monaco-vim'
  import { EmacsExtension } from 'monaco-emacs'
  import MarkdownPreview from './MarkdownPreview.svelte'
  import FileViewer from './FileViewer.svelte'
  import { formatDocument } from '../editor/format'

  self.MonacoEnvironment = {
    getWorker(_workerId, label) {
      if (label === 'json') return new jsonWorker()
      if (label === 'css' || label === 'scss' || label === 'less') return new cssWorker()
      if (label === 'html' || label === 'handlebars' || label === 'razor') return new htmlWorker()
      if (label === 'typescript' || label === 'javascript') return new tsWorker()
      return new editorWorker()
    },
  }

  // One Monaco theme per app theme: surface and text colours mirror app.css.
  monaco.editor.defineTheme('gitpad-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': '#131110',
      'editor.foreground': '#d8d3ca',
      'editorLineNumber.foreground': '#857f75',
      'editorLineNumber.activeForeground': '#d8d3ca',
      'editor.selectionBackground': '#264f78',
      'editor.lineHighlightBackground': '#23211e',
      'editorCursor.foreground': '#d8d3ca',
      'editorIndentGuide.background1': '#35312c',
      'editorIndentGuide.activeBackground1': '#5f594f',
      // Find/replace widget, clearly lighter than the editor surface.
      'editorWidget.background': '#2b2824',
      'editorWidget.foreground': '#d8d3ca',
      'widget.border': '#6f685d',
      // The `/` menu. Its own colours, not list.*: the selected row was VS
      // Code's cool blue, which reads as a foreign panel on this warm surface.
      'editorSuggestWidget.background': '#2b2824',
      'editorSuggestWidget.border': '#6f685d',
      'editorSuggestWidget.foreground': '#d8d3ca',
      'editorSuggestWidget.selectedBackground': '#35312c',
      'editorSuggestWidget.selectedForeground': '#ffffff',
      'editorSuggestWidget.highlightForeground': '#4daafc',
      'editorSuggestWidget.focusHighlightForeground': '#4daafc',
      'editorSuggestWidgetStatus.foreground': '#a39d93',
      // Widgets that otherwise inherit VS Code's cool greys.
      'list.hoverBackground': '#2b2824',
      'list.activeSelectionBackground': '#264f78',
      'list.activeSelectionForeground': '#d8d3ca',
      'list.inactiveSelectionBackground': '#2b2824',
      'list.highlightForeground': '#4daafc',
      'list.deemphasizedForeground': '#a39d93',
      'input.background': '#23211e',
      'input.foreground': '#d8d3ca',
      'dropdown.background': '#2b2824',
      'dropdown.foreground': '#d8d3ca',
      'focusBorder': '#4daafc',
      'badge.background': '#35312c',
      'badge.foreground': '#d8d3ca',
      'editorLink.activeForeground': '#4daafc',
      // Monaco parses these with Color.fromHex(), which only accepts #hex and
      // falls back to pure red on anything else: rgba() silently painted the
      // scrollbar red. Keep the notation #RRGGBBAA.
      // The source is a warm taupe, not --text: #d8d3ca only has 14 levels of
      // chroma and 40% alpha over this surface washes it down to a neutral grey.
      'scrollbarSlider.background': '#c8b89c80',
      'scrollbarSlider.hoverBackground': '#c8b89ca6',
      'scrollbarSlider.activeBackground': '#c8b89ccc',
    },
  })

  monaco.editor.defineTheme('gitpad-light', {
    base: 'vs',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': '#eeeade',
      'editor.foreground': '#24211d',
      'editorLineNumber.foreground': '#6b665c',
      'editorLineNumber.activeForeground': '#24211d',
      'editor.selectionBackground': '#d6e6f7',
      'editor.lineHighlightBackground': '#f2eee4',
      'editorCursor.foreground': '#24211d',
      'editorIndentGuide.background1': '#e8e3d7',
      'editorIndentGuide.activeBackground1': '#8f887b',
      'editorWidget.background': '#f2eee4',
      'editorWidget.foreground': '#24211d',
      'widget.border': '#8f887b',
      'editorSuggestWidget.background': '#f2eee4',
      'editorSuggestWidget.border': '#8f887b',
      'editorSuggestWidget.foreground': '#24211d',
      'editorSuggestWidget.selectedBackground': '#e8e3d7',
      'editorSuggestWidget.selectedForeground': '#24211d',
      'editorSuggestWidget.highlightForeground': '#005fb8',
      'editorSuggestWidget.focusHighlightForeground': '#005fb8',
      'editorSuggestWidgetStatus.foreground': '#6b665c',
      'list.hoverBackground': '#e8e3d7',
      'list.activeSelectionBackground': '#d6e6f7',
      'list.activeSelectionForeground': '#24211d',
      'list.inactiveSelectionBackground': '#ece7db',
      'list.highlightForeground': '#005fb8',
      'list.deemphasizedForeground': '#6b665c',
      'input.background': '#fffdf8',
      'input.foreground': '#24211d',
      'dropdown.background': '#f2eee4',
      'dropdown.foreground': '#24211d',
      'focusBorder': '#005fb8',
      'badge.background': '#e8e3d7',
      'badge.foreground': '#24211d',
      'editorLink.activeForeground': '#005fb8',
      // Same reasoning as gitpad-dark: a warm brown at low alpha, not --text.
      'scrollbarSlider.background': '#6b6152bf',
      'scrollbarSlider.hoverBackground': '#6b6152e0',
      'scrollbarSlider.activeBackground': '#6b6152f5',
    },
  })

  monaco.editor.defineTheme('gitpad-hc', {
    base: 'hc-black',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': '#000000',
      'editor.foreground': '#ffffff',
      'editorLineNumber.foreground': '#d4d4d4',
      'editorLineNumber.activeForeground': '#ffffff',
      'editor.selectionBackground': '#264f78',
      'editor.lineHighlightBackground': '#0a0a0a',
      'editorCursor.foreground': '#ffffff',
      'editorIndentGuide.background1': '#6fc3df',
      'editorIndentGuide.activeBackground1': '#6fc3df',
      'editorWidget.background': '#1a1a1a',
      'editorWidget.foreground': '#ffffff',
      'widget.border': '#6fc3df',
      // Inverted on purpose: Monaco's hc defaults paint the selection white, so
      // the text has to go black or it disappears into it.
      'editorSuggestWidget.background': '#1a1a1a',
      'editorSuggestWidget.border': '#6fc3df',
      'editorSuggestWidget.foreground': '#ffffff',
      'editorSuggestWidget.selectedBackground': '#ffffff',
      'editorSuggestWidget.selectedForeground': '#000000',
      'editorSuggestWidget.highlightForeground': '#6fc3df',
      'editorSuggestWidget.focusHighlightForeground': '#000000',
      'editorSuggestWidgetStatus.foreground': '#d4d4d4',
      'scrollbarSlider.background': '#00ffff80',
      'scrollbarSlider.hoverBackground': '#00ffffbf',
      'scrollbarSlider.activeBackground': '#00ffffe6',
    },
  })

  let host: HTMLDivElement
  let editor: monaco.editor.IStandaloneCodeEditor | undefined
  let saveError = $state<string | null>(null)
  const pending = new Set<string>()
  let currentPath = ''
  // setValue() fires onDidChangeModelContent: ignore those while reloading.
  let applyingExternal = false

  const models = new Map<string, monaco.editor.ITextModel>()
  const saved = new Map<string, string>()
  const inFlight = new Map<string, Promise<boolean>>()
  const viewStates = new Map<string, monaco.editor.ICodeEditorViewState | null>()

  const persist = debounce(() => void savePending(), () => ui.saveDebounceMs)
  /// Close path: one budget for the whole flush (waiting on in-flight writes
  /// included). Past this wait, report and let the user decide.
  const FLUSH_WAIT_MS = 4000
  let pushTimer: ReturnType<typeof setTimeout> | undefined
  let keymap: { dispose(): void } | undefined
  let vimStatus = $state<HTMLDivElement>()

  function activeTab() {
    return tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null
  }

  const active = $derived(activeTab())
  const isMarkdown = $derived(active?.language === 'markdown')
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
    const ids = [...pending]
    pending.clear()
    let ok = true
    for (const id of ids) {
      const tab = tabs.tabs.find((t) => t.id === id)
      if (!tab) continue
      const decision = saveDecision(tab)
      if (decision === 'skip') {
        tab.modified = false
        continue
      }
      if (decision === 'stale') {
        tab.modified = true
        saveError = 'file changed upstream — autosave paused'
        ok = false
        continue
      }
      if (decision === 'refuse') {
        tab.modified = true
        saveError = 'refusing to save an empty file — reopen the tab'
        ok = false
        continue
      }
      if (await writeTab(tab, tab.content)) armPush()
      // Keep a failed write queued so the next flush retries it.
      else {
        pending.add(id)
        ok = false
      }
    }
    return ok
  }

  /// Exit path: no debounce left, so write every tab that is still dirty.
  async function flushAll(): Promise<boolean> {
    try {
      return await withTimeout(
        flushBody().then((r) => r.ok),
        FLUSH_WAIT_MS,
        'flush',
      )
    } catch {
      // The body is not cancelled: it keeps waiting for the write in flight and
      // only then writes the rest, so nothing races. If the user cancels, the
      // `finally` handlers still clear `inFlight` and no `closing` flag is left set.
      saveError = 'a save is still in progress — close anyway?'
      return false
    }
  }

  async function flushBody(): Promise<{ ok: boolean; wrote: boolean }> {
    persist.cancel()
    pending.clear()
    // A save started by the debounce may still be writing: wait for it so the
    // loop below sees the flag it sets instead of writing the same file twice.
    await Promise.allSettled([...inFlight.values()])
    let ok = true
    let wrote = false
    for (const tab of tabs.tabs) {
      if (!tab.modified) continue
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
      wrote = true
    }
    return { ok, wrote }
  }

  function armPush() {
    clearTimeout(pushTimer)
    if (!ui.autoSync || gitStatus.conflicts.length > 0) return
    // A local-only repository has no account to push with: stay quiet.
    if (auth.authState === 'unauthenticated') return
    pushTimer = setTimeout(() => void pushWhenIdle(), ui.pushIdleMs)
  }

  async function pushWhenIdle() {
    if (!ui.autoSync || gitStatus.conflicts.length > 0) return
    if (await syncNow(null)) return
    // Another sync owns the gate: retry when it is done instead of dropping
    // the local commit.
    if (gitStatus.syncing) pushTimer = setTimeout(() => void pushWhenIdle(), 1000)
  }

  function onEdit(e: monaco.editor.IModelContentChangedEvent) {
    const changed = editor?.getModel()
    // A reload rewrites the buffer: its history starts over, it does not grow.
    if (changed) {
      if (applyingExternal) resetUndoState(changed.getAlternativeVersionId())
      else trackChange(changed.getAlternativeVersionId(), e.isUndoing, e.isRedoing)
    }
    if (applyingExternal) return
    const tab = activeTab()
    const model = editor?.getModel()
    if (!tab || !model) return
    tab.content = model.getValue()
    tab.modified = true
    pending.add(tab.id)
    persist.run()
  }

  function setEditor(next: monaco.editor.IStandaloneCodeEditor | null) {
    setEditorInstance(next)
  }

  // Three lines of air under the last line: cursorSurroundingLines cannot do it
  // alone, because at the end of the document there is no scrollable room.
  // Line height follows the font, so this is recomputed, not set once.
  function syncBottomPadding() {
    if (!editor) return
    const lineHeight = editor.getOption(monaco.editor.EditorOption.lineHeight)
    editor.updateOptions({ padding: { top: 6, bottom: 3 * lineHeight } })
  }

  function applyActive() {
    const tab = activeTab()
    const path = tab?.path ?? ''
    // onMount re-runs this once Monaco exists; setting currentPath here would
    // make that call return early and leave the first tab without a model.
    if (!editor) return
    if (path === currentPath) return
    if (editor && currentPath) viewStates.set(currentPath, editor.saveViewState())
    if (pending.size > 0 && !pending.has(path)) {
      persist.cancel()
      void savePending()
    }
    currentPath = path
    if (!tab || tab.view) {
      editor.setModel(null)
      resetUndoState(0)
      return
    }
    let model = models.get(path)
    if (!model) {
      model = monaco.editor.createModel(tab.content, tab.language, monaco.Uri.file(path))
      models.set(path, model)
      saved.set(path, tab.content)
    }
    editor.setModel(model)
    resetUndoState(model.getAlternativeVersionId())
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
    // Viewers never own a model, so their marker would never be consumed here.
    for (const path of discarded) if (!models.has(path)) discarded.delete(path)
  })

  $effect(() => {
    monaco.editor.setTheme(monacoTheme(ui.editorTheme))
  })

  $effect(() => {
    // Read before the optional chain: while the editor is still undefined the
    // argument is never evaluated, so the effect would subscribe to nothing.
    const fontSize = ui.fontSize
    const fontFamily = ui.fontFamily
    editor?.updateOptions({ fontSize, fontFamily })
    syncBottomPadding()
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
    const hadEdits = pending.size > 0 || tabs.tabs.some((t) => t.modified)
    if (pending.size > 0) {
      persist.cancel()
      await savePending()
    }
    // Avoid pushing just because the window lost focus.
    if (ui.autoSync && !ui.quitting && gitStatus.conflicts.length === 0 && (hadEdits || gitStatus.ahead > 0)) {
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
    pushToast(`${path} deleted externally`, 'info')
  }

  function markStale(path: string) {
    const tab = tabs.tabs.find((candidate) => candidate.path === path)
    if (!tab || tab.staleUpstream) return
    tab.staleUpstream = true
    tab.modified = true
    pushToast(
      `${path} changed on disk — autosave paused for this tab, copy your text and reopen it`,
      'warning',
    )
  }

  async function reloadTab(path: string) {
    const tab = tabs.tabs.find((candidate) => candidate.path === path)
    if (!tab) return
    // A viewer is read-only: ask it to re-read instead of parsing bytes as UTF-8.
    if (tab.view) {
      tab.reloadKey = (tab.reloadKey ?? 0) + 1
      return
    }
    const before = tab.content
    let content: string
    try {
      content = await invoke<string>('fs_read_file', { path })
    } catch (e) {
      // A dirty buffer must not be dropped because the file vanished.
      if (isMissing(e)) {
        if (tab.modified) markStale(path)
        else closeRemoved(path)
      } else markStale(path)
      return
    }
    // The user may have typed, or a save may have started, during the read.
    // The pending set is not checked: it marks scheduled saves, not writes in flight.
    const raced = inFlight.has(path) || tab.content !== before
    const action = decideReload(tab, content, raced)
    if (action === 'stale') return markStale(path)
    if (action === 'keep') return
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
    // Only an in-flight write is busy: a scheduled save is not, reloadTab tells
    // our own autosave echo (disk === savedContent) from a real external edit.
    const plan = planPullReload(changes, tabs.tabs, (path) => inFlight.has(path))
    for (const path of plan.close) closeRemoved(path)
    for (const path of plan.stale) markStale(path)
    for (const path of plan.reload) await reloadTab(path)
  }

  onMount(() => {
    editor = monaco.editor.create(host, {
      theme: monacoTheme(ui.editorTheme),
      automaticLayout: true,
      minimap: { enabled: false },
      // The 1 px ruler border reads as a stray line down the editor's right edge.
      overviewRulerBorder: false,
      fontSize: ui.fontSize,
      fontFamily: ui.fontFamily,
      scrollBeyondLastLine: false,
      // Three lines of air under the caret, so typing at the fold scrolls the
      // view instead of leaving the cursor glued to the bottom edge.
      cursorSurroundingLines: 3,
      renderWhitespace: 'selection',
      // Keeps the first line off the tab bar so tab and document read as one surface.
      padding: { top: 6 },
      scrollbar: { vertical: 'auto', verticalScrollbarSize: 6, horizontalScrollbarSize: 6 },
      // Without this Monaco adds a view zone on top and pushes the first lines
      // down whenever find or replace opens.
      find: { addExtraSpaceOnTop: false },
      // The `/` menu items are commands, not symbols: Monaco's kind glyphs are
      // all the same square here, so they only add noise.
      suggest: { showIcons: false },
    })
    setEditor(editor)
    syncBottomPadding()
    const slashSub = registerSlashCommands(monaco, editor)
    const tableSub = registerTableEditing(monaco, editor)
    const listSub = registerListEditing(monaco, editor)
    const formatSub = editor.addAction({
      id: 'gitpad.formatDocument',
      label: 'Format document',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyI],
      contextMenuGroupId: '1_modification',
      run: (target) => void formatDocument(target),
    })
    const contentSub = editor.onDidChangeModelContent(onEdit)
    const cursorSub = editor.onDidChangeCursorPosition((e) => {
      ui.cursor = { line: e.position.lineNumber, column: e.position.column }
    })
    window.addEventListener('blur', onBlur)
    registerFlush(flushAll)
    registerSaveAll(async () => {
      const r = await flushBody()
      return { ...r, error: r.ok ? null : saveError }
    })
    registerReload(applyPullChanges)
    registerReloadTab(reloadTab)
    applyActive()

    return () => {
      registerFlush(null)
      registerSaveAll(null)
      registerReload(null)
      registerReloadTab(null)
      setEditor(null)
      window.removeEventListener('blur', onBlur)
      slashSub()
      tableSub()
      listSub()
      formatSub.dispose()
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
    <div class="editor-wrap" class:hidden={previewMode === 'full' || active?.view}>
      <div class="editor" bind:this={host}></div>
      {#if ui.keybindings === 'vim'}
        <div class="vim" bind:this={vimStatus}></div>
      {/if}
    </div>
    {#if active && active.view}
      <FileViewer tab={active} />
    {/if}
    {#if previewVisible}
      <div class="preview"><MarkdownPreview /></div>
    {/if}
  </div>
  {#if saveError}
    <p class="error" role="alert">{saveError}</p>
  {/if}
</div>

<style>
  .pane {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: var(--sp-1);
    min-height: 0;
    border-radius: 0 0 var(--r-lg) var(--r-lg);
    background: var(--surface-base);
  }
  .content {
    display: flex;
    flex: 1;
    min-height: 0;
    border-radius: 0 0 var(--r-lg) var(--r-lg);
    background: var(--surface-0);
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
    border-radius: var(--r-sm);
    background: var(--surface-1);
    color: var(--muted);
    font: var(--fs-sm) ui-monospace, monospace;
    pointer-events: none;
  }
  .preview {
    flex: 1;
    min-width: 0;
    box-shadow: inset 1px 0 0 var(--border);
  }
  .error {
    position: absolute;
    right: 8px;
    bottom: 8px;
    margin: 0;
    padding: 4px 8px;
    border: 1px solid var(--err);
    border-radius: var(--r-sm);
    background: var(--surface-1);
    color: var(--err);
    font-size: var(--fs-sm);
  }
</style>
