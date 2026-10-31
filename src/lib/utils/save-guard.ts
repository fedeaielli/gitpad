export type SaveDecision = 'stale' | 'skip' | 'refuse' | 'write'

// The real protection against the pull/read race is the backend worktree lock;
// this guard only stops an empty buffer or a stale one from overwriting the file.
export function saveDecision(tab: {
  content: string
  savedContent?: string
  staleUpstream?: boolean
}): SaveDecision {
  if (tab.staleUpstream) return 'stale' // a pull rewrote this file under us
  if (tab.content === tab.savedContent) return 'skip' // nothing new to save
  if (tab.content === '' && !!tab.savedContent) return 'refuse' // emptying a full file
  return 'write'
}
