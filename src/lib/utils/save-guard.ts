export type SaveDecision = 'skip' | 'refuse' | 'write'

// The real protection against the pull/read race is the backend worktree lock;
// this guard only stops an empty buffer from overwriting a file on disk.
export function saveDecision(tab: { content: string; savedContent?: string }): SaveDecision {
  if (tab.content === tab.savedContent) return 'skip' // nothing new to save
  if (tab.content === '' && !!tab.savedContent) return 'refuse' // emptying a full file
  return 'write'
}
