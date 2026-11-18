<script lang="ts">
  import { onMount } from 'svelte'
  import { fade, fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { prefersReducedMotion } from 'svelte/motion'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'

  import { open } from '@tauri-apps/plugin-dialog'
  import { openExternal } from '../utils/open'
  import { ui } from '../stores/ui.svelte'
  import { applyGitStatus, errorMessage, type GitStatus } from '../stores/git.svelte'
  import { pushToast } from '../stores/toasts.svelte'
  import { loadUser, signInWithPat } from '../stores/auth.svelte'
  import GithubLogin from './GithubLogin.svelte'

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

  const STEPS = [
    { id: 'welcome', name: 'Start' },
    { id: 'auth', name: 'GitHub' },
    { id: 'repo', name: 'Repo' },
    { id: 'done', name: 'Ready' },
  ] as const

  let { onskip }: { onskip: () => void } = $props()

  let step = $state<Step>('welcome')
  /// A step once reached stays open, the way a linear stepper behaves.
  let furthest = $state(0)
  const index = $derived(STEPS.findIndex((s) => s.id === step))
  $effect(() => {
    if (index > furthest) furthest = index
  })

  /// Which way the last move went, so the new step enters the same way.
  let dir = $state<1 | -1>(1)
  function go(next: Step) {
    dir = STEPS.findIndex((s) => s.id === next) >= index ? 1 : -1
    step = next
  }

  const PANEL: Record<Step, { title: string; sub: string }> = {
    welcome: { title: 'GitPad', sub: 'Markdown notes · Git-backed' },
    auth: { title: 'Connect GitHub', sub: 'Credentials stored in your OS keyring' },
    repo: { title: 'Repository', sub: 'Select where to clone your notes' },
    done: { title: "You're set", sub: 'Everything is ready to go' },
  }
  let token = $state('')
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
      await signInWithPat(token.trim())
      goRepo()    } catch (e) {
      fail(e)
    }
  }

  async function goRepo() {
    go('repo')
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
      go('done')
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
      if (result.initialized) {
        pushToast(
          'Created a new repository here; the next save commits every file in this folder',
          'warning',
        )
      }
      applyGitStatus(await invoke<GitStatus>('git_status'))
      ui.treeVersion += 1
      go('done')
    } catch (e) {
      fail(e)
    } finally {
      busy = false
    }
  }

</script>

<!-- The overlay covers the title bar, so it carries the drag region itself:
     clicks on the card and its controls stay normal (Tauri skips those). -->
<div
  class="overlay"
  data-tauri-drag-region
  out:fade={{ duration: prefersReducedMotion.current ? 0 : 160, easing: cubicOut }}
