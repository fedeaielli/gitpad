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
  import { tabs, discarded } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
  import { getEditor, registerFlush, setEditorInstance } from '../editor/bridge'
  import { monacoTheme } from '../utils/themes'
  import { errorMessage, gitStatus, syncNow } from '../stores/git.svelte'
  import { debounce } from '../utils/debounce'
  import { initVimMode } from 'monaco-vim'
  import { EmacsExtension } from 'monaco-emacs'
  import MarkdownPreview from './MarkdownPreview.svelte'

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

  const models = new Map<string, monaco.editor.ITextModel>()
  const saved = new Map<string, string>()
  const inFlight = new Set<string>()
  const viewStates = new Map<string, monaco.editor.ICodeEditorViewState | null>()

  const persist = debounce(() => void savePending(), () => ui.saveDebounceMs)
  let pushTimer: ReturnType<typeof setTimeout> | undefined
  let keymap: { dispose(): void } | undefined
  let vimStatus = $state<HTMLDivElement>()

  function activeTab() {
    return tabs.tabs.find((t) => t.id === tabs.activeTabId) ?? null
  }

  const isMarkdown = $derived(activeTab()?.language === 'markdown')
  const previewVisible = $derived(ui.previewMode !== 'off' && isMarkdown)

  async function savePending(): Promise<boolean> {
    const id = pendingId
    pendingId = ''
    const tab = tabs.tabs.find((t) => t.id === id)
    if (!tab) return true
    const content = tab.content
    inFlight.add(tab.path)
    try {
      await invoke('fs_write_file', { path: tab.path, content })
      saved.set(tab.path, content)
      if (tab.content === content) tab.modified = false
      saveError = null
      armPush()
      return true
    } catch (e) {
      // Keep it pending so the next flush retries instead of losing the buffer.
      if (!pendingId) pendingId = id
      tab.modified = true
      saveError = `save failed: ${errorMessage(e)}`
      return false
    } finally {
      inFlight.delete(tab.path)
    }
  }

  /// Exit path: no debounce left, so write every tab that is still dirty.
  async function flushAll(): Promise<boolean> {
    persist.cancel()
    const attempted = pendingId
    let ok = await savePending()
    for (const tab of tabs.tabs) {
      if (!tab.modified || tab.id === attempted) continue
      const content = tab.content
      inFlight.add(tab.path)
      try {
        await invoke('fs_write_file', { path: tab.path, content })
        saved.set(tab.path, content)
        if (tab.content === content) tab.modified = false
        saveError = null
      } catch (e) {
        tab.modified = true
        saveError = `save failed: ${errorMessage(e)}`
        ok = false
        break
      } finally {
        inFlight.delete(tab.path)
      }
    }
    return ok
  }

  function armPush() {
    clearTimeout(pushTimer)
    if (!ui.autoSync) return
    pushTimer = setTimeout(() => void syncNow(null), ui.pushIdleMs)
  }

  function onEdit() {
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
    if (path === currentPath) return
    if (editor && currentPath) viewStates.set(currentPath, editor.saveViewState())
    if (pendingId && pendingId !== path) {
      persist.cancel()
      void savePending()
    }
    currentPath = path
    if (!editor) return
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
    const hadEdits = pendingId !== null || tabs.tabs.some((t) => t.modified)
    if (pendingId) {
      persist.cancel()
      await savePending()
    }
    // Avoid pushing just because the window lost focus.
    if (ui.autoSync && (hadEdits || gitStatus.ahead > 0)) await syncNow(null)
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
    applyActive()

    return () => {
      registerFlush(null)
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
  <div class="editor-wrap" class:hidden={ui.previewMode === 'full' && isMarkdown}>
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
  {#if saveError}
    <p class="error">{saveError}</p>
  {/if}
</div>

<style>
  .pane {
    position: relative;
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
