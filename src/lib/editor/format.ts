import type { editor } from 'monaco-editor'
import { getEditor } from './bridge'
import { pushToast } from '../stores/toasts.svelte'

/// Prettier's exact output for the languages the editor formats.
export async function formatText(text: string, language: string): Promise<string> {
  const prettier = await import('prettier/standalone')
  if (language === 'yaml') {
    const yaml = await import('prettier/plugins/yaml')
    return prettier.format(text, { parser: 'yaml', plugins: [yaml] })
  }
  const babel = await import('prettier/plugins/babel')
  const estree = await import('prettier/plugins/estree')
  const plugins = [babel, estree]
  try {
    return await prettier.format(text, { parser: 'json', plugins })
  } catch {
    // A file with no value, only comments, is valid for the jsonc parser.
    return prettier.format(text, { parser: 'jsonc', plugins })
  }
}

export async function formatDocument(editor: editor.ICodeEditor): Promise<void> {
  const model = editor.getModel()
  if (!model) return
  const language = model.getLanguageId()
  if (language !== 'json' && language !== 'yaml') {
    pushToast('No formatter for this file type', 'info')
    return
  }
  const text = model.getValue()
  let formatted: string
  try {
    formatted = await formatText(text, language)
  } catch (e) {
    pushToast(`Format failed: ${e instanceof Error ? e.message : String(e)}`)
    return
  }
  if (formatted === text) return
  const position = editor.getPosition()
  editor.pushUndoStop()
  editor.executeEdits('format', [
    { range: model.getFullModelRange(), text: formatted, forceMoveMarkers: true },
  ])
  editor.pushUndoStop()
  if (position) {
    const line = Math.min(position.lineNumber, model.getLineCount())
    editor.setPosition({ lineNumber: line, column: Math.min(position.column, model.getLineMaxColumn(line)) })
  }
  editor.focus()
}

/// Palette entry: format whatever the editor is showing, if it can.
export async function formatActiveDocument(): Promise<void> {
  const editor = getEditor()
  if (editor) await formatDocument(editor)
}
