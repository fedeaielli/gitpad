import { invoke } from '@tauri-apps/api/core'

export interface GithubUser {
  login: string
  avatarUrl: string
}

/// The activity bar and the account tab share one copy: a window focus used to
/// cost a `GET /user` each time, which also filled the log with duplicates.
const STALE_MS = 5 * 60 * 1000
let loadedAt = 0
let pending: Promise<void> | null = null

export const auth = $state({
  authState: 'unauthenticated' as 'unauthenticated' | 'authenticated' | 'pending',
  user: null as GithubUser | null,
})

export function setUser(user: GithubUser | null) {
  auth.user = user
  auth.authState = user ? 'authenticated' : 'unauthenticated'
  loadedAt = Date.now()
}

/// A plain logout: the next check has to ask the backend again.
export function forgetUser() {
  setUser(null)
  loadedAt = 0
}

/// Fetches the user unless a fresh copy is already in the store.
export async function loadUser(force = false): Promise<void> {
  if (!force && loadedAt > 0 && Date.now() - loadedAt < STALE_MS) return
  if (pending) return pending
  auth.authState = 'pending'
  pending = invoke<GithubUser>('github_get_user')
    .then((user) => setUser(user))
    .catch(() => setUser(null))
    .finally(() => {
      loadedAt = Date.now()
      pending = null
    })
  return pending
}
