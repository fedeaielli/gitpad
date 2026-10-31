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
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        d="M12 9C10.3425 9 9.00002 10.3425 9.00002 12C9.00002 13.6575 10.3425 15 12 15C13.6575 15 15 13.6575 15 12C15 10.3425 13.6575 9 12 9ZM12 13.5C11.172 13.5 10.5 12.828 10.5 12C10.5 11.172 11.172 10.5 12 10.5C12.828 10.5 13.5 11.172 13.5 12C13.5 12.828 12.828 13.5 12 13.5ZM21.8475 14.5725L19.9185 12.942C19.8675 12.8985 19.8195 12.8505 19.776 12.7995C19.332 12.279 19.3965 11.5005 19.9185 11.058L21.8475 9.4275C22.0395 9.2655 22.113 9.0045 22.0365 8.766C21.579 7.3545 20.823 6.06 19.8285 4.962C19.7085 4.83 19.5405 4.758 19.368 4.758C19.2975 4.758 19.227 4.77 19.1595 4.794L16.779 5.6415C16.716 5.664 16.65 5.682 16.584 5.694C16.509 5.7075 16.434 5.715 16.3605 5.715C15.7725 5.715 15.2505 5.298 15.141 4.701L14.6865 2.223C14.6415 1.977 14.451 1.782 14.205 1.7295C13.485 1.5765 12.7485 1.5 12.0015 1.5C11.2545 1.5 10.5165 1.578 9.79652 1.7295C9.55052 1.782 9.36002 1.977 9.31502 2.223L8.86202 4.701C8.85002 4.767 8.83202 4.8315 8.80952 4.8945C8.62802 5.4 8.15102 5.715 7.64102 5.715C7.50302 5.715 7.36202 5.691 7.22402 5.643L4.84352 4.7955C4.77602 4.7715 4.70402 4.7595 4.63502 4.7595C4.46252 4.7595 4.29452 4.8315 4.17452 4.9635C3.17852 6.0615 2.42402 7.356 1.96502 8.7675C1.88702 9.006 1.96202 9.267 2.15402 9.429L4.08302 11.0595C4.13402 11.103 4.18202 11.151 4.22552 11.202C4.66952 11.7225 4.60502 12.501 4.08302 12.9435L2.15402 14.574C1.96202 14.736 1.88852 14.997 1.96502 15.2355C2.42252 16.647 3.17852 17.9415 4.17452 19.0395C4.29452 19.1715 4.46252 19.2435 4.63502 19.2435C4.70552 19.2435 4.77602 19.2315 4.84352 19.2075L7.22402 18.36C7.28702 18.3375 7.35302 18.3195 7.41902 18.3075C7.49402 18.294 7.56902 18.288 7.64252 18.288C8.23052 18.288 8.75252 18.705 8.86202 19.302L9.31502 21.78C9.36002 22.026 9.55052 22.221 9.79652 22.2735C10.5165 22.4265 11.2545 22.503 12.0015 22.503C12.7485 22.503 13.4865 22.425 14.205 22.2735C14.451 22.221 14.6415 22.026 14.6865 21.78L15.141 19.302C15.153 19.236 15.171 19.1715 15.1935 19.1085C15.375 18.603 15.852 18.288 16.362 18.288C16.5 18.288 16.641 18.312 16.779 18.36L19.158 19.2075C19.227 19.2315 19.2975 19.2435 19.3665 19.2435C19.539 19.2435 19.707 19.1715 19.827 19.0395C20.823 17.9415 21.5775 16.647 22.035 15.2355C22.113 14.997 22.038 14.736 21.846 14.574L21.8475 14.5725Z"
      />
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
  .settings svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
    stroke: none;
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
