import { describe, expect, it } from 'vitest'
import { buildSessions, isStartOnly, matches, parseLog } from './logs'

const START = '[2026-11-04][13:21:04][app][INFO] gitpad 0.1.5 starting'
const PUSH = '[2026-11-04][13:22:10][git][INFO] push: main'
const FAIL = '[2026-11-04][13:23:00][git][ERROR] git push: rejected'
const WARN = '[2026-11-04][13:24:00][editor][WARN] commit after delete failed: boom'

describe('parseLog', () => {
  it('reads the plugin format', () => {
    const [entry] = parseLog(PUSH)
    expect(entry).toMatchObject({
      date: '2026-11-04',
      time: '13:22:10',
      level: 'info',
      category: 'git',
      message: 'push: main',
    })
    expect(entry.at).toBe(Date.parse('2026-11-04T13:22:10'))
  })

  it('maps levels and the legacy module targets', () => {
    const entries = parseLog([FAIL, WARN, '[2026-11-04][13:25:00][gitpad_lib][INFO] old'].join('\n'))
    expect(entries.map((e) => [e.level, e.category])).toEqual([
      ['error', 'git'],
      ['warn', 'editor'],
      ['info', 'app'],
    ])
  })

  it('keeps unparsable lines instead of dropping them', () => {
    const entries = parseLog('thread panicked at src/main.rs:1\n\n')
    expect(entries).toHaveLength(1)
    expect(entries[0].message).toContain('panicked')
  })
})

describe('buildSessions', () => {
  it('splits at every start marker and puts the newest first', () => {
    const text = [
      START,
      PUSH,
      '[2026-11-04][14:00:00][app][INFO] gitpad 0.1.5 starting',
      FAIL,
    ].join('\n')
    const sessions = buildSessions([{ name: 'gitpad.log', current: true, modified: 0, text }])
    expect(sessions).toHaveLength(2)
    expect(sessions[0].entries.map((e) => e.message)).toEqual(['gitpad 0.1.5 starting', 'git push: rejected'])
    expect(sessions[1].entries.map((e) => e.message)).toEqual(['gitpad 0.1.5 starting', 'push: main'])
    expect(sessions[0].startedAt).toBeGreaterThan(sessions[1].startedAt)
  })

  it('falls back to the file time when a session has no start line', () => {
    const sessions = buildSessions([{ name: 'gitpad_old.log', current: false, modified: 42, text: PUSH }])
    expect(sessions).toHaveLength(1)
    expect(sessions[0].startedAt).toBe(42)
    expect(sessions[0].current).toBe(false)
  })

  it('keeps every session aligned with its own first line', () => {
    const text = [
      '[2026-11-04][00:00:54][app][INFO] gitpad 0.1.5 starting',
      '[2026-11-04][00:07:09][app][INFO] gitpad 0.1.5 starting',
      PUSH,
    ].join('\n')
    const sessions = buildSessions([{ name: 'gitpad.log', current: true, modified: 0, text }])
    expect(sessions.map((s) => [s.startedAt, s.entries[0].at])).toEqual([
      [Date.parse('2026-11-04T00:07:09'), Date.parse('2026-11-04T00:07:09')],
      [Date.parse('2026-11-04T00:00:54'), Date.parse('2026-11-04T00:00:54')],
    ])
  })

  it('marks a session that recorded nothing but its start', () => {
    const text = [START, '[2026-11-04][14:00:00][app][INFO] gitpad 0.1.5 starting', FAIL].join('\n')
    const [busy, quiet] = buildSessions([{ name: 'gitpad.log', current: true, modified: 0, text }])
    expect(isStartOnly(quiet)).toBe(true)
    expect(isStartOnly(busy)).toBe(false)
  })

  it('returns nothing for an empty file', () => {
    expect(buildSessions([{ name: 'gitpad.log', current: true, modified: 0, text: '' }])).toEqual([])
  })
})

describe('matches', () => {
  const [entry] = parseLog(FAIL)

  it('filters by level, category and text', () => {
    expect(matches(entry, 'error', [], '')).toBe(true)
    expect(matches(entry, 'warn', [], '')).toBe(false)
    expect(matches(entry, 'all', ['editor'], '')).toBe(false)
    expect(matches(entry, 'all', ['git'], 'REJECTED')).toBe(true)
    expect(matches(entry, 'all', ['git'], 'nope')).toBe(false)
  })
})
