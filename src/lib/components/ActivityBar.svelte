<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { openExternal } from '../utils/open'
  import { ui } from '../stores/ui.svelte'
  import { gitStatus } from '../stores/git.svelte'
  import { auth, forgetUser, loadUser } from '../stores/auth.svelte'
  import { pushToast } from '../stores/toasts.svelte'
  import { tip } from '../utils/tooltip'

  const user = $derived(auth.user)

  // The panel clips its content, so the menu is fixed and placed by hand.
  let menuOpen = $state(false)
  let menuPos = $state({ left: 0, bottom: 0 })
  let account: HTMLButtonElement | undefined

  // Keyboard users land on the first item, Escape brings them back.
  function focusFirst(node: HTMLElement) {
    node.querySelector<HTMLButtonElement>('.menu-item')?.focus()
  }

  function closeMenu(refocus = false) {
    menuOpen = false
    if (refocus) account?.focus()
  }

  function toggleMenu() {
    // Signed out: the account form is the only thing worth showing.
    if (!user) {
      ui.settingsOpen = true
      return
    }
    if (menuOpen) {
      closeMenu()
      return
    }
    const rect = account?.getBoundingClientRect()
    if (!rect) return
    menuPos = { left: Math.round(rect.right + 6), bottom: Math.round(window.innerHeight - rect.bottom) }
    menuOpen = true
  }

  async function gotoAccount() {
    closeMenu()
    if (!user) return
    await openExternal(`https://github.com/${user.login}`)
  }

  async function logout() {
    closeMenu()
    try {
      await invoke('auth_clear')
    } catch (e) {
      pushToast(`logout: ${e}`)
      return
    }
    forgetUser()
  }

  onMount(() => {
    // Off the startup path: the request and avatar fetch can wait for first paint.
    const timer = setTimeout(() => void loadUser(), 1200)
    // Refreshes only when the cached copy is stale, so a focus is free.
    const onFocus = () => void loadUser()
    window.addEventListener('focus', onFocus)
    return () => {
      clearTimeout(timer)
      window.removeEventListener('focus', onFocus)
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

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape' && menuOpen) {
      e.stopPropagation()
      closeMenu(true)
    }
  }}
/>

