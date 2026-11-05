<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'

  import { open } from '@tauri-apps/plugin-dialog'
  import { openExternal } from '../utils/open'
  import { ui } from '../stores/ui.svelte'
  import { applyGitStatus, errorMessage, type GitStatus } from '../stores/git.svelte'
  import { loadUser, setUser } from '../stores/auth.svelte'
  import GithubLogin from './GithubLogin.svelte'

  interface UserInfo {
    login: string
    avatarUrl: string
  }
  interface RepoInfo {
    name: string
    fullName: string
    cloneUrl: string
    private: boolean
  }
  interface LocalRepo {
    from: string
    to: string
    initialized: boolean
    remote: boolean
  }
  type Step = 'welcome' | 'auth' | 'repo' | 'done'

  let { onskip }: { onskip: () => void } = $props()

  let step = $state<Step>('welcome')
  let token = $state('')
  let user = $state<UserInfo | null>(null)
  let repo = $state<RepoInfo | null>(null)
  /// Folder that will hold the clone; empty keeps the app data dir.
  let folder = $state('')
  /// Set when the user opened a folder instead of cloning from GitHub.
  let localRepo = $state<LocalRepo | null>(null)
  let busy = $state(false)
  let error = $state<string | null>(null)
  /// Set when the token had to stay in memory instead of the keyring.
  let storageNote = $state('')

  /// What the repository step is waiting for. `account`, `lookup` and
  /// `creating` come from the backend; `cloning` is local to the next step.
  type Stage = 'account' | 'lookup' | 'creating' | 'cloning'
  const STAGES: Record<Stage, string> = {
    account: 'Contacting GitHub…',
    lookup: 'Looking for gitpad_notes…',
    creating: 'Creating gitpad_notes…',
    cloning: 'Cloning the repository…',
  }
  let stage = $state<Stage>('account')

  onMount(() => {
    const unlisten = listen<string>('onboarding-stage', (e) => {
      if (e.payload in STAGES) stage = e.payload as Stage
    })
    return () => {
      void unlisten.then((fn) => fn())
    }
  })

  function fail(e: unknown) {
    error = errorMessage(e)
    busy = false
  }

  async function validatePat() {
    busy = true
    error = null
    try {
      const info = await invoke<UserInfo>('github_validate_token', { token: token.trim() })
      await invoke('auth_set_pat', { token: token.trim() })
      user = info
      // The activity bar reads the shared store: without this it stays signed
      // out until its cached "no user" expires.
      setUser(info)
      goRepo()
    } catch (e) {
      fail(e)
    }
  }

  async function goRepo() {
    step = 'repo'
    busy = true
    error = null
    stage = 'account'
    // The token is already stored; this only tells the UI where it lives.
    try {
      if (!(await invoke<boolean>('auth_keyring_ok'))) {
        storageNote =
          'Signed in, but the OS keyring is unavailable: the token is kept in a protected file on this machine.'
      }
    } catch {
      // A failed probe must not block the setup.
    }
    try {
      repo = await invoke<RepoInfo>('github_init_repo')
    } catch (e) {
      error = errorMessage(e)
    } finally {
      busy = false
    }
  }

  async function chooseFolder() {
    const picked = await open({ directory: true, multiple: false, title: 'Repository folder' })
    if (typeof picked === 'string') folder = picked
  }

  async function clone() {
    if (!repo) return
    busy = true
    error = null
    stage = 'cloning'
    try {
      const parent = folder.trim()
      ui.repoPath = await invoke<string>('git_clone', { url: repo.cloneUrl, parent: parent || null })
      applyGitStatus(await invoke<GitStatus>('git_status'))
      ui.treeVersion += 1
      step = 'done'
    } catch (e) {
      fail(e)
    } finally {
      busy = false
    }
  }

  function finish() {
    ui.onboardingComplete = true
    ui.treeVersion += 1
  }

  /// Use an existing folder: initialized in place when it is not a repository.
  async function useLocal() {
    if (busy) return
    // Busy before the picker: the wizard must not move on while it is open.
    busy = true
    error = null
    const picked = await open({ directory: true, multiple: false, title: 'Open a local repository' })
    if (typeof picked !== 'string' || !picked) {
      busy = false
      return
    }
    try {
      const result = await invoke<LocalRepo>('git_use_local', { path: picked })
      ui.repoPath = result.to
      localRepo = result
      applyGitStatus(await invoke<GitStatus>('git_status'))
      ui.treeVersion += 1
      step = 'done'
    } catch (e) {
      fail(e)
    } finally {
      busy = false
    }
  }

