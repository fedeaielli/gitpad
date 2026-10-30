<script module lang="ts">
  import type { editor } from 'monaco-editor'

  let flush: (() => Promise<void>) | null = null
  let current = $state<editor.IStandaloneCodeEditor | null>(null)

  export function registerFlush(fn: (() => Promise<void>) | null) {
    flush = fn
  }

  /// Used by the close handshake in App.svelte before the app exits.
  export async function flushPendingSave() {
    await flush?.()
  }

  /// Shared with MarkdownPreview so it can sync its scroll position.
  export function getEditor() {
    return current
  }

  /// Jumps to a search hit; the editor already holds the active model.
  export function revealLine(line: number, column: number) {
    if (!current) return
    current.revealLineInCenter(line)
    current.setPosition({ lineNumber: line, column })
    current.focus()
  }

  export function setEditor(next: editor.IStandaloneCodeEditor | null) {
    current = next
  }
</script>

<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import loader from '@monaco-editor/loader'
  import * as monaco from 'monaco-editor'
  import editorWorker from 'monaco-editor/editor/editor.worker?worker'
  import jsonWorker from 'monaco-editor/language/json/json.worker?worker'
  import cssWorker from 'monaco-editor/language/css/css.worker?worker'
  import htmlWorker from 'monaco-editor/language/html/html.worker?worker'
  import tsWorker from 'monaco-editor/language/typescript/ts.worker?worker'
  import { tabs, discarded } from '../stores/tabs.svelte'
  import { ui } from '../stores/ui.svelte'
  import {
    gitStatus,
    isIgnorableSyncError,
    markPushed,
    type GitStatus,
  } from '../stores/git.svelte'
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
  loader.config({ monaco })

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

  async function savePending() {
    const id = pendingId
    pendingId = ''
    const tab = tabs.tabs.find((t) => t.id === id)
    if (!tab) return
    const content = tab.content
    inFlight.add(tab.path)
    try {
      await invoke('fs_write_file', { path: tab.path, content })
      saved.set(tab.path, content)
      if (tab.content === content) tab.modified = false
      saveError = null
      armPush()
    } catch (e) {
      tab.modified = true
      saveError = `save failed: ${String(e)}`
    } finally {
      inFlight.delete(tab.path)
    }
  }

  function armPush() {
    clearTimeout(pushTimer)
    if (!ui.autoSync) return
    pushTimer = setTimeout(() => void push(), ui.pushIdleMs)
  }

  async function push() {
    if (gitStatus.syncing || !gitStatus.configured) return
    gitStatus.syncing = true
    try {
      markPushed(await invoke<GitStatus>('git_push', { message: null }))
    } catch (e) {
      const message = String(e)
      if (!isIgnorableSyncError(message)) gitStatus.error = message
    } finally {
      gitStatus.syncing = false
    }
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
          saveError = `save failed: ${String(e)}`
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
    monaco.editor.setTheme(ui.editorTheme)
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
    if (pendingId) {
      persist.cancel()
      await savePending()
    }
    if (ui.autoSync) await push()
  }

  onMount(() => {
    editor = monaco.editor.create(host, {
      theme: ui.editorTheme,
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
    registerFlush(async () => {
      persist.cancel()
      await savePending()
    })
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
