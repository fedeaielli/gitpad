export interface GithubUser {
  login: string
  avatarUrl: string
}

export const auth = $state({
  authState: 'unauthenticated' as 'unauthenticated' | 'authenticated' | 'pending',
  user: null as GithubUser | null,
})