</script>

<div class="overlay">
  <div class="card">
    {#if step === 'welcome'}
      <h1>GitPad</h1>
      <p>Markdown notes in a Git repository you own. Everything is saved locally and pushed to GitHub.</p>
      <ul class="shortcuts">
        <li><kbd>Ctrl</kbd>+<kbd>P</kbd> command palette</li>
        <li><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>P</kbd> markdown preview</li>
        <li><kbd>Esc</kbd> close palette</li>
      </ul>
      <button class="primary" disabled={busy} onclick={() => (step = 'auth')}>Get Started</button>
      <button disabled={busy} onclick={useLocal}>{busy ? 'Opening…' : 'Use a local folder…'}</button>
      <p class="muted">
        An existing folder works too; GitPad initializes it when it is not a repository yet.
      </p>
    {:else if step === 'auth'}
      <h1>Connect GitHub</h1>
      <p>GitPad stores the credential in your OS keyring.</p>
      <input
        bind:value={token}
        placeholder="ghp_… personal access token (scope: repo)"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      />
      <div class="row">
        <a href="https://github.com/settings/tokens" onclick={(e) => { e.preventDefault(); void openExternal('https://github.com/settings/tokens') }}>Create a token</a>
        <button class="primary" disabled={busy || !token.trim()} onclick={validatePat}>{busy ? 'Validating…' : 'Validate'}</button>
      </div>
      <div class="divider">or</div>
      <GithubLogin label="Login with GitHub" onsuccess={() => { void loadUser(true); goRepo() }} />
    {:else if step === 'repo'}
      <h1>Repository</h1>
      {#if busy}
        <p class="stage"><span class="loading"></span>{STAGES[stage]}</p>
      {:else if repo}
        <p><code>{repo.fullName}</code> {repo.private ? '(private)' : ''}</p>
        <p class="muted">{repo.cloneUrl}</p>
        <div class="folder">
          <p class="muted">{folder || 'Default app folder'}</p>
          <button disabled={busy} onclick={chooseFolder}>{folder ? 'Change…' : 'Choose…'}</button>
        </div>
        <button class="primary" disabled={busy} onclick={clone}>Clone &amp; Open</button>
      {:else}
        <div class="row">
          <button onclick={() => (step = 'auth')}>Back</button>
          <button class="primary" onclick={goRepo}>Retry</button>
        </div>
      {/if}
    {:else}
      <h1>You're set</h1>
      {#if localRepo}
        <p><code>{localRepo.to}</code></p>
        <p class="muted">
          {localRepo.remote
            ? 'Pushes go to its origin remote.'
            : 'No remote: commits stay on this machine until you add one.'}
        </p>
      {:else}
        <p>Edits commit locally right away. Pushes happen on idle, when the window loses focus and on close.</p>
      {/if}
      <p class="muted">Ctrl+Shift+P toggles the markdown preview.</p>
      <button class="primary" onclick={finish}>Start Editing</button>
    {/if}
    {#if user}
      <p class="muted">Signed in as {user.login}</p>
    {/if}
    {#if storageNote}
      <p class="warn">{storageNote}</p>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
    <button class="skip" onclick={onskip}>Skip configuration</button>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-0);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 380px;
    max-width: calc(100vw - 32px);
    padding: 20px;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-2);
  }
  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  code {
    font-family: ui-monospace, monospace;
    font-size: 12px;
  }
  .muted {
    color: var(--muted);
    font-size: 11px;
    overflow-wrap: anywhere;
  }
  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0 0 4px;
    padding: 0;
    list-style: none;
    color: var(--muted);
    font-size: 11px;
  }
  kbd {
    padding: 1px 4px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-2);
    font: inherit;
  }
  .error {
    color: var(--err);
    font-size: 11px;
    overflow-wrap: anywhere;
  }

  .warn {
    color: var(--warn);
    font-size: 11px;
    overflow-wrap: anywhere;
  }
  .stage {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .folder p {
    flex: 1;
    min-width: 0;
  }
  .divider {
    color: var(--muted);
    font-size: 11px;
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }  a {
    color: var(--accent-fg);
    font-size: 12px;
  }
  button {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-2);
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary {
    color: var(--ok);
  }
  .skip {
    align-self: center;
    padding: 2px 4px;
    border: 0;
    color: var(--muted);
    font-size: 11px;
    text-decoration: underline;
  }
  .skip:hover {
    color: var(--text);
  }
</style>
