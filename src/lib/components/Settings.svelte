<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ui, THEMES, KEYBINDINGS } from '../stores/ui.svelte'
  import { pushToast } from '../stores/toasts.svelte'

  interface UserInfo {
    login: string
    avatarUrl: string
  }

  type Tab = 'appearance' | 'sync' | 'editor' | 'extensions' | 'account'

  const TABS: Array<{ id: Tab; label: string }> = [
    { id: 'appearance', label: 'Appearance' },
    { id: 'sync', label: 'Sync' },
    { id: 'editor', label: 'Editor' },
    { id: 'extensions', label: 'Extensions' },
    { id: 'account', label: 'Account' },
  ]

  const LANGUAGES = [
    'plaintext',
    'markdown',
    'json',
    'yaml',
    'toml',
    'shell',
    'python',
    'rust',
    'javascript',
    'typescript',
    'html',
    'css',
    'cpp',
    'c',
    'go',
    'java',
  ]

  let tab = $state<Tab>('appearance')
  let user = $state<UserInfo | null>(null)
  let accountError = $state('')
  let newExt = $state('')
  let newLang = $state('plaintext')
  let extMessage = $state('')

  const extensions = $derived(
    Object.entries(ui.extensions).sort(([a], [b]) => a.localeCompare(b)),
  )

  onMount(async () => {
    try {
      user = await invoke<UserInfo>('github_get_user')
    } catch {
      accountError = 'not connected'
    }
  })

  function clamp(value: number, min: number, max: number, apply: (n: number) => void) {
    if (!Number.isFinite(value)) return
    apply(Math.min(max, Math.max(min, Math.round(value))))
  }

  function addExtension() {
    const ext = newExt.trim().replace(/^\./, '').toLowerCase()
    if (!ext) {
      extMessage = 'Enter an extension, e.g. rs'
      return
    }
    ui.extensions = { ...ui.extensions, [ext]: newLang }
    extMessage = `Added .${ext} (${newLang})`
    newExt = ''
  }

  function removeExtension(ext: string) {
    const next = { ...ui.extensions }
    delete next[ext]
    ui.extensions = next
    extMessage = ''
  }

  async function logout() {
    try {
      await invoke('auth_clear')
    } catch (e) {
      pushToast(`logout: ${e}`)
      return
    }
    ui.onboardingComplete = false
    ui.settingsOpen = false
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') ui.settingsOpen = false
  }}
/>

