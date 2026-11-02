<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ui } from '../stores/ui.svelte'
  import { gitStatus } from '../stores/git.svelte'
  import { tip } from '../utils/tooltip'

  interface UserInfo {
    login: string
    avatarUrl: string
  }

  let user = $state<UserInfo | null>(null)

  async function loadUser() {
    user = await invoke<UserInfo>('github_get_user').catch(() => null)
  }

  onMount(() => {
    // Off the startup path: the request and avatar fetch can wait for first paint.
    const timer = setTimeout(() => void loadUser(), 1200)
    // Refresh after login/logout without restarting the app.
    window.addEventListener('focus', loadUser)
    return () => {
      clearTimeout(timer)
      window.removeEventListener('focus', loadUser)
    }
  })

  // Clicking the active view folds the panel, like VS Code.
  function select(view: 'files' | 'git' | 'history') {
    if (ui.activeView === view && ui.sidebarOpen) {
      ui.sidebarOpen = false
      return
    }
    ui.activeView = view
    ui.sidebarOpen = true
  }

  const changed = $derived(gitStatus.modifiedFiles.length)
</script>

<nav class="rail" aria-label="Primary">
  <button
    type="button"
    class="item"
    class:active={ui.activeView === 'files'}
    class:open={ui.sidebarOpen}
    use:tip={'Explorer'}
    aria-label="Explorer"
    aria-pressed={ui.activeView === 'files' && ui.sidebarOpen}
    onclick={() => select('files')}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3 2h5l1.5 1.5H13V14H3z" />
    </svg>
  </button>
  <button
    type="button"
    class="item"
    class:active={ui.activeView === 'git'}
    class:open={ui.sidebarOpen}
    use:tip={'Source control'}
    aria-label="Source control"
    aria-pressed={ui.activeView === 'git' && ui.sidebarOpen}
    onclick={() => select('git')}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="5" cy="3.5" r="1.6" />
      <circle cx="5" cy="12.5" r="1.6" />
      <circle cx="11" cy="6" r="1.6" />
      <path d="M5 5.1v5.8M6.6 3.5h2c1.2 0 1.8.7 1.8 1.9v.3" />
    </svg>
    {#if changed > 0}
      <span class="count">{changed}</span>
    {/if}
  </button>
  <button
    type="button"
    class="item"
    class:active={ui.activeView === 'history'}
    class:open={ui.sidebarOpen}
    use:tip={'History'}
    aria-label="History"
    aria-pressed={ui.activeView === 'history' && ui.sidebarOpen}
    onclick={() => select('history')}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="5.6" />
      <path d="M8 5v3.2l2.2 1.3" />
    </svg>
  </button>

  <div class="spacer"></div>

  <button
    type="button"
    class="item avatar"
    use:tip={user ? `Account: ${user.login}` : 'Account'}
    aria-label="Account"
    onclick={() => (ui.settingsOpen = true)}
  >
    {#if user}
      <img src={user.avatarUrl} alt={user.login} />
    {:else}
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <circle cx="8" cy="5.5" r="2.5" />
        <path d="M3.5 13c.6-2.2 2.3-3.3 4.5-3.3s3.9 1.1 4.5 3.3" />
      </svg>
    {/if}
  </button>
  <button
    type="button"
    class="item settings"
    use:tip={'Settings'}
    aria-label="Settings"
    onclick={() => (ui.settingsOpen = true)}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="3" />
      <path
        d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
      />
    </svg>
  </button>
</nav>

<style>
  .rail {
    display: flex;
    flex: 0 0 var(--w-activitybar);
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 4px;
    border-radius: var(--r-lg);
    background: var(--surface-1);
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: calc(var(--w-activitybar) - 8px);
    height: calc(var(--w-activitybar) - 8px);
    border-radius: var(--r-lg);
    color: var(--muted);
  }
  .item:hover {
    color: var(--text);
  }
  .item:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -2px;
  }
  /* The view stays selected when the panel is closed; only the bar fades. */
  .item.active {
    color: var(--text);
  }
  .item.active::before {
    position: absolute;
    top: 8px;
    bottom: 8px;
    left: 0;
    width: 2px;
    border-radius: 0 2px 2px 0;
    background: var(--accent);
    content: '';
  }
  .item.active:not(.open)::before {
    background: var(--muted);
    opacity: 0.4;
  }
  .item svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .avatar img {
    width: 22px;
    height: 22px;
    border-radius: 50%;
  }
  .count {
    position: absolute;
    top: 4px;
    right: 4px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 7px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 9px;
    line-height: 14px;
    text-align: center;
  }
  .spacer {
    flex: 1;
  }
</style>
