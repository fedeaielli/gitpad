/// Markdown tables as data: find the block around a line, change it, write it
/// back aligned. Pure string work, so the odd cases are testable alone.
export type Align = 'none' | 'left' | 'center' | 'right'

export type Table = {
  /// Line indexes into the document, both zero based and inclusive.
  start: number
  end: number
  /// Header first, delimiter row left out (its colons live in `aligns`).
  rows: string[][]
  aligns: Align[]
}

const PIPE_ROW = /^\s*\|.*\|\s*$/
const DELIMITER = /^(:)?-+(:)?$/

/// The table the given line belongs to, or null. `lines` is the whole document.
export function parseTable(lines: string[], index: number): Table | null {
  if (index < 0 || index >= lines.length || !PIPE_ROW.test(lines[index])) return null
  let start = index
  while (start > 0 && PIPE_ROW.test(lines[start - 1])) start--
  let end = index
  while (end < lines.length - 1 && PIPE_ROW.test(lines[end + 1])) end++
  // A header alone is not a table: markdown needs the delimiter row.
  if (end - start < 1) return null

  const header = cells(lines[start])
  const delimiter = cells(lines[start + 1])
  const aligns: Align[] = []
  for (const cell of delimiter) {
    const match = DELIMITER.exec(cell)
    if (!match) return null
    aligns.push(match[1] && match[2] ? 'center' : match[1] ? 'left' : match[2] ? 'right' : 'none')
  }

  const rows = [header, ...lines.slice(start + 2, end + 1).map(cells)]
  const columns = Math.max(aligns.length, ...rows.map((row) => row.length))
  while (aligns.length < columns) aligns.push('none')
  for (const row of rows) while (row.length < columns) row.push('')
  return { start, end, rows, aligns }
}

/// The same table as aligned markdown lines: header, delimiter, then the rest.
export function renderTable(rows: string[][], aligns: Align[]): string[] {
  const columns = Math.max(1, aligns.length, ...rows.map((row) => row.length))
  const widths = Array.from({ length: columns }, (_, column) => {
    const longest = Math.max(3, ...rows.map((row) => (row[column] ?? '').length))
    // The colons eat into the dashes, so aligned columns need a wider cell.
    if (aligns[column] === 'center') return Math.max(longest, 5)
    if (aligns[column] === 'left' || aligns[column] === 'right') return Math.max(longest, 4)
    return longest
  })
  const row = (values: string[]) =>
    '| ' + widths.map((width, column) => pad(values[column] ?? '', width, aligns[column])).join(' | ') + ' |'
  const delimiter =
    '| ' + widths.map((width, column) => dashes(width, aligns[column])).join(' | ') + ' |'
  return [row(rows[0] ?? []), delimiter, ...rows.slice(1).map(row)]
}

/// Which cell the cursor is in. A cursor on a pipe counts as the cell before it.
export function cellIndex(line: string, column: number): number {
  const before = line.slice(0, Math.max(0, column - 1))
  const pipes = [...before].filter((char) => char === '|').length
  return Math.max(0, pipes - 1)
}

/// Where the content of a cell starts, as a 1-based Monaco column.
export function contentColumn(line: string, cell: number): number {
  let pipe = 0
  for (let index = 0; index < line.length; index++) {
    if (line[index] !== '|') continue
    if (pipe === cell) {
      let after = index + 1
      while (after < line.length && line[after] === ' ') after++
      // A blank cell is all padding: type at its start, not against the pipe.
      if (after >= line.length || line[after] === '|') after = Math.min(index + 2, after)
      return after + 1
    }
    pipe++
  }
  return line.length + 1
}

/// Where Tab goes from a cell: the next one, a new row, or nowhere. The
/// registration turns this into an edit; keeping it here makes it testable.
export type TabStep = { kind: 'cell'; row: number; cell: number } | { kind: 'add' }

export function tabStep(table: Table, row: number, cell: number, backwards: boolean): TabStep | null {
  if (backwards) {
    if (cell > 0) return { kind: 'cell', row, cell: cell - 1 }
    if (row > 0) return { kind: 'cell', row: row - 1, cell: table.aligns.length - 1 }
    return null
  }
  if (cell < table.aligns.length - 1) return { kind: 'cell', row, cell: cell + 1 }
  return { kind: 'add' }
}

export function addRow(rows: string[][], at: number): string[][] {
  const next = rows.map((row) => [...row])
  next.splice(at, 0, new Array(next[0]?.length ?? 0).fill(''))
  return next
}

/// Removing the header promotes the first data row, so the table stays valid.
export function removeRow(rows: string[][], at: number): string[][] {
  if (rows.length <= 1) return rows
  const next = rows.map((row) => [...row])
  next.splice(at, 1)
  return next
}

export function addColumn(
  rows: string[][],
  aligns: Align[],
  at: number,
): { rows: string[][]; aligns: Align[] } {
  const nextAligns = [...aligns]
  nextAligns.splice(at, 0, 'none')
  return { rows: rows.map((row) => [...row.slice(0, at), '', ...row.slice(at)]), aligns: nextAligns }
}

export function removeColumn(
  rows: string[][],
  aligns: Align[],
  at: number,
): { rows: string[][]; aligns: Align[] } {
  if (aligns.length <= 1) return { rows, aligns }
  const nextAligns = aligns.filter((_, column) => column !== at)
  return { rows: rows.map((row) => row.filter((_, column) => column !== at)), aligns: nextAligns }
}

function cells(line: string): string[] {
  const trimmed = line.trim()
  return trimmed
    .slice(1, -1)
    .split('|')
    .map((cell) => cell.trim())
}

function pad(value: string, width: number, align: Align): string {
  const space = width - value.length
  if (space <= 0) return value
  if (align === 'right') return ' '.repeat(space) + value
  if (align === 'center') {
    const left = Math.floor(space / 2)
    return ' '.repeat(left) + value + ' '.repeat(space - left)
  }
  return value + ' '.repeat(space)
}

function dashes(width: number, align: Align): string {
  if (align === 'left') return ':' + '-'.repeat(width - 1)
  if (align === 'right') return '-'.repeat(width - 1) + ':'
  if (align === 'center') return ':' + '-'.repeat(width - 2) + ':'
  return '-'.repeat(width)
}
