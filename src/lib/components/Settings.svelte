<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ui, THEMES, KEYBINDINGS } from '../stores/ui.svelte'
  import { pushToast } from '../stores/toasts.svelte'
  import { contrast } from '../utils/color'
  import { DEFAULT_FONT, surfaceForTheme } from '../utils/themes'
  import { open } from '@tauri-apps/plugin-dialog'
  import { errorMessage, applyGitStatus, type GitStatus } from '../stores/git.svelte'
  import { retarget, tabs, closeTab } from '../stores/tabs.svelte'
  import { flushPendingSave } from '../editor/bridge'

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

  const ACCENT_PRESETS = [
    '#0078d4', // blue, the default accent
    '#8b5cf6', // violet
    '#10b981', // green
    '#f38518', // orange
    '#e5484d', // red
    '#ec4899', // pink
    '#06b6d4', // cyan
    '#d7ba7d', // amber
  ]

  let tab = $state<Tab>('appearance')
  // The swatches sit on the panel surface, which differs per theme: re-read it when the theme changes.
  const accentWarning = $derived.by(() => {
    if (!ui.accent) return false
    void ui.editorTheme
    return (contrast(ui.accent, surfaceForTheme()) ?? 99) < 3
  })
  let user = $state<UserInfo | null>(null)
  let accountError = $state('')
  let newExt = $state('')
  let newLang = $state('plaintext')
  let extMessage = $state('')
  let moving = $state(false)
  let importOpen = $state(false)
  let importUrl = $state('')
  let importFolder = $state('')
  let importing = $state(false)

  const extensions = $derived(
    Object.entries(ui.extensions).sort(([a], [b]) => a.localeCompare(b)),
  )


  /// Installed families, filled in by fc-list; the select falls back to the
  /// saved value when the list is empty or the font was uninstalled.
  let fonts = $state<string[]>([])
  const fontOptions = $derived(
    fonts.includes(ui.fontFamily) || ui.fontFamily === DEFAULT_FONT
      ? fonts
      : [ui.fontFamily, ...fonts],
  )

  onMount(async () => {
    try {
      user = await invoke<UserInfo>('github_get_user')
    } catch {
      accountError = 'not connected'
    }
    fonts = (await invoke<string[]>('fs_list_fonts').catch(() => [])) ?? []
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

  async function moveRepo() {
    const picked = await open({ directory: true, multiple: false, title: 'Repository folder' })
    if (typeof picked !== 'string' || !picked) return
    moving = true
    try {
      const moved = await invoke<{ from: string; to: string }>('git_move_repo', { parent: picked })
      // Open tabs follow the clone, else their saves would land in the old folder.
      retarget(moved.from, moved.to)
      ui.repoPath = moved.to
      ui.treeVersion += 1
      pushToast('Repository moved')
    } catch (e) {
      pushToast(`move repository: ${errorMessage(e)}`)
    } finally {
      moving = false
    }
  }

  async function chooseImportFolder() {
    const picked = await open({ directory: true, multiple: false, title: 'Clone repository into' })
    if (typeof picked === 'string' && picked) importFolder = picked
  }

  async function importRepo() {
    const url = importUrl.trim()
    if (!url || !importFolder || importing) return
    // Unsaved edits belong to the repo being replaced, so they must land first.
    if (!(await flushPendingSave())) return
    importing = true
    try {
      const result = await invoke<{ from: string; to: string }>('git_import_repo', {
        url,
        parent: importFolder,
      })
      // Open tabs point into the old clone: they close instead of retargeting.
      for (const tab of [...tabs.tabs]) closeTab(tab.id)
      ui.repoPath = result.to
      ui.expanded = []
      ui.treeVersion += 1
      void invoke<GitStatus>('git_status').then(applyGitStatus).catch(() => {})
      importOpen = false
      importUrl = ''
      importFolder = ''
      pushToast('Repository imported')
    } catch (e) {
      pushToast(`import repository: ${errorMessage(e)}`)
    } finally {
      importing = false
    }
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
          <span>Accent color</span>
          <div class="accent-picker">
            {#each ACCENT_PRESETS as color (color)}
              <button
                type="button"
                class="swatch"
                class:active={ui.accent === color}
                style="--swatch-color: {color}"
                aria-label="Accent {color}"
                aria-pressed={ui.accent === color}
                onclick={() => (ui.accent = color)}
              ></button>
            {/each}
            <input
              type="text"
              class="hex-input"
              value={ui.accent}
              placeholder="#0078d4"
              aria-label="Accent hex value"
              onchange={(e) => {
                const v = e.currentTarget.value.trim()
                if (/^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(v)) ui.accent = v
              }}
            />
            {#if ui.accent}
              <button type="button" class="reset-accent" onclick={() => (ui.accent = '')}>
                Reset
              </button>
            {/if}
            {#if accentWarning}
              <span class="accent-warn">Low contrast with panel background</span>
            {/if}
          </div>
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
          <select class="wide" bind:value={ui.fontFamily}>
            <option value={DEFAULT_FONT}>Default (monospace)</option>
            {#each fontOptions as font (font)}
              <option value={font}>{font}</option>
            {/each}
          </select>
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
          <span>Repository folder</span>
          <div class="inline">
            <span class="hint path">{ui.repoPath || 'Default app folder'}</span>
            <button type="button" class="change" disabled={moving} onclick={moveRepo}>
              {moving ? 'Moving…' : 'Change…'}
            </button>
          </div>
        </div>
        <div class="field">
          <span>Import repository</span>
          <div class="inline">
            <span class="hint">Clone another repo and switch to it</span>
            <button
              type="button"
              class="change"
              aria-expanded={importOpen}
              onclick={() => (importOpen = !importOpen)}
            >
              {importOpen ? 'Cancel' : 'Import…'}
            </button>
          </div>
        </div>
        {#if importOpen}
          <div class="import">
            <input
              type="text"
              class="wide"
              placeholder="https://github.com/user/repo.git"
              bind:value={importUrl}
              onkeydown={(e) => {
                if (e.key === 'Enter') void importRepo()
              }}
            />
            <div class="inline">
              <span class="hint path">
                {importFolder || 'Choose the folder that will hold the clone'}
              </span>
              <button type="button" class="change" disabled={importing} onclick={chooseImportFolder}>
                Choose…
              </button>
            </div>
            <div class="inline">
              <button
                type="button"
                class="change go"
                disabled={importing || !importUrl.trim() || !importFolder}
                onclick={importRepo}
              >
                {importing ? 'Cloning…' : 'Clone as active repository'}
              </button>
              <span class="hint">Open tabs close: they belong to the current repository.</span>
            </div>
          </div>
        {/if}
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
    background: var(--scrim);
  }
  .modal {
    position: relative;
    display: flex;
    flex-direction: column;
    width: min(880px, 92vw);
    height: min(620px, 88vh);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: var(--surface-2);
    box-shadow:
      0 20px 60px rgba(0, 0, 0, 0.6),
      0 8px 32px rgba(0, 0, 0, 0.4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-3) var(--sp-4);
    border-bottom: 1px solid var(--border);
    border-radius: var(--r-lg) var(--r-lg) 0 0;
    background: var(--surface-1);
  }
  h2 {
    margin: 0;
    color: var(--text);
    font-size: var(--fs-lg);
    font-weight: 600;
  }
  .close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--muted);
    font-size: 18px;
    cursor: pointer;
  }
  .close:hover {
    background: var(--surface-0);
    color: var(--text);
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: var(--sp-1) var(--sp-2) 0;
    border-bottom: 1px solid var(--border);
    background: var(--surface-1);
  }
  .tab {
    position: relative;
    padding: var(--sp-2) var(--sp-3);
    border: none;
    border-top: 2px solid transparent;
    border-radius: var(--r-sm) var(--r-sm) 0 0;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .tab:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .tab.active {
    background: var(--surface-2);
    color: var(--text);
  }
  /* Inset past the corners: a border-top would follow the radius and bow. */
  .tab.active::after {
    position: absolute;
    top: 0;
    right: var(--r-sm);
    left: var(--r-sm);
    height: 2px;
    background: var(--accent);
    content: '';
  }
  .body {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--sp-4);
    overflow-y: auto;
    padding: var(--sp-4);
    background: var(--surface-2);
  }
  .field {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    font-size: 13px;
  }
  .field > span:first-child {
    min-width: 120px;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  /* Dark only: the dark tokens sit too close in value to show any depth, so the
     modal uses explicit surfaces here. Light and HC keep the token colours. */
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .modal {
    background: #2c2c2c;
    box-shadow:
      0 24px 64px rgba(0, 0, 0, 0.7),
      0 8px 24px rgba(0, 0, 0, 0.5);
  }
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .modal header,
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .tabs {
    background: #1e1e1e;
    border-bottom-color: #3a3a3a;
  }
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .tab:hover,
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .tab.active,
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .body {
    background: #2c2c2c;
  }
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .field > span:first-child {
    /* --muted is 4.27:1 on #2c2c2c; this is 4.96:1. */
    color: #9a9a9a;
  }
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .close:hover {
    background: #3a3a3a;
  }
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .modal select,
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .modal input[type='text'],
  :global(:root:not([data-theme='vs']):not([data-theme='hc-black'])) .modal input[type='number'] {
    border-color: #3a3a3a;
    background: #1a1a1a;
  }
  .modal select,
  .modal input[type='text'],
  .modal input[type='number'] {
    padding: var(--sp-1) var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-0);
    color: var(--text);
  }
  .modal select:focus,
  .modal input[type='text']:focus,
  .modal input[type='number']:focus {
    border-color: var(--accent-fg);
    outline: none;
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
  .path {
    overflow-wrap: anywhere;
  }
  .field button.change {
    padding: var(--sp-1) var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-0);
    color: var(--text);
  }
  .field button.change:hover:not(:disabled) {
    border-color: var(--accent-fg);
  }
  .import {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
  }
  .import .go {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .import .go:hover:not(:disabled) {
    border-color: var(--accent-fg);
    filter: brightness(1.1);
  }
  .accent-picker {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
  }
  .swatch {
    width: 20px;
    height: 20px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: var(--r-md);
    background: var(--swatch-color);
    cursor: pointer;
  }
  .swatch.active,
  .swatch:focus-visible {
    border-color: var(--text);
    outline: none;
  }
  .hex-input {
    width: 80px;
    font-family: monospace;
    font-size: var(--fs-sm);
  }
  .reset-accent {
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font-size: var(--fs-xs);
    text-decoration: underline;
    cursor: pointer;
  }
  .reset-accent:hover {
    color: var(--text);
  }
  .accent-warn {
    flex-basis: 100%;
    color: var(--warn);
    font-size: var(--fs-xs);
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
    border-radius: var(--r-sm);
    background: var(--surface-0);
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
    border-radius: var(--r-sm);
    background: var(--surface-0);
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
    border-radius: var(--r-sm);
    color: var(--err);
    justify-self: start;
  }
</style>
