export type LogLevel = 'info' | 'warn' | 'error'
export type LogCategory = 'editor' | 'git' | 'app'

export interface LogEntry {
  date: string
  time: string
  /** Parsed timestamp, 0 when the line did not match. */
  at: number
  level: LogLevel
  category: LogCategory
  message: string
}

export interface LogSession {
  key: string
  file: string
  /** The session the app is writing to right now. */
  current: boolean
  startedAt: number
  entries: LogEntry[]
}

export interface LogSource {
  name: string
  current: boolean
  modified: number
  text: string
}

// [2026-11-04][13:21:04][app][INFO] gitpad 0.1.5 starting
const LINE = /^\[(\d{4}-\d{2}-\d{2})\]\[(\d{2}:\d{2}:\d{2})\]\[([^\]]+)\]\[([A-Z]+)\](?: (.*))?$/

/// Rust logs carry their module as target; the app writes `editor`, `git` or
/// `app` explicitly (see app_log).
function categoryOf(target: string): LogCategory {
  if (target === 'git' || target.endsWith('::git')) return 'git'
  if (target === 'editor' || target.endsWith('::fs')) return 'editor'
  return 'app'
}

function levelOf(name: string): LogLevel {
  const level = name.toLowerCase()
  if (level === 'error') return 'error'
  if (level === 'warn') return 'warn'
  return 'info'
}

export function parseLog(text: string): LogEntry[] {
  const entries: LogEntry[] = []
  for (const line of text.split('\n')) {
    const match = LINE.exec(line)
    if (!match) {
      // Keep anything the plugin did not format (continuation, panic output).
      if (line.trim()) {
        entries.push({ date: '', time: '', at: 0, level: 'info', category: 'app', message: line })
      }
      continue
    }
    entries.push({
      date: match[1],
      time: match[2],
      at: Date.parse(`${match[1]}T${match[2]}`),
      level: levelOf(match[4]),
      category: categoryOf(match[3]),
      message: match[5] ?? '',
    })
  }
  return entries
}

/// The app writes this on every start: it is the only session boundary in a file.
export function isStart(entry: LogEntry): boolean {
  return entry.category === 'app' && /\bstarting$/.test(entry.message)
}

export function buildSessions(sources: LogSource[]): LogSession[] {
  const sessions: LogSession[] = []
  for (const source of sources) {
    let entries: LogEntry[] = []
    let startedAt = source.modified
    const flush = () => {
      if (!entries.length) return
      sessions.push({
        key: `${source.name}#${startedAt}#${sessions.length}`,
        file: source.name,
        current: source.current,
        startedAt,
        entries,
      })
      entries = []
    }
    for (const entry of parseLog(source.text)) {
      if (isStart(entry)) {
        flush()
        startedAt = entry.at || source.modified
      }
      entries.push(entry)
    }
    flush()
  }
  return sessions.sort((a, b) => b.startedAt - a.startedAt)
}

export function matches(entry: LogEntry, level: LogLevel | 'all', categories: LogCategory[], query: string) {
  if (level !== 'all' && entry.level !== level) return false
  if (categories.length && !categories.includes(entry.category)) return false
  const needle = query.trim().toLowerCase()
  if (needle && !entry.message.toLowerCase().includes(needle)) return false
  return true
}
