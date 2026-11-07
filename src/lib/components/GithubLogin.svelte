<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { openExternal } from '../utils/open'
  import { errorMessage } from '../stores/git.svelte'

  interface DeviceCode {
    deviceCode: string
    userCode: string
    verificationUri: string
    expiresIn: number
    interval: number
  }

  interface Props {
    label?: string
    /// Called once the token is stored, so the caller can load the user.
    onsuccess: () => void
  }

  let { label = 'Sign in with GitHub', onsuccess }: Props = $props()

  /// False when the build has no OAuth client id, which hides the button.
  let available = $state(false)
  let code = $state<DeviceCode | null>(null)
  let busy = $state(false)
  let error = $state('')
  let copied = $state(false)
  /// Countdown to the device code deadline: the backend stops polling after it.
  let secondsLeft = $state(0)
  let ticker: ReturnType<typeof setInterval> | undefined

  const clock = $derived(
    `${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, '0')}`,
  )

  function stopTicker() {
    if (ticker) clearInterval(ticker)
    ticker = undefined
  }

  /// Ends the wait only. The backend keeps polling in its own task, so a code
  /// completed in the browser still stores the token; the next check finds it.
  function cancel() {
    stopTicker()
    busy = false
    code = null
    error = ''
  }

  onMount(() => {
    void invoke<boolean>('github_device_flow_available')
      .catch(() => false)
      .then((ok) => (available = ok))

    // The backend polls in its own task and reports through events; these only
    // matter while a flow started here is still waiting.
    const unlistens: Array<() => void> = []
    void listen('auth-success', () => {
      if (!busy) return
      stopTicker()
      busy = false
      code = null
      onsuccess()
    }).then((fn) => unlistens.push(fn))
    void listen<string>('auth-error', (e) => {
      if (!busy) return
      stopTicker()
      busy = false
      error = e.payload
    }).then((fn) => unlistens.push(fn))
    return () => {
      stopTicker()
      unlistens.forEach((fn) => fn())
    }
  })

  async function start() {
    busy = true
    error = ''
    copied = false
    try {
      code = await invoke<DeviceCode>('github_device_flow_start')
      secondsLeft = code.expiresIn
      stopTicker()
      ticker = setInterval(() => {
        secondsLeft = Math.max(0, secondsLeft - 1)
        if (secondsLeft === 0) cancel()
      }, 1000)
      // Not awaited: a cold browser start takes seconds, and the code has to be
      // on screen (and the poll running) before that wait is over.
      void openExternal(code.verificationUri)
      await invoke('github_device_flow_poll', { deviceCode: code.deviceCode })
    } catch (e) {
      stopTicker()
      error = errorMessage(e)
      busy = false
      code = null
    }
  }

  async function copy() {
    if (!code) return
    try {
      await navigator.clipboard.writeText(code.userCode)
      copied = true
    } catch (e) {
      error = errorMessage(e)
    }
  }
</script>

<div class="login">
  {#if !available}
    <p class="hint">
      GitHub login is unavailable: this build has no OAuth client id (GITHUB_CLIENT_ID).
    </p>
  {:else}
    <button class="start" disabled={busy} onclick={start}>
      <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
        <path
          fill="currentColor"
          d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
        />
      </svg>
      {busy ? `Waiting for GitHub… ${clock}` : label}
    </button>
    {#if busy}
      <button class="cancel" onclick={cancel}>Cancel</button>
    {/if}
    {#if code}
      <p class="code">
        Code <strong>{code.userCode}</strong>
        <button class="quiet" onclick={copy}>{copied ? 'Copied' : 'Copy'}</button>
      </p>
      <p class="hint">
        Enter it on
        <button class="quiet link" onclick={() => code && void openExternal(code.verificationUri)}>
          github.com/login/device
        </button>
        — checking every {code.interval}s.
      </p>
    {/if}
  {/if}
  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .login {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    min-width: 0;
  }
  .start {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 7px 12px;
    border: 1px solid var(--accent);
    border-radius: var(--r-md);
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .start:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .code {
    margin: 0;
    color: var(--text);
    font-size: var(--fs-md);
  }
  .code strong {
    margin: 0 6px;
    font-family: ui-monospace, monospace;
    letter-spacing: 0.08em;
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .quiet {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent-fg);
    font: inherit;
    cursor: pointer;
  }
  .link {
    text-decoration: underline;
  }
  .cancel {
    align-self: center;
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: var(--fs-xs);
    text-decoration: underline;
    cursor: pointer;
  }
  .error {
    margin: 0;
    color: var(--err);
  }
</style>