>
  <div
    class="card"
    class:back={dir < 0}
    out:fly={{ y: 6, opacity: 1, duration: prefersReducedMotion.current ? 0 : 160, easing: cubicOut }}
  >
    <div class="panel" class:done={step === 'done'}>
      {#key step}
        <div
          class="badge"
          aria-hidden="true"
          in:fade={{ duration: prefersReducedMotion.current ? 0 : 120 }}
        >
            {#if step === 'welcome'}
            <span class="mark"></span>
          {:else if step === 'auth'}
            <svg viewBox="0 0 16 16" width="20" height="20">
              <path
                fill="currentColor"
                d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
              />
            </svg>
          {:else if step === 'repo'}
            <svg viewBox="0 0 16 16" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.3">
              <circle cx="4.5" cy="3.5" r="1.7" />
              <circle cx="4.5" cy="12.5" r="1.7" />
              <circle cx="11.5" cy="6.5" r="1.7" />
              <path d="M4.5 5.2v5.6M9.8 6.5H8a2.5 2.5 0 0 0-2.5 2.5v1.3" />
            </svg>
          {:else}
            <svg
              viewBox="0 0 16 16"
              width="20"
              height="20"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path pathLength="1" d="M3 8.6 6.4 12 13 4.6" />
            </svg>
          {/if}
        </div>
        <div in:fade={{ duration: prefersReducedMotion.current ? 0 : 120 }}>
          <h1>{PANEL[step].title}</h1>
          <p class="sub">{PANEL[step].sub}</p>
        </div>
      {/key}
    </div>
    <ol class="steps" aria-label="Setup steps">
      {#each STEPS as s, i}
        <li class:done={i < index} class:active={i === index}>
          <button
            type="button"
            class="step"
            disabled={busy || i > furthest}
            aria-current={i === index ? 'step' : undefined}
            onclick={() => go(s.id)}
          >
            <span class="dot" aria-hidden="true">
              {#if i < index}
                <svg
                  viewBox="0 0 16 16"
                  width="10"
                  height="10"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M3 8.4 6.3 12 13 4.6" />
                </svg>
              {:else}
                {i + 1}
              {/if}
            </span>
            <span class="name">{s.name}</span>
          </button>
        </li>
      {/each}
    </ol>
    {#if step === 'welcome'}
      <button class="btn btn-primary cta" disabled={busy} onclick={() => go('auth')}>
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
          <path
            fill="currentColor"
            d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
          />
        </svg>
        Get started with GitHub
      </button>
      <button class="btn full" disabled={busy} onclick={useLocal}>
        {busy ? 'Opening…' : 'Use a local folder'}
      </button>
    {:else if step === 'auth'}
      <label class="label" for="pat">Personal access token</label>
      <div class="field">
        <input
          id="pat"
          bind:value={token}
          placeholder="ghp_…  (scope: repo)"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          aria-invalid={!!error}
          aria-describedby={error ? 'pat-error' : undefined}
        />
        <button class="btn inline" disabled={busy || !token.trim()} onclick={validatePat}>
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
      {#if busy}
        <p class="stage"><span class="loading"></span>{STAGES[stage]}</p>
      {:else if repo}
        <p class="label">Selected repository</p>
        <div class="box">
          <p class="repo-name"><code>{repo.fullName}</code></p>
          {#if repo.private}<span class="pill">private</span>{/if}
        </div>
        <p class="label">Local folder</p>
        <div class="folder">
          <p class="pick" title={target || 'Default app folder'}>
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
              <path d="M1.8 4.2h4l1.4 1.6h7v7.4a.6.6 0 0 1-.6.6H2.4a.6.6 0 0 1-.6-.6Z" />
            </svg>
            <span class="path" class:set={!!target}>{target || 'Default app folder'}</span>
          </p>
          <button class="btn" disabled={busy} onclick={chooseFolder}>
            {folder ? 'Change…' : 'Choose…'}
          </button>
        </div>
        <button class="btn btn-primary cta" disabled={busy} onclick={clone}>Clone and open</button>
      {:else}
        <div class="row">
          <button class="btn" onclick={() => go('auth')}>Back</button>
          <button class="btn" onclick={goRepo}>Retry</button>
        </div>
      {/if}
    {:else}
      {#if localRepo}
        <div class="box">
          <p class="repo-name"><code>{localRepo.to}</code></p>
          <p class="repo-url">
            {localRepo.remote
              ? 'Pushes go to its origin remote.'
              : 'No remote: commits stay on this machine until you add one.'}
          </p>
        </div>
      {/if}
      <button class="btn btn-primary cta" onclick={finish}>Start editing</button>
    {/if}
    {#if step === 'auth' || step === 'repo'}
      <div class="foot">
        {#if step === 'repo' && !repo}
          <span></span>
        {:else}
          <button
            class="btn btn-ghost fbtn"
            disabled={busy}
            onclick={() => go(step === 'repo' ? 'auth' : 'welcome')}
          >
            Back
          </button>
        {/if}
        <button class="btn btn-ghost fbtn" disabled={busy} onclick={useLocal}>Continue offline</button>
      </div>
    {/if}
    {#if storageNote}
      <p class="warn">{storageNote}</p>
    {/if}
    {#if error}
      <p class="error" id="pat-error" role="alert">{error}</p>
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
    gap: var(--sp-3);
    width: 480px;
    max-width: calc(100vw - var(--sp-6));
    padding: var(--sp-6);
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--r-overlay);
    box-shadow: var(--shadow-2);
    animation:
      gp-fade-in var(--t-base) var(--ease-out),
      gp-panel-in var(--t-spring) var(--ease-spring);
  }
  /* Each step's blocks mount fresh, so the cascade replays per step. */
  .card > * {
    animation: gp-step-in var(--t-base) var(--ease-out) both;
  }
  .card > :nth-child(2) {
    animation-delay: 30ms;
  }
  .card > :nth-child(3) {
    animation-delay: 60ms;
  }
  .card > :nth-child(4) {
    animation-delay: 90ms;
  }
  .card > :nth-child(n + 5) {
    animation-delay: 120ms;
  }
  @keyframes gp-step-in {
    from {
      opacity: 0;
      transform: translateX(10px);
    }
  }
  /* A move backwards enters from the side it came from. */
  .card.back > * {
    animation-name: gp-step-back;
  }
  @keyframes gp-step-back {
    from {
      opacity: 0;
      transform: translateX(-10px);
    }
  }
  /* The path through the wizard: a dot and a label per step, aligned with the
     drift stepper, with the line between them turning accent once passed. */
  .steps {
    display: flex;
    align-items: center;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .steps li {
    display: flex;
    flex: 1;
    align-items: center;
    min-width: 0;
  }
  .steps li:last-child {
    flex: 0 0 auto;
  }
  .steps li:not(:last-child)::after {
    content: '';
    flex: 1;
    min-width: 20px;
    height: 1px;
    margin-left: var(--sp-2);
    background: var(--divider);
    transition: background-color var(--t-base) var(--ease-out);
  }
  .steps li.done::after {
    background: var(--accent);
  }
  .step {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    padding: var(--sp-05) var(--sp-1);
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    color: var(--muted);
    font-size: var(--fs-xs);
    white-space: nowrap;
  }
  .step:not(:disabled):hover {
    color: var(--text);
  }
  .step:disabled {
    opacity: 1;
    cursor: default;
  }
  li.active .step,
  li.done .step {
    color: var(--text);
  }
  .dot {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    width: 20px;
    height: 20px;
    border: 1px solid var(--border-strong);
    border-radius: 50%;
    font-size: var(--fs-2xs);
    font-variant-numeric: tabular-nums;
    transition:
      background-color var(--t-base) var(--ease-out),
      border-color var(--t-base) var(--ease-out),
      color var(--t-base) var(--ease-out),
      box-shadow var(--t-base) var(--ease-out);
  }
  li.active .dot {
    border-color: var(--accent);
    color: var(--text);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }
  li.done .dot {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
  /* The gradient panel rides over the card's top edge, the way the reference
     card does: inset sideways, raised above the border, carrying the title. */
  .panel {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    min-height: 96px;
    margin: calc(-1 * (var(--sp-6) + var(--sp-1))) var(--sp-3) 0;
    padding: 0 var(--sp-3);
    border-radius: var(--r-lg);
    background: linear-gradient(
      100deg,
      var(--accent) 0%,
      color-mix(in srgb, var(--accent) 72%, black) 100%
    );
    box-shadow: var(--shadow-1);
    color: var(--on-accent);
  }
  .badge {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--on-accent) 16%, transparent);
  }
  .mark {
    width: 20px;
    height: 24px;
    background: var(--on-accent);
    mask: url('/logo.png') center / contain no-repeat;
  }
  .sub {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .panel .sub {
    color: color-mix(in srgb, var(--on-accent) 76%, transparent);
  }
  /* The last card is the only one that means "done": the check draws in. */
  .panel.done .badge {
    background: color-mix(in srgb, var(--on-accent) 22%, transparent);
    animation: gp-check-pop var(--t-base) var(--ease-out);
  }
  .panel.done .badge svg path {
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: gp-check-draw var(--t-fast) var(--t-base) var(--ease-out) forwards;
  }
  @keyframes gp-check-pop {
    from {
      opacity: 0;
      transform: scale(0.86);
    }
    60% {
      opacity: 1;
      transform: scale(1.04);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  @keyframes gp-check-draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  /* The footer carries the sideways moves: back, and the offline way out. */
  .foot {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-2);
  }
  .fbtn {
    font-size: var(--fs-sm);
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
    gap: var(--sp-05);
    padding: var(--sp-0) var(--sp-0) var(--sp-0) var(--sp-25);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--surface-0);
  }
  .field:focus-within {
    outline: var(--focus-ring);
    outline-offset: -2px;
    border-color: var(--border-strong);
  }
  .field input:focus-visible,
  .field .inline:focus-visible {
    outline: none;
  }
  .field input {
    flex: 1;
    height: auto;
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
    gap: var(--sp-05);
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
    gap: var(--sp-2);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
  }
  .folder {
    display: flex;
    align-items: stretch;
    gap: var(--sp-2);
  }
  /* Read-only on purpose: one trigger for the picker, not two. */
  .pick {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
    margin: 0;
    padding: var(--sp-2) var(--sp-25);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-0);
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
  .box {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-1);
    padding: var(--sp-3);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-0);
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
    padding: var(--sp-0) var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-pill);
    color: var(--muted);
    font-size: var(--fs-xs);
    line-height: 1.6;
  }
  .divider {
    display: flex;
    align-items: center;
    gap: var(--sp-25);
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
  /* The one action worth the accent: every card keeps its own weight. */
  .cta {
    width: 100%;
  }
  .full {
    width: 100%;
  }
  .skip {
    align-self: center;
    padding: var(--sp-0) var(--sp-05);
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
