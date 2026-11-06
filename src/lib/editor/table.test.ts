import { describe, expect, it } from 'vitest'
import {
  addColumn,
  addRow,
  cellIndex,
  contentColumn,
  parseTable,
  removeColumn,
  removeRow,
  renderTable,
  tabStep,
} from './table'

const doc = (...lines: string[]) => lines

function tableAt(lines: string[], index: number) {
  const table = parseTable(lines, index)
  if (!table) throw new Error('no table')
  return table
}

describe('parseTable', () => {
  it('finds the block around a line and leaves the delimiter out of the rows', () => {
    const lines = doc('intro', '| A | B |', '| --- | --- |', '| 1 | 2 |', 'outro')
    const table = tableAt(lines, 2)
    expect(table.start).toBe(1)
    expect(table.end).toBe(3)
    expect(table.rows).toEqual([
      ['A', 'B'],
      ['1', '2'],
    ])
    expect(table.aligns).toEqual(['none', 'none'])
  })

  it('reads a table without padding', () => {
    const table = tableAt(doc('|A|B|', '|---|---|', '|1|2|'), 2)
    expect(table.rows).toEqual([
      ['A', 'B'],
      ['1', '2'],
    ])
  })

  it('keeps the alignments of the delimiter row', () => {
    expect(tableAt(doc('| A | B | C |', '| :-- | --: | :-: |'), 0).aligns).toEqual([
      'left',
      'right',
      'center',
    ])
  })

  it('widens to the longest row when the header is short', () => {
    const table = tableAt(doc('| A |', '| --- |', '| 1 | 2 |'), 2)
    expect(table.aligns).toEqual(['none', 'none'])
    expect(table.rows[0]).toEqual(['A', ''])
  })

  it('rejects a lone line, a header without delimiter and a broken delimiter', () => {
    expect(parseTable(doc('| A | B |'), 0)).toBeNull()
    expect(parseTable(doc('text', '| A | B |'), 1)).toBeNull()
    expect(parseTable(doc('| A | B |', '| --x | --- |'), 0)).toBeNull()
    expect(parseTable(doc('| A |', '| --- |', 'text'), 3)).toBeNull()
  })
})

describe('renderTable', () => {
  it('aligns the columns to the longest cell and is idempotent', () => {
    const lines = renderTable(
      [
        ['A', 'Bee'],
        ['1', '2'],
      ],
      ['none', 'none'],
    )
    expect(lines).toEqual(['| A   | Bee |', '| --- | --- |', '| 1   | 2   |'])
    expect(renderTable(parseTable(lines, 0)!.rows, ['none', 'none'])).toEqual(lines)
  })

  it('keeps the colon markers inside the column width', () => {
    const [header, delimiter] = renderTable(
      [
        ['a', 'b', 'c'],
        ['1', '2', '3'],
      ],
      ['left', 'right', 'center'],
    )
    expect(delimiter).toBe('| :--- | ---: | :---: |')
    expect(header).toBe('| a    |    b |   c   |')
  })
})

describe('cell positions', () => {
  const line = '| a | b |'

  it('counts the pipes before the cursor', () => {
    expect(cellIndex(line, 1)).toBe(0)
    expect(cellIndex(line, 3)).toBe(0)
    expect(cellIndex(line, 7)).toBe(1)
    expect(cellIndex('| x |', 1)).toBe(0)
  })

  it('puts the caret on the first character of a cell', () => {
    expect(contentColumn(line, 0)).toBe(3)
    expect(contentColumn(line, 1)).toBe(7)
    expect(contentColumn(line, 9)).toBe(line.length + 1)
  })

  it('puts the caret inside a blank cell, not against the pipe', () => {
    expect(contentColumn('|       | b |', 0)).toBe(3)
    expect(contentColumn('|  |', 0)).toBe(3)
  })
})

describe('structure changes', () => {  const rows = [
    ['A', 'B'],
    ['1', '2'],
  ]

  it('adds an empty row of the right width', () => {
    expect(addRow(rows, 1)).toEqual([
      ['A', 'B'],
      ['', ''],
      ['1', '2'],
    ])
  })

  it('promotes the first data row when the header goes', () => {
    expect(removeRow(rows, 0)).toEqual([['1', '2']])
    expect(removeRow([['A', 'B']], 0)).toEqual([['A', 'B']])
  })

  it('adds a column with no alignment at the given index', () => {
    expect(addColumn(rows, ['left', 'none'], 1)).toEqual({
      rows: [
        ['A', '', 'B'],
        ['1', '', '2'],
      ],
      aligns: ['left', 'none', 'none'],
    })
  })

  it('removes a column but never the last one', () => {
    expect(removeColumn(rows, ['left', 'right'], 0)).toEqual({
      rows: [['B'], ['2']],
      aligns: ['right'],
    })
    expect(removeColumn([['A']], ['none'], 0)).toEqual({ rows: [['A']], aligns: ['none'] })
  })
})

describe('tabStep', () => {
  const table = { start: 0, end: 2, rows: [['A', 'B'], ['1', '2']], aligns: ['none' as const, 'none' as const] }

  it('walks forward cell by cell and asks for a row after the last', () => {
    expect(tabStep(table, 1, 0, false)).toEqual({ kind: 'cell', row: 1, cell: 1 })
    expect(tabStep(table, 1, 1, false)).toEqual({ kind: 'add' })
  })

  it('walks back to the last cell of the row above', () => {
    expect(tabStep(table, 1, 1, true)).toEqual({ kind: 'cell', row: 1, cell: 0 })
    expect(tabStep(table, 1, 0, true)).toEqual({ kind: 'cell', row: 0, cell: 1 })
    expect(tabStep(table, 0, 0, true)).toBeNull()
  })
})
