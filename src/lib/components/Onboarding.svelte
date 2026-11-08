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
    /// Absolute default clone path, already localised by the backend.
    defaultFolder: string
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
  /// The default destination is <documents>/<repo>, and only the backend knows
  /// where "documents" is called in this locale.
  const target = $derived(folder || repo?.defaultFolder || '')
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
    // The note is cosmetic: never let a slow keyring probe hold the wizard.
    void invoke<boolean>('auth_keyring_ok')
      .then((ok) => {
        if (!ok) {
          storageNote =
            'Signed in, but the OS keyring is unavailable: the token is kept in a protected file on this machine.'
        }
      })
      .catch(() => {})
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
      <div class="head">
        <div class="brand"><span class="mark" aria-hidden="true"></span></div>
        <div>
          <h1>GitPad</h1>
          <p class="sub">Markdown notes · Git-backed</p>
        </div>
      </div>
      <hr class="rule" />
      <p class="lead">
        Markdown notes in a Git repository you own. Saved locally and pushed to GitHub.
      </p>
      <p class="label">Keyboard shortcuts</p>
      <ul class="shortcuts">
        <li>
          <span class="keys"><kbd>Ctrl</kbd><kbd>P</kbd></span><span>Command palette</span>
        </li>
        <li>
          <span class="keys"><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>P</kbd></span><span
            >Markdown preview</span
          >
        </li>
        <li><span class="keys"><kbd>Esc</kbd></span><span>Close palette</span></li>
      </ul>
      <hr class="rule" />
      <button class="cta" disabled={busy} onclick={() => (step = 'auth')}>
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
          <path
            fill="currentColor"
            d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
          />
        </svg>
        Get started with GitHub
      </button>
      <button class="full" disabled={busy} onclick={useLocal}>
        {busy ? 'Opening…' : 'Use a local folder'}
      </button>
      <p class="helper">An existing folder works too — GitPad initializes it if needed.</p>
    {:else if step === 'auth'}
      <div class="head">
        <div class="brand">
          <svg viewBox="0 0 16 16" width="20" height="20" aria-hidden="true">
            <path
              fill="currentColor"
              d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
            />
          </svg>
        </div>
        <div>
          <h1>Connect GitHub</h1>
          <p class="sub">Credentials stored in your OS keyring</p>
        </div>
      </div>
      <hr class="rule" />
      <label class="label" for="pat">Personal access token</label>
      <div class="field">
        <input
          id="pat"
          bind:value={token}
          placeholder="ghp_…  (scope: repo)"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
        />
        <button class="primary inline" disabled={busy || !token.trim()} onclick={validatePat}>
          {busy ? 'Validating…' : 'Validate'}
        </button>
      </div>
      <a
        class="link"
        href="https://github.com/settings/tokens"
        onclick={(e) => { e.preventDefault(); void openExternal('https://github.com/settings/tokens') }}
      >
        Create a token on GitHub
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
          <path fill="none" stroke="currentColor" stroke-width="1.5" d="M6 3h7v7M13 3 4.5 11.5" />
        </svg>
      </a>
      <div class="divider">or</div>
      <GithubLogin label="Login with GitHub" onsuccess={() => { void loadUser(true); goRepo() }} />
    {:else if step === 'repo'}
      <div class="head">
        <div class="brand">
          <svg viewBox="0 0 16 16" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
            <circle cx="4.5" cy="3.5" r="1.7" />
            <circle cx="4.5" cy="12.5" r="1.7" />
            <circle cx="11.5" cy="6.5" r="1.7" />
            <path d="M4.5 5.2v5.6M9.8 6.5H8a2.5 2.5 0 0 0-2.5 2.5v1.3" />
          </svg>
        </div>
        <div>
          <h1>Repository</h1>
          <p class="sub">Select where to clone your notes</p>
        </div>
      </div>
      <hr class="rule" />
      {#if busy}
        <p class="stage"><span class="loading"></span>{STAGES[stage]}</p>
      {:else if repo}
        <p class="label">Selected repository</p>
        <div class="box">
          <p class="repo-name"><code>{repo.fullName}</code></p>
          {#if repo.private}<span class="pill">private</span>{/if}
          <p class="repo-url"><code>{repo.cloneUrl}</code></p>
        </div>
        <p class="label">Local folder</p>
        <div class="folder">
          <p class="pick" title={target || 'Default app folder'}>
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
              <path d="M1.8 4.2h4l1.4 1.6h7v7.4a.6.6 0 0 1-.6.6H2.4a.6.6 0 0 1-.6-.6Z" />
            </svg>
            <span class="path" class:set={!!target}>{target || 'Default app folder'}</span>
          </p>
          <button class="choose" disabled={busy} onclick={chooseFolder}>
            {folder ? 'Change…' : 'Choose…'}
          </button>
        </div>
        <hr class="rule" />
        <button class="cta" disabled={busy} onclick={clone}>Clone and open</button>
      {:else}
        <div class="row">
          <button onclick={() => (step = 'auth')}>Back</button>
          <button class="primary" onclick={goRepo}>Retry</button>
        </div>
      {/if}
    {:else}
      <div class="head">
        <div class="brand done">
          <svg viewBox="0 0 16 16" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M3 8.6 6.4 12 13 4.6" />
          </svg>
        </div>
        <div>
          <h1>You're set</h1>
          <p class="sub">Everything is ready to go</p>
        </div>
      </div>
      <hr class="rule" />
      {#if localRepo}
        <div class="box">
          <p class="repo-name"><code>{localRepo.to}</code></p>
          <p class="repo-url">
            {localRepo.remote
              ? 'Pushes go to its origin remote.'
              : 'No remote: commits stay on this machine until you add one.'}
          </p>
        </div>
      {:else}
        <p class="lead">Edits commit locally right away. Pushes happen on idle, when the window loses focus and on close.</p>
      {/if}
      <p class="hintbox"><kbd>Ctrl</kbd><kbd>P</kbd> opens the command palette</p>
      <hr class="rule" />
      <button class="cta" onclick={finish}>Start editing</button>
    {/if}
    {#if user && step !== 'done'}
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
    width: 420px;
    max-width: calc(100vw - 32px);
    padding: 20px;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: var(--r-overlay);
    box-shadow: var(--shadow-2);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .brand {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 44px;
    height: 44px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-2);
  }
  .mark {
    width: 20px;
    height: 24px;
    background: var(--muted);
    mask: url('/logo.png') center / contain no-repeat;
  }
  .sub {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .lead {
    font-size: var(--fs-lg);
    line-height: 1.45;
  }
  /* The only card that means "done", so the tile carries the success colour. */
  .brand.done {
    border-color: var(--ok);
    background: var(--ok-soft);
    color: var(--ok);
  }
  .hintbox {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
  }
  /* --border equals --surface-3: inside this card the rule would vanish. */
  .rule {
    margin: 0;
    border: 0;
    border-top: 1px solid var(--divider);
  }
  .label {
    color: var(--muted);
    font-size: var(--fs-xs);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  /* The ring belongs to the whole field, so the button can sit inside it. */
  .field {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 3px 3px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    background: var(--surface-1);
  }
  .field:focus-within {
    border-color: var(--accent);
  }
  .field input {
    flex: 1;
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .inline {
    flex: 0 0 auto;
  }
  .link {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    gap: 4px;
    text-decoration: none;
  }
  .link:hover {
    text-decoration: underline;
  }
  h1 {
    margin: 0;
    font-size: var(--fs-xl);
    font-weight: 600;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  code {
    font-family: ui-monospace, monospace;
    font-size: var(--fs-sm);
  }
  .muted {
    color: var(--muted);
    font-size: var(--fs-xs);
    overflow-wrap: anywhere;
  }
  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .shortcuts li {
    display: grid;
    grid-template-columns: 150px 1fr;
    align-items: center;
    gap: 8px;
  }
  .keys {
    display: flex;
    gap: 4px;
  }
  kbd {
    padding: 3px 8px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-2);
    font-family: ui-monospace, monospace;
    font-size: var(--fs-sm);
  }
  .error {
    color: var(--err);
    font-size: var(--fs-xs);
    overflow-wrap: anywhere;
  }

  .warn {
    color: var(--warn);
    font-size: var(--fs-xs);
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
    align-items: stretch;
    gap: 8px;
  }
  /* Read-only on purpose: one trigger for the picker, not two. */
  .pick {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 0;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-1);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .pick svg {
    flex: 0 0 auto;
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .path.set {
    color: var(--text);
  }
  .choose {
    flex: 0 0 auto;
    padding: 0 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    background: var(--surface-1);
    font-size: var(--fs-sm);
  }
  .choose:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .box {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
  }
  .repo-name code {
    font-size: var(--fs-lg);
  }
  .repo-url {
    max-width: 100%;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .pill {
    padding: 1px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-size: var(--fs-xs);
    line-height: 1.6;
  }
  .divider {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: var(--fs-xs);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .divider::before,
  .divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--divider);
  }
  a {
    color: var(--accent-fg);
    font-size: var(--fs-sm);
  }
  button {
    padding: 6px 12px;
    border: 1px solid var(--border-strong);
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
  /* The one action worth the accent: every card keeps its own weight. */
  .cta {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 14px;
    border: 1px solid var(--accent);
    border-radius: var(--r-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--fs-sm);
  }
  .full {
    padding: 8px 14px;
    border-radius: var(--r-md);
    font-size: var(--fs-sm);
  }
  .helper {
    color: var(--muted);
    font-size: var(--fs-xs);
    text-align: center;
  }
  .skip {
    align-self: center;
    padding: 2px 4px;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: var(--fs-xs);
  }
  .skip:hover {
    color: var(--text);
    text-decoration: underline;
  }
</style>
