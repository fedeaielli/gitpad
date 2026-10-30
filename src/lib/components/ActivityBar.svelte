<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ui } from '../stores/ui.svelte'
  import { gitStatus } from '../stores/git.svelte'

  interface UserInfo {
    login: string
    avatarUrl: string
  }

  let user = $state<UserInfo | null>(null)

  async function loadUser() {
    user = await invoke<UserInfo>('github_get_user').catch(() => null)
  }

  onMount(() => {
    void loadUser()
    // Refresh after login/logout without restarting the app.
    window.addEventListener('focus', loadUser)
    return () => window.removeEventListener('focus', loadUser)
  })

  // Clicking the active view folds the panel, like VS Code.
  function select(view: 'files' | 'git') {
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
    class="item"
    class:active={ui.activeView === 'files' && ui.sidebarOpen}
    title="Explorer"
    aria-label="Explorer"
    onclick={() => select('files')}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3 2h5l1.5 1.5H13V14H3z" />
    </svg>
  </button>
  <button
    class="item"
    class:active={ui.activeView === 'git' && ui.sidebarOpen}
    title="Source control"
    aria-label="Source control"
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

  <div class="spacer"></div>

  <button
    class="item avatar"
    title={user ? `Account: ${user.login}` : 'Account'}
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
    class="item settings"
    title="Settings"
    aria-label="Settings"
    onclick={() => (ui.settingsOpen = true)}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="2.2" />
      <path d="M8 1.5v1.8M8 12.7v1.8M1.5 8h1.8M12.7 8h1.8M3.4 3.4l1.3 1.3M11.3 11.3l1.3 1.3M12.6 3.4l-1.3 1.3M4.7 11.3l-1.3 1.3" />
    </svg>
  </button>
</nav>

<style>
  .rail {
    display: flex;
    flex: 0 0 44px;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 4px 0;
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    color: var(--muted);
  }
  .item:hover {
    color: var(--text);
  }
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
    top: 6px;
    right: 5px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 7px;
    background: var(--accent);
    color: #fff;
    font-size: 9px;
    line-height: 14px;
    text-align: center;
  }
  .spacer {
    flex: 1;
  }
</style>
