import type * as monaco from 'monaco-editor'

type Editor = monaco.editor.IStandaloneCodeEditor

/// A Markdown list item: bullets, numbers and GitHub task checkboxes.
export type ListItem = {
  /// Marker the next line starts with.
  prefix: string
  /// Index just past the marker, so an empty item can be cleared whole.
  end: number
  /// Nothing follows the marker: the next Enter should end the list.
  empty: boolean
}

const ITEM = /^(\s*)([-*+]|\d+[.)])(\s+)(\[[ xX]\]\s+)?/

export function listItem(line: string): ListItem | null {
  const match = ITEM.exec(line)
  if (!match) return null
  const [, indent, marker, space, task] = match
  const next = /^\d/.test(marker)
    ? `${Number.parseInt(marker, 10) + 1}${marker.slice(-1)}`
    : marker
  // A task continues unchecked, whatever state the line above was in.
  const prefix = `${indent}${next}${space}${task ? '[ ] ' : ''}`
  return { prefix, end: match[0].length, empty: line.slice(match[0].length).trim() === '' }
}

/// Enter carries a Markdown list to the next line and clears an empty item.
/// Only Markdown models, only with an empty selection, and only when the caret
/// is past the marker: every other Enter stays Monaco's own.
/// shortcut: a numbered line inside a fenced block continues too, add a fence
/// check if that ever bites.
export function registerListEditing(m: typeof monaco, editor: Editor): () => void {
  const sub = editor.onKeyDown((event) => {
    if (event.keyCode !== m.KeyCode.Enter) return
    if (event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return
    const model = editor.getModel()
    const position = editor.getPosition()
    const selection = editor.getSelection()
    if (!model || !position || !selection || !selection.isEmpty()) return
    if (model.getLanguageId() !== 'markdown') return

    const item = listItem(model.getLineContent(position.lineNumber))
    if (!item) return
    if (!item.empty && position.column - 1 < item.end) return

    event.preventDefault()
    event.stopPropagation()

    // setPosition alone never scrolls: only Monaco's own key path reveals the
    // caret, so a hand-placed caret would sit below the fold until the next key.
    // setPosition alone never scrolls: only Monaco's own key path reveals the
    // caret, so a hand-placed caret would sit below the fold until the next key.
    if (item.empty) {
      const line = position.lineNumber
      editor.executeEdits('list', [{ range: new m.Range(line, 1, line, item.end + 1), text: '' }])
      editor.setPosition({ lineNumber: line, column: 1 })
      return
    }
    const line = position.lineNumber
    const column = position.column
    editor.executeEdits('list', [
      { range: new m.Range(line, column, line, column), text: '\n' + item.prefix },
    ])
    const next = { lineNumber: line + 1, column: item.prefix.length + 1 }
    editor.setPosition(next)
    editor.revealPosition(next)
  })
  return () => sub.dispose()
}