<nav class="rail" class:separated={ui.sidebarOpen} aria-label="Primary">
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
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        d="M7.5 22.5H17.595C17.07 23.4 16.11 24 15 24H7.5C4.185 24 1.5 21.315 1.5 18V6C1.5 4.89 2.1 3.93 3 3.405V18C3 20.475 5.025 22.5 7.5 22.5ZM21 8.121V18C21 19.6545 19.6545 21 18 21H7.5C5.8455 21 4.5 19.6545 4.5 18V3C4.5 1.3455 5.8455 0 7.5 0H12.879C13.4715 0 14.0505 0.24 14.4705 0.6585L20.3415 6.5295C20.766 6.954 21 7.5195 21 8.121ZM13.5 6.75C13.5 7.164 13.8375 7.5 14.25 7.5H19.1895L13.5 1.8105V6.75ZM19.5 18V9H14.25C13.0095 9 12 7.9905 12 6.75V1.5H7.5C6.672 1.5 6 2.1735 6 3V18C6 18.8265 6.672 19.5 7.5 19.5H18C18.828 19.5 19.5 18.8265 19.5 18Z"
      />
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
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        d="M21 8.25C21 6.1815 19.3185 4.5 17.25 4.5C15.1815 4.5 13.5 6.1815 13.5 8.25C13.5 10.023 14.739 11.5035 16.395 11.892C16.116 12.819 15.2655 13.5 14.25 13.5H9.75C8.9025 13.5 8.1285 13.7925 7.5 14.268V7.4235C9.21 7.0755 10.5 5.5605 10.5 3.75C10.5 1.6815 8.8185 0 6.75 0C4.6815 0 3 1.6815 3 3.75C3 5.562 4.29 7.0755 6 7.4235V16.575C4.29 16.923 3 18.438 3 20.2485C3 22.317 4.6815 23.9985 6.75 23.9985C8.8185 23.9985 10.5 22.317 10.5 20.2485C10.5 18.4755 9.261 16.995 7.605 16.6065C7.884 15.6795 8.7345 14.9985 9.75 14.9985H14.25C16.0845 14.9985 17.61 13.6725 17.931 11.9295C19.674 11.607 21 10.0845 21 8.25ZM4.5 3.75C4.5 2.5095 5.5095 1.5 6.75 1.5C7.9905 1.5 9 2.5095 9 3.75C9 4.9905 7.9905 6 6.75 6C5.5095 6 4.5 4.9905 4.5 3.75ZM9 20.25C9 21.4905 7.9905 22.5 6.75 22.5C5.5095 22.5 4.5 21.4905 4.5 20.25C4.5 19.0095 5.5095 18 6.75 18C7.9905 18 9 19.0095 9 20.25ZM17.25 10.5C16.0095 10.5 15 9.4905 15 8.25C15 7.0095 16.0095 6 17.25 6C18.4905 6 19.5 7.0095 19.5 8.25C19.5 9.4905 18.4905 10.5 17.25 10.5Z"
      />
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
      <path
        d="M7.99909 3C10.7605 3 12.9991 5.23858 12.9991 8C12.9991 10.7614 10.7605 13 7.99909 13C5.39117 13 3.2491 11.003 3.0195 8.45512C2.99471 8.1801 2.75167 7.97723 2.47664 8.00202C2.20161 8.0268 1.99875 8.26985 2.02353 8.54488C2.29916 11.6035 4.86898 14 7.99909 14C11.3128 14 13.9991 11.3137 13.9991 8C13.9991 4.68629 11.3128 2 7.99909 2C6.20656 2 4.59815 2.78613 3.49909 4.03138V2.5C3.49909 2.22386 3.27524 2 2.99909 2C2.72295 2 2.49909 2.22386 2.49909 2.5V5.5C2.49909 5.77614 2.72295 6 2.99909 6H3.08812C3.09498 6.00014 3.10184 6.00014 3.10868 6H5.99909C6.27524 6 6.49909 5.77614 6.49909 5.5C6.49909 5.22386 6.27524 5 5.99909 5H3.99863C4.91128 3.78495 6.36382 3 7.99909 3ZM7.99909 5.5C7.99909 5.22386 7.77524 5 7.49909 5C7.22295 5 6.99909 5.22386 6.99909 5.5V8.5C6.99909 8.77614 7.22295 9 7.49909 9H9.49909C9.77524 9 9.99909 8.77614 9.99909 8.5C9.99909 8.22386 9.77524 8 9.49909 8H7.99909V5.5Z"
      />
    </svg>
  </button>

  <div class="spacer"></div>

  <button
    type="button"
    class="item avatar"
    bind:this={account}
    use:tip={user ? `Account: ${user.login}` : 'Account'}
    aria-label="Account"
    aria-haspopup={user ? 'menu' : undefined}
    aria-expanded={user ? menuOpen : undefined}
    onclick={toggleMenu}
  >
    {#if user}
      <img src={user.avatarUrl} alt={user.login} />
    {:else}
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M6 5C6 3.89543 6.89543 3 8 3C9.10457 3 10 3.89543 10 5C10 6.10457 9.10457 7 8 7C6.89543 7 6 6.10457 6 5ZM5.49998 8L10.5 8C11.3284 8 12 8.67157 12 9.5C12 10.6161 11.541 11.5103 10.7879 12.1148C10.0466 12.7098 9.05308 13 8 13C6.94692 13 5.95342 12.7098 5.21215 12.1148C4.45897 11.5103 4 10.6161 4 9.5C4 8.67161 4.67156 8 5.49998 8ZM8 0C3.58172 0 0 3.58172 0 8C0 12.4183 3.58172 16 8 16C12.4183 16 16 12.4183 16 8C16 3.58172 12.4183 0 8 0ZM1 8C1 4.13401 4.13401 1 8 1C11.866 1 15 4.13401 15 8C15 11.866 11.866 15 8 15C4.13401 15 1 11.866 1 8Z"
        />
      </svg>
    {/if}
  </button>

  {#if menuOpen && user}
    <!-- Click-anywhere-else closes the menu without stealing the click. -->
    <button class="scrim" aria-label="Close menu" onclick={() => closeMenu()}></button>
    <div
      class="menu"
      role="menu"
      aria-label="Account"
      use:focusFirst
      style:left="{menuPos.left}px"
      style:bottom="{menuPos.bottom}px"
    >
      <button type="button" class="menu-item" role="menuitem" onclick={gotoAccount}>Go to account</button>
      <button type="button" class="menu-item" role="menuitem" onclick={logout}>Logout</button>
    </div>
  {/if}
  <button
    type="button"
    class="item settings"
    use:tip={'Settings'}
    aria-label="Settings"
    onclick={() => (ui.settingsOpen = true)}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        d="M12 9C10.3425 9 9.00002 10.3425 9.00002 12C9.00002 13.6575 10.3425 15 12 15C13.6575 15 15 13.6575 15 12C15 10.3425 13.6575 9 12 9ZM12 13.5C11.172 13.5 10.5 12.828 10.5 12C10.5 11.172 11.172 10.5 12 10.5C12.828 10.5 13.5 11.172 13.5 12C13.5 12.828 12.828 13.5 12 13.5ZM21.8475 14.5725L19.9185 12.942C19.8675 12.8985 19.8195 12.8505 19.776 12.7995C19.332 12.279 19.3965 11.5005 19.9185 11.058L21.8475 9.4275C22.0395 9.2655 22.113 9.0045 22.0365 8.766C21.579 7.3545 20.823 6.06 19.8285 4.962C19.7085 4.83 19.5405 4.758 19.368 4.758C19.2975 4.758 19.227 4.77 19.1595 4.794L16.779 5.6415C16.716 5.664 16.65 5.682 16.584 5.694C16.509 5.7075 16.434 5.715 16.3605 5.715C15.7725 5.715 15.2505 5.298 15.141 4.701L14.6865 2.223C14.6415 1.977 14.451 1.782 14.205 1.7295C13.485 1.5765 12.7485 1.5 12.0015 1.5C11.2545 1.5 10.5165 1.578 9.79652 1.7295C9.55052 1.782 9.36002 1.977 9.31502 2.223L8.86202 4.701C8.85002 4.767 8.83202 4.8315 8.80952 4.8945C8.62802 5.4 8.15102 5.715 7.64102 5.715C7.50302 5.715 7.36202 5.691 7.22402 5.643L4.84352 4.7955C4.77602 4.7715 4.70402 4.7595 4.63502 4.7595C4.46252 4.7595 4.29452 4.8315 4.17452 4.9635C3.17852 6.0615 2.42402 7.356 1.96502 8.7675C1.88702 9.006 1.96202 9.267 2.15402 9.429L4.08302 11.0595C4.13402 11.103 4.18202 11.151 4.22552 11.202C4.66952 11.7225 4.60502 12.501 4.08302 12.9435L2.15402 14.574C1.96202 14.736 1.88852 14.997 1.96502 15.2355C2.42252 16.647 3.17852 17.9415 4.17452 19.0395C4.29452 19.1715 4.46252 19.2435 4.63502 19.2435C4.70552 19.2435 4.77602 19.2315 4.84352 19.2075L7.22402 18.36C7.28702 18.3375 7.35302 18.3195 7.41902 18.3075C7.49402 18.294 7.56902 18.288 7.64252 18.288C8.23052 18.288 8.75252 18.705 8.86202 19.302L9.31502 21.78C9.36002 22.026 9.55052 22.221 9.79652 22.2735C10.5165 22.4265 11.2545 22.503 12.0015 22.503C12.7485 22.503 13.4865 22.425 14.205 22.2735C14.451 22.221 14.6415 22.026 14.6865 21.78L15.141 19.302C15.153 19.236 15.171 19.1715 15.1935 19.1085C15.375 18.603 15.852 18.288 16.362 18.288C16.5 18.288 16.641 18.312 16.779 18.36L19.158 19.2075C19.227 19.2315 19.2975 19.2435 19.3665 19.2435C19.539 19.2435 19.707 19.1715 19.827 19.0395C20.823 17.9415 21.5775 16.647 22.035 15.2355C22.113 14.997 22.038 14.736 21.846 14.574L21.8475 14.5725ZM19.092 17.589L17.2815 16.944C16.9845 16.839 16.6755 16.785 16.362 16.785C15.2085 16.785 14.1705 17.514 13.782 18.5985C13.731 18.738 13.6935 18.882 13.6665 19.029L13.3215 20.9055C12.8865 20.9685 12.444 21 12.0015 21C11.559 21 11.1165 20.9685 10.68 20.904L10.3365 19.0275C10.098 17.727 8.96552 16.7835 7.64252 16.7835C7.48052 16.7835 7.31552 16.7985 7.14902 16.8285C7.00352 16.8555 6.86102 16.893 6.72002 16.9425L4.90952 17.5875C4.35752 16.896 3.91652 16.1385 3.59102 15.321L5.05202 14.0865C5.61152 13.614 5.95202 12.951 6.01202 12.222C6.07202 11.493 5.84252 10.785 5.36702 10.227C5.27102 10.1145 5.16452 10.008 5.05202 9.912L3.59102 8.6775C3.91652 7.86 4.35752 7.101 4.90952 6.411L6.72002 7.056C7.01702 7.161 7.32602 7.215 7.64102 7.215C8.79452 7.215 9.83252 6.486 10.221 5.4015C10.272 5.2605 10.3095 5.1165 10.3365 4.971L10.68 3.0945C11.1165 3.0315 11.559 2.9985 12.0015 2.9985C12.444 2.9985 12.8865 3.03 13.3215 3.093L13.665 4.9695C13.9035 6.27 15.036 7.2135 16.359 7.2135C16.521 7.2135 16.686 7.1985 16.851 7.1685C16.9965 7.1415 17.1405 7.104 17.2815 7.0545L19.092 6.4095C19.644 7.0995 20.085 7.8585 20.4105 8.676L18.951 9.9105C18.3915 10.383 18.0495 11.046 17.991 11.775C17.931 12.504 18.1605 13.2135 18.636 13.77C18.7335 13.884 18.8385 13.989 18.9525 14.085L20.4135 15.3195C20.088 16.137 19.647 16.896 19.095 17.586L19.092 17.589Z"
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
    /* Rounded by the wrapper that also holds the sidebar. */
    border-radius: 0;
    box-sizing: border-box;
    background: var(--surface-1);
  }
  /* Follows the panel borders; only when the sidebar is there to divide. */
  .rail.separated {
    border-right: 1px solid var(--border);
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
    background: var(--accent);
    content: '';
  }
  .item.active:not(.open)::before {
    background: var(--muted);
    opacity: 0.4;
  }
  .item svg {
    width: 22px;
    height: 22px;
    /* Codicons are filled paths: no stroke, or they grow a fat outline. */
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
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    border: 0;
    background: transparent;
  }
  .menu {
    position: fixed;
    z-index: 51;
    display: flex;
    flex-direction: column;
    min-width: 190px;
    padding: var(--sp-1);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-3);
    box-shadow: var(--shadow-2);
    font-size: var(--fs-sm);
  }
  .menu-item {
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-sm);
    text-align: left;
  }
  .menu-item:hover,
  .menu-item:focus-visible {
    background: var(--accent);
    color: var(--on-accent);
    outline: none;
  }
</style>
