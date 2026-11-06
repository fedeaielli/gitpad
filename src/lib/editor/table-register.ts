import type * as monaco from 'monaco-editor'
import { addRow, cellIndex, contentColumn, parseTable, renderTable, tabStep, type Align, type Table } from './table'

type Editor = monaco.editor.IStandaloneCodeEditor

/// Rewrites the table block in one edit, then parks the caret in a cell. The
/// whole block changes at once so undo brings the old table back in one step.
export function writeTable(
  m: typeof monaco,
  editor: Editor,
  table: Table,
  rows: string[][],
  aligns: Align[],
  row: number,
  cell: number,
) {
  const model = editor.getModel()
  if (!model) return
  const rendered = renderTable(rows, aligns)
  const last = table.end + 1
  editor.executeEdits('table', [
    {
      range: new m.Range(table.start + 1, 1, last, model.getLineMaxColumn(last)),
      text: rendered.join('\n'),
    },
  ])
  // The delimiter row sits between the header and the first data row, so a
  // data row is one line further down than its index suggests.
  const index = Math.min(row === 0 ? 0 : row + 1, rendered.length - 1)
  editor.setPosition({ lineNumber: table.start + index + 1, column: contentColumn(rendered[index], cell) })
  editor.focus()
}

/// Tab walks the cells of a Markdown table and adds a row from the last one.
/// Outside a table the key is left to Monaco, so indentation still works.
export function registerTableEditing(m: typeof monaco, editor: Editor): () => void {
  const sub = editor.onKeyDown((event) => {
    if (event.keyCode !== m.KeyCode.Tab) return
    if (event.altKey || event.ctrlKey || event.metaKey) return
    const model = editor.getModel()
    const position = editor.getPosition()
    if (!model || !position) return
    if (model.getLanguageId() !== 'markdown') return
    const lines = model.getLinesContent()
    const table = parseTable(lines, position.lineNumber - 1)
    if (!table) return

    const cell = cellIndex(lines[position.lineNumber - 1], position.column)
    const row = Math.max(0, position.lineNumber - 1 - table.start - 1)
    event.preventDefault()
    event.stopPropagation()

    const go = (targetRow: number, targetCell: number) => {
      const index = targetRow === 0 ? 0 : targetRow + 1
      const line = table.start + index + 1
      editor.setPosition({ lineNumber: line, column: contentColumn(lines[line - 1], targetCell) })
      editor.focus()
    }

    if (event.shiftKey) {
      const step = tabStep(table, row, cell, true)
      if (!step || step.kind !== 'cell') return
      return go(step.row, step.cell)
    }
    const step = tabStep(table, row, cell, false)
    if (!step) return
    if (step.kind === 'cell') return go(step.row, step.cell)
    writeTable(m, editor, table, addRow(table.rows, row + 1), table.aligns, row + 1, 0)
  })
  return () => sub.dispose()
}