<div class="overlay" role="presentation">
  <button class="backdrop" aria-label="Close settings" onclick={() => (ui.settingsOpen = false)}
  ></button>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Settings">
    <header>
      <h2>Settings</h2>
      <button class="close" aria-label="Close settings" onclick={() => (ui.settingsOpen = false)}>
        ×
      </button>
    </header>

    <div class="tabs" role="tablist">
      {#each TABS as item (item.id)}
        <button
          role="tab"
          class="tab"
          class:active={tab === item.id}
          aria-selected={tab === item.id}
          onclick={() => (tab = item.id)}
        >
          {item.label}
        </button>
      {/each}
    </div>

    <div class="body">
      {#if tab === 'appearance'}
        <div class="field">
          <span>Theme</span>
          <select bind:value={ui.editorTheme}>
            {#each THEMES as theme (theme)}
              <option value={theme}>{theme}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <span>Font size</span>
          <div class="inline">
            <input
              type="number"
              min="8"
              max="32"
              value={ui.fontSize}
              onchange={(e) =>
                clamp(e.currentTarget.valueAsNumber, 8, 32, (n) => (ui.fontSize = n))}
            />
            <span class="hint">8–32 px</span>
          </div>
        </div>
        <div class="field">
          <span>Font family</span>
          <input
            class="wide"
            value={ui.fontFamily}
            onchange={(e) => (ui.fontFamily = e.currentTarget.value)}
          />
        </div>
      {:else if tab === 'sync'}
        <div class="field">
          <span>Auto-sync</span>
          <label class="inline">
            <input type="checkbox" bind:checked={ui.autoSync} />
            Push automatically after edits
          </label>
        </div>
        <div class="field">
          <span>Save debounce</span>
          <div class="inline">
            <input
              type="range"
              min="500"
              max="10000"
              step="500"
              value={ui.saveDebounceMs}
              oninput={(e) => (ui.saveDebounceMs = e.currentTarget.valueAsNumber)}
            />
            <span class="value">{ui.saveDebounceMs / 1000}s</span>
          </div>
        </div>
        <div class="field">
          <span>Push after idle</span>
          <div class="inline">
            <input
              type="number"
              min="5"
              max="300"
              value={Math.round(ui.pushIdleMs / 1000)}
              onchange={(e) =>
                clamp(e.currentTarget.valueAsNumber, 5, 300, (n) => (ui.pushIdleMs = n * 1000))}
            />
            <span class="hint">seconds, 5–300</span>
          </div>
        </div>
      {:else if tab === 'editor'}
        <div class="field">
          <span>Keybindings</span>
          <select bind:value={ui.keybindings}>
            {#each KEYBINDINGS as mode (mode)}
              <option value={mode}>{mode}</option>
            {/each}
          </select>
        </div>
        <p class="note">
          {ui.keybindings === 'vscode'
            ? 'Standard Monaco shortcuts.'
            : `${ui.keybindings} keybindings are applied to the editor; Ctrl+P still opens the palette.`}
        </p>
      {:else if tab === 'extensions'}
        <ul class="extensions">
          {#each extensions as [ext, lang] (ext)}
            <li>
              <span class="ext">.{ext}</span>
              <span class="lang">{lang}</span>
              <button aria-label="Remove .{ext}" onclick={() => removeExtension(ext)}>×</button>
            </li>
          {/each}
        </ul>
        <div class="add">
          <input placeholder="ext" bind:value={newExt} onkeydown={(e) => {
            if (e.key === 'Enter') addExtension()
          }} />
          <select bind:value={newLang}>
            {#each LANGUAGES as lang (lang)}
              <option value={lang}>{lang}</option>
            {/each}
          </select>
          <button class="primary" onclick={addExtension}>Add</button>
        </div>
        {#if extMessage}
          <p class="note">{extMessage}</p>
        {/if}
      {:else}
        {#if user}
          <div class="field">
            <span>Signed in</span>
            <div class="account">
              <img src={user.avatarUrl} alt="" />
              <span>{user.login}</span>
            </div>
          </div>
          <div class="field">
            <span></span>
            <button class="danger" onclick={logout}>Logout</button>
          </div>
        {:else}
          <p class="note">{accountError || 'loading…'}</p>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
  }
  .modal {
    position: relative;
    display: flex;
    flex-direction: column;
    width: min(880px, 92vw);
    height: min(620px, 88vh);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-panel);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 8px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .close {
    padding: 0 6px;
    color: var(--muted);
    font-size: 18px;
  }
  .close:hover {
    color: var(--text);
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    padding: 7px 12px;
    border-bottom: 2px solid transparent;
    color: var(--muted);
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 18px 22px;
  }
  .field {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    padding: 9px 0;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .field > span:first-child {
    color: var(--muted);
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .inline input[type='number'] {
    width: 80px;
  }
  .inline input[type='range'] {
    flex: 1;
    max-width: 320px;
  }
  .wide {
    width: 100%;
    max-width: 460px;
  }
  .hint,
  .note {
    color: var(--muted);
    font-size: 11px;
  }
  .note {
    margin: 12px 0 0;
  }
  .value {
    min-width: 44px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .extensions {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0 0 14px;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }
  .extensions li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px;
    border-radius: 3px;
    background: var(--bg-raised);
  }
  .extensions .ext {
    width: 120px;
  }
  .extensions .lang {
    flex: 1;
    min-width: 0;
    color: var(--muted);
  }
  .extensions button {
    color: var(--muted);
  }
  .extensions button:hover {
    color: var(--err);
  }
  .add {
    display: flex;
    gap: 8px;
    max-width: 560px;
  }
  .add input {
    flex: 1;
    min-width: 0;
  }
  .add select {
    flex: 0 0 160px;
    min-width: 0;
  }
  .add button {
    flex: 0 0 auto;
    padding: 4px 14px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-raised);
  }
  .add button:hover {
    border-color: var(--accent);
  }
  .account {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .account img {
    width: 26px;
    height: 26px;
    border-radius: 50%;
  }
  .danger {
    padding: 4px 12px;
    border: 1px solid var(--err);
    border-radius: 3px;
    color: var(--err);
    justify-self: start;
  }
</style>
