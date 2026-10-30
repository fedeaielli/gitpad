<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { ui } from '../stores/ui.svelte'
  import { applyGitStatus, type GitStatus } from '../stores/git.svelte'

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
  interface DeviceCode {
    deviceCode: string
    userCode: string
    verificationUri: string
    expiresIn: number
    interval: number
  }

  type Step = 'welcome' | 'auth' | 'repo' | 'done'

  let { onskip }: { onskip: () => void } = $props()

  let step = $state<Step>('welcome')
  let token = $state('')
  let user = $state<UserInfo | null>(null)
  let device = $state<DeviceCode | null>(null)
  let repo = $state<RepoInfo | null>(null)
  let busy = $state(false)
  let error = $state<string | null>(null)

  function fail(e: unknown) {
    error = String(e)
    busy = false
  }

  async function validatePat() {
    busy = true
    error = null
    try {
      const info = await invoke<UserInfo>('github_validate_token', { token: token.trim() })
      await invoke('auth_store', { key: 'github_token', value: token.trim() })
      user = info
      goRepo()
    } catch (e) {
      fail(e)
    }
  }

  async function startDevice() {
    busy = true
    error = null
    try {
      device = await invoke<DeviceCode>('github_device_flow_start')
      await openUrl(device.verificationUri)
      await invoke('github_device_flow_poll', { deviceCode: device.deviceCode })
    } catch (e) {
      fail(e)
    }
  }

  async function goRepo() {
    step = 'repo'
    busy = true
    error = null
    try {
      repo = await invoke<RepoInfo>('github_init_repo')
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }

  async function clone() {
    if (!repo) return
    busy = true
    error = null
    try {
      await invoke('git_clone', { url: repo.cloneUrl })
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

  onMount(() => {
    const unlistens: Array<() => void> = []
    void listen('auth-success', async () => {
      if (step !== 'auth') return
      busy = false
      goRepo()
    }).then((fn) => unlistens.push(fn))
    void listen<string>('auth-error', (e) => {
      if (step !== 'auth') return
      busy = false
      error = e.payload
    }).then((fn) => unlistens.push(fn))
    return () => unlistens.forEach((fn) => fn())
  })
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
      <button class="primary" onclick={() => (step = 'auth')}>Get Started</button>
    {:else if step === 'auth'}
      <h1>Connect GitHub</h1>
      <p>GitPad stores the credential in your OS keyring.</p>
      <input bind:value={token} placeholder="ghp_… personal access token (scope: repo)" />
      <div class="row">
        <a href="https://github.com/settings/tokens" onclick={(e) => { e.preventDefault(); void openUrl('https://github.com/settings/tokens') }}>Create a token</a>
        <button class="primary" disabled={busy || !token.trim()} onclick={validatePat}>Validate</button>
      </div>
      <div class="divider">or</div>
      <button disabled={busy} onclick={startDevice}>
        {device ? 'Waiting for GitHub…' : 'Login with GitHub'}
      </button>
      {#if device}
        <p class="code">Code: <strong>{device.userCode}</strong></p>
        <p class="muted">Open the page and enter the code. Checking every {device.interval}s.</p>
      {/if}
    {:else if step === 'repo'}
      <h1>Repository</h1>
      {#if busy}
        <p>Looking for or creating <code>gitpad_notes</code>…</p>
      {:else if repo}
        <p><code>{repo.fullName}</code> {repo.private ? '(private)' : ''}</p>
        <p class="muted">{repo.cloneUrl}</p>
        <button class="primary" disabled={busy} onclick={clone}>Clone &amp; Open</button>
      {:else}
        <p>Could not set up the repository.</p>
        <button onclick={goRepo}>Retry</button>
      {/if}
    {:else}
      <h1>You're set</h1>
      <p>Edits commit locally right away. Pushes happen on idle, when the window loses focus and on close.</p>
      <p class="muted">Ctrl+Shift+P toggles the markdown preview.</p>
      <button class="primary" onclick={finish}>Start Editing</button>
    {/if}
    {#if user}
      <p class="muted">Signed in as {user.login}</p>
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
    background: var(--bg);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 380px;
    max-width: calc(100vw - 32px);
    padding: 20px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 6px;
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
    border-radius: 3px;
    background: var(--bg-raised);
    font: inherit;
  }
  .error {
    color: var(--err);
    font-size: 11px;
    overflow-wrap: anywhere;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .divider {
    color: var(--muted);
    font-size: 11px;
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .code {
    font-size: 14px;
    letter-spacing: 0.1em;
  }
  a {
    color: var(--accent);
    font-size: 12px;
  }
  button {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-raised);
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
