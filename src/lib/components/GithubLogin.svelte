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

  onMount(() => {
    void invoke<boolean>('github_device_flow_available')
      .catch(() => false)
      .then((ok) => (available = ok))

    // The backend polls in its own task and reports through events; these only
    // matter while a flow started here is still waiting.
    const unlistens: Array<() => void> = []
    void listen('auth-success', () => {
      if (!busy) return
      busy = false
      code = null
      onsuccess()
    }).then((fn) => unlistens.push(fn))
    void listen<string>('auth-error', (e) => {
      if (!busy) return
      busy = false
      error = e.payload
    }).then((fn) => unlistens.push(fn))
    return () => unlistens.forEach((fn) => fn())
  })

  async function start() {
    busy = true
    error = ''
    copied = false
    try {
      code = await invoke<DeviceCode>('github_device_flow_start')
      await openExternal(code.verificationUri)
      await invoke('github_device_flow_poll', { deviceCode: code.deviceCode })
    } catch (e) {
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
      {busy ? 'Waiting for GitHub…' : label}
    </button>
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
    align-self: flex-start;
    padding: var(--sp-1) var(--sp-3);
    border: 1px solid var(--accent);
    border-radius: var(--r-md);
    background: var(--accent);
    color: var(--on-accent);
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
  .error {
    margin: 0;
    color: var(--err);
  }
</style>
