<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ui, THEMES, KEYBINDINGS, type Keybinding } from '../stores/ui.svelte'

  interface UserInfo {
    login: string
    avatarUrl: string
  }

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

  let user = $state<UserInfo | null>(null)
  let accountError = $state('')
  let newExt = $state('')
  let newLang = $state('plaintext')

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
    if (!ext) return
    ui.extensions = { ...ui.extensions, [ext]: newLang }
    newExt = ''
  }

  function removeExtension(ext: string) {
    const next = { ...ui.extensions }
    delete next[ext]
    ui.extensions = next
  }

  async function logout() {
    await invoke('auth_clear').catch(() => {})
    ui.onboardingComplete = false
    ui.settingsOpen = false
  }
</script>

<aside class="panel" aria-label="Settings">
  <header>
    <span>Settings</span>
    <button class="close" aria-label="Close settings" onclick={() => (ui.settingsOpen = false)}>
      ×
    </button>
  </header>

  <section>
    <h3>Appearance</h3>
    <label>
      Theme
      <select bind:value={ui.editorTheme}>
        {#each THEMES as theme (theme)}
          <option value={theme}>{theme}</option>
        {/each}
      </select>
    </label>
    <label>
      Font size
      <input
        type="number"
        min="8"
        max="32"
        value={ui.fontSize}
        onchange={(e) => clamp(e.currentTarget.valueAsNumber, 8, 32, (n) => (ui.fontSize = n))}
      />
    </label>
    <label>
      Font family
      <input value={ui.fontFamily} onchange={(e) => (ui.fontFamily = e.currentTarget.value)} />
    </label>
  </section>

  <section>
    <h3>Sync</h3>
    <label class="row">
      <input type="checkbox" bind:checked={ui.autoSync} />
      Auto-sync
    </label>
    <label>
      Save debounce: {ui.saveDebounceMs / 1000}s
      <input
        type="range"
        min="500"
        max="10000"
        step="500"
        value={ui.saveDebounceMs}
        oninput={(e) => (ui.saveDebounceMs = e.currentTarget.valueAsNumber)}
      />
    </label>
    <label>
      Push after idle (seconds)
      <input
        type="number"
        min="5"
        max="300"
        value={Math.round(ui.pushIdleMs / 1000)}
        onchange={(e) =>
          clamp(e.currentTarget.valueAsNumber, 5, 300, (n) => (ui.pushIdleMs = n * 1000))}
      />
    </label>
  </section>

  <section>
    <h3>Editor</h3>
    <label>
      Keybindings
      <select bind:value={ui.keybindings}>
        {#each KEYBINDINGS as mode (mode)}
          <option value={mode}>{mode}</option>
        {/each}
      </select>
    </label>
    {#if ui.keybindings === 'vscode'}
      <p class="hint">Standard Monaco shortcuts.</p>
    {/if}
    <span class="label">Supported extensions</span>
    <ul class="extensions">
      {#each extensions as [ext, lang] (ext)}
        <li>
          <span>.{ext}</span>
          <span class="lang">{lang}</span>
          <button aria-label="Remove .{ext}" onclick={() => removeExtension(ext)}>×</button>
        </li>
      {/each}
    </ul>
    <form
      class="add"
      onsubmit={(e) => {
        e.preventDefault()
        addExtension()
      }}
    >
      <input placeholder="ext" bind:value={newExt} />
      <select bind:value={newLang}>
        {#each LANGUAGES as lang (lang)}
          <option value={lang}>{lang}</option>
        {/each}
      </select>
      <button type="submit">Add</button>
    </form>
  </section>

  <section>
    <h3>Account</h3>
    {#if user}
      <div class="account">
        <img src={user.avatarUrl} alt="" />
        <span>{user.login}</span>
      </div>
      <button class="logout" onclick={logout}>Logout</button>
    {:else}
      <p class="hint">{accountError || 'loading…'}</p>
    {/if}
  </section>
</aside>

<style>
  .panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 40;
    display: flex;
    flex-direction: column;
    width: 340px;
    overflow-y: auto;
    padding: 12px;
    gap: 18px;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
    box-shadow: -4px 0 16px rgba(0, 0, 0, 0.35);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-weight: 600;
  }
  .close {
    padding: 0 6px;
    color: var(--muted);
    font-size: 16px;
  }
  .close:hover {
    color: var(--text);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h3 {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
  }
  label.row {
    flex-direction: row;
    align-items: center;
    gap: 6px;
  }
  input,
  select {
    width: 100%;
  }
  label.row input {
    width: auto;
  }
  input[type='range'] {
    padding: 0;
  }
  .label {
    font-size: 12px;
  }
  .extensions {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }
  .extensions li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 4px;
    border-radius: 3px;
    background: var(--bg-raised);
  }
  .extensions .lang {
    flex: 1;
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
    gap: 4px;
  }
  .add input {
    flex: 1;
  }
  .add select {
    flex: 1;
  }
  .add button,
  .logout {
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-raised);
  }
  .account {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .account img {
    width: 24px;
    height: 24px;
    border-radius: 50%;
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
  }
</style>
