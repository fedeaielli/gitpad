<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { getVersion } from '@tauri-apps/api/app'
  import { ui, THEMES, KEYBINDINGS } from '../stores/ui.svelte'
  import { pushToast } from '../stores/toasts.svelte'
  import { openExternal } from '../utils/open'
  import { checkUpdate, type UpdateInfo } from '../utils/update'
  import { contrast } from '../utils/color'
  import { DEFAULT_FONT, surfaceForTheme } from '../utils/themes'
  import { defaultDesc } from '../utils/file-sort'
  import { focusTrap } from '../utils/focus'
  import { tip } from '../utils/tooltip'
  import { open } from '@tauri-apps/plugin-dialog'
  import { errorMessage, applyGitStatus, gitStatus, type GitStatus } from '../stores/git.svelte'
  import { auth, forgetUser, loadUser, signInWithPat } from '../stores/auth.svelte'
  import Check from './Check.svelte'
  import GithubLogin from './GithubLogin.svelte'
  import LogsPane from './LogsPane.svelte'
  import { tabs, closeTab } from '../stores/tabs.svelte'
  import { flushPendingSave } from '../editor/bridge'
  import { confirmDialog } from '../stores/confirm.svelte'

  type Tab = 'appearance' | 'sync' | 'editor' | 'extensions' | 'logs' | 'account' | 'about'

  /// Codicon paths, filled like the activity bar's; `box` only for the icons
  /// drawn on a larger grid.
  const TABS: Array<{ id: Tab; label: string; icon: string; box?: string }> = [
    {
      id: 'appearance',
      label: 'Appearance',
      icon: 'M8.00101 1C4.13401 1 1.00101 3.8 1.00101 7.667C1.00101 8.956 2.04501 10 3.33401 10C4.75101 10 4.72101 9 6.00001 9C6.64401 9 7.00001 9.606 7.00001 10.25V11.5C7.00001 13.433 8.56701 15 10.5 15C13.653 15 14.999 11.215 14.999 8C14.999 4.134 11.866 1 8.00001 1H8.00101ZM10.5 14C9.12201 14 8.00001 12.878 8.00001 11.5V10.25C8.00001 8.967 7.14001 8 6.00001 8C5.04001 8 4.49801 8.412 4.13901 8.685C3.85401 8.902 3.72401 9 3.33401 9C2.59901 9 2.00101 8.402 2.00101 7.667C2.00101 4.436 4.58001 2 8.00101 2C11.309 2 14 4.692 14 8C14 10.412 13.068 14 10.501 14H10.5ZM12 11C12 11.552 11.552 12 11 12C10.448 12 10 11.552 10 11C10 10.448 10.448 10 11 10C11.552 10 12 10.448 12 11ZM13 8C13 8.552 12.552 9 12 9C11.448 9 11 8.552 11 8C11 7.448 11.448 7 12 7C12.552 7 13 7.448 13 8ZM6.00001 5C6.00001 5.552 5.55201 6 5.00001 6C4.44801 6 4.00001 5.552 4.00001 5C4.00001 4.448 4.44801 4 5.00001 4C5.55201 4 6.00001 4.448 6.00001 5ZM10 5C10 4.448 10.448 4 11 4C11.552 4 12 4.448 12 5C12 5.552 11.552 6 11 6C10.448 6 10 5.552 10 5ZM9.00001 4C9.00001 4.552 8.55201 5 8.00001 5C7.44801 5 7.00001 4.552 7.00001 4C7.00001 3.448 7.44801 3 8.00001 3C8.55201 3 9.00001 3.448 9.00001 4Z',
    },
    {
      id: 'sync',
      label: 'Sync',
      icon: 'M14 3.5V6.5C14 6.78 13.78 7 13.5 7H10.5C10.22 7 9.99999 6.78 9.99999 6.5C9.99999 6.22 10.22 6 10.5 6H12.58C11.78 4.17 10.01 3 7.99999 3C5.77999 3 3.79999 4.5 3.18999 6.64C3.12999 6.86 2.92999 7 2.70999 7C2.65999 7 2.61999 7 2.56999 6.98C2.29999 6.9 2.14999 6.63 2.22999 6.36C2.95999 3.79 5.32999 2 7.99999 2C10.05 2 11.91 3.02 13 4.69V3.5C13 3.22 13.22 3 13.5 3C13.78 3 14 3.22 14 3.5ZM13.42 9.02C13.16 8.95 12.88 9.1 12.8 9.37C12.19 11.51 10.22 13.01 7.98999 13.01C5.97999 13.01 4.20999 11.84 3.40999 10.01H5.48999C5.76999 10.01 5.98999 9.79 5.98999 9.51C5.98999 9.23 5.76999 9.01 5.48999 9.01H2.48999C2.20999 9.01 1.98999 9.23 1.98999 9.51V12.51C1.98999 12.79 2.20999 13.01 2.48999 13.01C2.76999 13.01 2.98999 12.79 2.98999 12.51V11.32C4.07999 12.98 5.93999 14.01 7.98999 14.01C10.66 14.01 13.03 12.22 13.76 9.65C13.84 9.38 13.68 9.11 13.41 9.03L13.42 9.02Z',
    },
    {
      id: 'editor',
      label: 'Editor',
      icon: 'M9.80307 3.0431C10.0554 3.15525 10.1691 3.45073 10.0569 3.70307L6.05691 12.7031C5.94475 12.9554 5.64927 13.0691 5.39693 12.9569C5.14459 12.8448 5.03094 12.5493 5.14309 12.2969L9.14309 3.29693C9.25525 3.04459 9.55073 2.93094 9.80307 3.0431ZM4.33218 5.3763C4.53857 5.55976 4.55716 5.87579 4.3737 6.08218L2.66898 8L4.3737 9.91782C4.55716 10.1242 4.53857 10.4402 4.33218 10.6237C4.12579 10.8072 3.80975 10.7886 3.6263 10.5822L1.6263 8.33218C1.4579 8.14274 1.4579 7.85726 1.6263 7.66782L3.6263 5.41782C3.80975 5.21143 4.12579 5.19284 4.33218 5.3763ZM11.6678 5.3763C11.8742 5.19284 12.1902 5.21143 12.3737 5.41782L14.3737 7.66782C14.5421 7.85726 14.5421 8.14274 14.3737 8.33218L12.3737 10.5822C12.1902 10.7886 11.8742 10.8072 11.6678 10.6237C11.4614 10.4402 11.4428 10.1242 11.6263 9.91782L13.331 8L11.6263 6.08218C11.4428 5.87579 11.4614 5.55976 11.6678 5.3763Z',
    },
    {
      id: 'extensions',
      label: 'Extensions',
      icon: 'M15 4.95703C15 4.58711 14.8563 4.24054 14.5949 3.97992L12.0096 1.39234C11.4879 0.86922 10.5788 0.86922 10.0571 1.39234L8 3.45119V3.32321C8 2.55068 7.37187 1.922 6.6 1.922H2.4C1.62813 1.922 1 2.55068 1 3.32321V13.5988C1 14.3713 1.62813 15 2.4 15H12.6667C13.4385 15 14.0667 14.3713 14.0667 13.5988V9.39514C14.0667 8.62261 13.4385 7.99393 12.6667 7.99393H12.5379L14.5949 5.93508C14.8553 5.67445 15 5.32602 15 4.95703ZM2.4 2.85521H6.6C6.85667 2.85521 7.06667 3.06446 7.06667 3.32228V7.99299H1.93333V3.32228C1.93333 3.06446 2.14333 2.85521 2.4 2.85521ZM1.93333 13.5979V8.92714H7.06667V14.0649H2.4C2.14333 14.0649 1.93333 13.8547 1.93333 13.5979ZM13.1333 9.39421V13.5979C13.1333 13.8547 12.9233 14.0649 12.6667 14.0649H8V8.92714H12.6667C12.9233 8.92714 13.1333 9.13638 13.1333 9.39421ZM8 7.99299V6.46287L9.5288 7.99299H8ZM13.9351 5.2737L11.3488 7.86221C11.1789 8.03223 10.8859 8.03223 10.716 7.86221L8.12973 5.2737C8.0448 5.18963 7.99813 5.07753 7.99813 4.95796C7.99813 4.83839 8.0448 4.7263 8.12973 4.64129L10.716 2.05278C10.8009 1.96777 10.9129 1.92106 11.0324 1.92106C11.1519 1.92106 11.2639 1.96777 11.3488 2.05278L13.9351 4.64129C14.02 4.72536 14.0667 4.83746 14.0667 4.95703C14.0667 5.0766 14.02 5.1887 13.9351 5.2737Z',
    },
    {
      id: 'logs',
      label: 'Logs',
      box: '0 0 24 25',
      icon: 'M8.25 8.06775C7.836 8.06775 7.5 8.40375 7.5 8.81775C7.5 9.23175 7.836 9.56775 8.25 9.56775H15.75C16.164 9.56775 16.5 9.23175 16.5 8.81775C16.5 8.40375 16.164 8.06775 15.75 8.06775H8.25ZM7.5 13.3177C7.5 12.9037 7.836 12.5677 8.25 12.5677H15.75C16.164 12.5677 16.5 12.9037 16.5 13.3177C16.5 13.7317 16.164 14.0677 15.75 14.0677H8.25C7.836 14.0677 7.5 13.7317 7.5 13.3177ZM8.25 17.0677C7.836 17.0677 7.5 17.4037 7.5 17.8177C7.5 18.2317 7.836 18.5677 8.25 18.5677H11.25C11.664 18.5677 12 18.2317 12 17.8177C12 17.4037 11.664 17.0677 11.25 17.0677H8.25ZM6.75 0.567749C6.336 0.567749 6 0.903749 6 1.31775V2.06775H5.25C4.007 2.06775 3 3.07475 3 4.31775V22.3177C3 23.5607 4.007 24.5677 5.25 24.5677H14.25C14.449 24.5677 14.64 24.4888 14.78 24.3477L20.78 18.3477C20.921 18.2067 21 18.0167 21 17.8177V4.31775C21 3.07475 19.993 2.06775 18.75 2.06775H18V1.31775C18 0.903749 17.664 0.567749 17.25 0.567749C16.836 0.567749 16.5 0.903749 16.5 1.31775V2.06775H12.75V1.31775C12.75 0.567749 12.414 0.567749 12 0.567749C11.586 0.567749 11.25 0.903749 11.25 1.31775V2.06775H7.5V1.31775C7.5 0.903749 7.164 0.567749 6.75 0.567749ZM18.75 3.56775C19.164 3.56775 19.5 3.90375 19.5 4.31775V17.0677H15.75C14.507 17.0677 13.5 18.0747 13.5 19.3177V23.0677H5.25C4.836 23.0677 4.5 22.7317 4.5 22.3177V4.31775C4.5 3.90375 4.836 3.56775 5.25 3.56775H18.75ZM18.439 18.5677L15 22.0067V19.3177C15 18.9037 15.336 18.5677 15.75 18.5677H18.439Z',
    },
    {
      id: 'account',
      label: 'Account',
      icon: 'M6 5C6 3.89543 6.89543 3 8 3C9.10457 3 10 3.89543 10 5C10 6.10457 9.10457 7 8 7C6.89543 7 6 6.10457 6 5ZM5.49998 8L10.5 8C11.3284 8 12 8.67157 12 9.5C12 10.6161 11.541 11.5103 10.7879 12.1148C10.0466 12.7098 9.05308 13 8 13C6.94692 13 5.95342 12.7098 5.21215 12.1148C4.45897 11.5103 4 10.6161 4 9.5C4 8.67161 4.67156 8 5.49998 8ZM8 0C3.58172 0 0 3.58172 0 8C0 12.4183 3.58172 16 8 16C12.4183 16 16 12.4183 16 8C16 3.58172 12.4183 0 8 0ZM1 8C1 4.13401 4.13401 1 8 1C11.866 1 15 4.13401 15 8C15 11.866 11.866 15 8 15C4.13401 15 1 11.866 1 8Z',
    },
    {
      id: 'about',
      label: 'About',
      icon: 'M8.49902 7.49998C8.49902 7.22384 8.27517 6.99998 7.99902 6.99998C7.72288 6.99998 7.49902 7.22384 7.49902 7.49998V10.5C7.49902 10.7761 7.72288 11 7.99902 11C8.27517 11 8.49902 10.7761 8.49902 10.5V7.49998ZM8.74807 5.50001C8.74807 5.91369 8.41271 6.24905 7.99903 6.24905C7.58535 6.24905 7.25 5.91369 7.25 5.50001C7.25 5.08633 7.58535 4.75098 7.99903 4.75098C8.41271 4.75098 8.74807 5.08633 8.74807 5.50001ZM8 1C4.13401 1 1 4.13401 1 8C1 11.866 4.13401 15 8 15C11.866 15 15 11.866 15 8C15 4.13401 11.866 1 8 1ZM2 8C2 4.68629 4.68629 2 8 2C11.3137 2 14 4.68629 14 8C14 11.3137 11.3137 14 8 14C4.68629 14 2 11.3137 2 8Z',
    },
  ]

  /// Arrow keys move selection without leaving the nav: automatic activation.
  function onNavKey(e: KeyboardEvent) {
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) return
    e.preventDefault()
    const index = TABS.findIndex((item) => item.id === tab)
    const next =
      e.key === 'Home'
        ? 0
        : e.key === 'End'
          ? TABS.length - 1
          : (index + (e.key === 'ArrowDown' ? 1 : -1) + TABS.length) % TABS.length
    tab = TABS[next].id
    const buttons = (e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>(
      '[role="tab"]',
    )
    buttons[next]?.focus()
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

  /// Chrome colours each theme card previews. shortcut: a new theme needs an
  /// entry here too, add one when a theme lands.
  const THEME_PREVIEW: Record<
    string,
    { name: string; bg: string; fg: string; accent: string }
  > = {
    'gitpad-dark': { name: 'GitPad Dark', bg: '#1c1a18', fg: '#d8d3ca', accent: '#0078d4' },
    vs: { name: 'Light', bg: '#fbf9f4', fg: '#24211d', accent: '#005fb8' },
    'hc-black': { name: 'High contrast', bg: '#000000', fg: '#ffffff', accent: '#f38518' },
  }

  const PREVIEW_MODES = [
    { value: 'off', label: 'Editor only' },
    { value: 'split', label: 'Split' },
    { value: 'full', label: 'Preview only' },
  ] as const

  const KEYBINDING_LABELS: Record<(typeof KEYBINDINGS)[number], string> = {
    vscode: 'VS Code',
    vim: 'Vim',
    emacs: 'Emacs',
  }

  // The dialog is mounted per opening, so the requested tab is read here.
  let tab = $state<Tab>(ui.settingsTab)
  // The swatches sit on the panel surface, which differs per theme: re-read it when the theme changes.
  const accentWarning = $derived.by(() => {
    if (!ui.accent) return false
    void ui.editorTheme
    return (contrast(ui.accent, surfaceForTheme()) ?? 99) < 3
  })
  const user = $derived(auth.user)
  const displayName = $derived(user?.name?.trim() || user?.login || '')
  const memberSince = $derived.by(() => {
    const raw = user?.createdAt
    if (!raw) return ''
    const date = new Date(raw)
    return Number.isNaN(date.getTime())
      ? ''
      : `Member since ${date.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })}`
  })
  const accountPending = $derived(auth.authState === 'pending')
  const changesLabel = $derived(
    gitStatus.changes.length === 0
      ? 'Clean'
      : `${gitStatus.changes.length} change${gitStatus.changes.length === 1 ? '' : 's'}`,
  )
  const syncLabel = $derived(
    gitStatus.ahead > 0 && gitStatus.behind > 0
      ? `${gitStatus.ahead} ahead · ${gitStatus.behind} behind`
      : gitStatus.ahead > 0
        ? `${gitStatus.ahead} to push`
        : gitStatus.behind > 0
          ? `${gitStatus.behind} to pull`
          : 'Up to date',
  )
  const lastPushLabel = $derived(
    gitStatus.lastPush ? gitStatus.lastPush.toLocaleString() : 'Never',
  )
  let accountError = $state('')
  let keyringOk = $state<boolean | null>(null)
  let pat = $state('')
  let signingIn = $state(false)
  let newExt = $state('')
  let newLang = $state('plaintext')
  let extMessage = $state('')
  let moving = $state(false)
  let connecting = $state(false)
  let localBusy = $state(false)
  let importOpen = $state(false)
  let importUrl = $state('')
  let importFolder = $state('')
  let importing = $state(false)
  let aboutVersion = $state('')
  let changelog = $state('')
  let update = $state<UpdateInfo | null>(null)
  let updateState = $state<'idle' | 'checking' | 'current' | 'newer' | 'failed'>('idle')
  const latestVersion = $derived(update?.latest ?? '')
  const releaseUrl = $derived(update?.url ?? '')
  // The changelog is prose here, not a code block: drop the Markdown heading markers.
  const changelogText = $derived(changelog.replace(/^#{2,3} (.+)$/gm, '$1'))

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
    void invoke<boolean>('auth_keyring_ok')
      .then((ok) => (keyringOk = ok))
      .catch(() => {})
    await loadUser()
    if (!auth.user) accountError = 'not connected'
    fonts = (await invoke<string[]>('fs_list_fonts').catch(() => [])) ?? []
    aboutVersion = await getVersion().catch(() => '')
    changelog = (await invoke<string>('app_changelog').catch(() => '')) ?? ''
  })

  async function checkForUpdates() {
    if (updateState === 'checking') return
    updateState = 'checking'
    try {
      update = await checkUpdate()
      updateState = update.newer ? 'newer' : 'current'
    } catch {
      // Offline, rate limited or private: the button is the only report.
      updateState = 'failed'
    }
  }

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
    const picked = await open({ directory: true, multiple: false, title: 'Move repository into' })
    if (typeof picked !== 'string' || !picked) return
    const sep = ui.repoPath.includes('\\') ? '\\' : '/'
    const name = ui.repoPath.split(/[\\/]/).filter(Boolean).pop() ?? ''
    const final = picked.replace(/[\\/]+$/, '') + sep + name
    const ok = await confirmDialog({
      kind: 'warning',
      title: 'Move repository',
      message: `Move ${ui.repoPath} into ${picked}? It will end up at ${final}.`,
      confirmLabel: 'Move',
    })
    if (!ok) return
    moving = true
    try {
      const moved = await invoke<{ from: string; to: string }>('git_move_repo', { parent: picked })
      ui.repoPath = moved.to
      ui.treeVersion += 1
      pushToast('Repository moved', 'success')
    } catch (e) {
      pushToast(`move repository: ${errorMessage(e)}`)
    } finally {
      moving = false
    }
  }

  async function chooseImportFolder() {
    try {
      const picked = await open({ directory: true, multiple: false, title: 'Clone repository into' })
      if (typeof picked === 'string' && picked) importFolder = picked
    } catch (e) {
      pushToast(`choose folder: ${errorMessage(e)}`)
    }
  }

  async function importRepo() {
    const url = importUrl.trim()
    if (!url || importing) return
    // Unsaved edits belong to the repo being replaced, so they must land first.
    if (!(await flushPendingSave())) return
    importing = true
    try {
      const result = await invoke<{ from: string; to: string }>('git_import_repo', {
        url,
        parent: importFolder || null,
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
      pushToast('Repository imported', 'success')
    } catch (e) {
      pushToast(`import repository: ${errorMessage(e)}`)
    } finally {
      importing = false
    }
  }

  async function useLocalRepo() {
    const picked = await open({ directory: true, multiple: false, title: 'Open a local repository' })
    if (typeof picked !== 'string' || !picked || localBusy) return
    // Edits pending in the current repo must land before switching away.
    if (!(await flushPendingSave())) return
    localBusy = true
    try {
      const result = await invoke<{ from: string; to: string; initialized: boolean; remote: boolean }>(
        'git_use_local',
        {
          path: picked,
        },
      )
      // Tabs point into the folder being replaced, so they close like on import.
      if (result.to !== result.from) {
        for (const tab of [...tabs.tabs]) closeTab(tab.id)
        ui.expanded = []
      }
      ui.repoPath = result.to
      ui.treeVersion += 1
      void invoke<GitStatus>('git_status').then(applyGitStatus).catch(() => {})
      pushToast(
        result.initialized
          ? 'Created a new repository here; the next save commits every file in this folder'
          : result.remote
            ? 'Local repository opened'
            : 'Local repository opened (no remote)',
        result.initialized ? 'warning' : 'success',
      )
    } catch (e) {
      pushToast(`open local repository: ${errorMessage(e)}`)
    } finally {
      localBusy = false
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
    // Stay on the form: the onboarding wizard is for the first run, not for
    // signing back in.
    forgetUser()
    accountError = ''
  }

  async function refreshUser() {
    await loadUser(true)
    accountError = auth.user ? '' : 'not connected'
  }

  function openProfile() {
    if (user) void openExternal(`https://github.com/${user.login}`)
  }

  /// A local-only folder keeps its history: attach `origin` and push, no clone.
  async function attachToGithub() {
    if (connecting) return
    connecting = true
    try {
      const repo = await invoke<{ cloneUrl: string }>('github_init_repo', { init: false })
      applyGitStatus(await invoke<GitStatus>('git_attach_origin', { url: repo.cloneUrl }))
      ui.treeVersion += 1
      pushToast('Repository connected to GitHub', 'success')
    } catch (e) {
      pushToast(`connect repository: ${errorMessage(e)}`)
    } finally {
      connecting = false
    }
  }

  async function signIn() {
    const token = pat.trim()
    if (!token || signingIn) return
    signingIn = true
    try {
      // Validate before storing: a bad token must never reach the keyring.
      await signInWithPat(token)
      pat = ''
      accountError = ''
    } catch (e) {
      pushToast(`sign in: ${e}`)
    } finally {
      signingIn = false
    }
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
  <div class="modal" role="dialog" aria-modal="true" aria-label="Settings" use:focusTrap>
    <header>
      <h2>Settings</h2>
      <button class="icon-btn lg close" aria-label="Close settings" onclick={() => (ui.settingsOpen = false)}>
        ×
      </button>
    </header>

    <div class="layout">
      <div
        class="rail"
        role="tablist"
        aria-orientation="vertical"
        aria-label="Settings sections"
        tabindex="-1"
        onkeydown={onNavKey}
      >
        {#each TABS as item (item.id)}
          <button
            role="tab"
            id="settings-tab-{item.id}"
            aria-controls="settings-panel"
            class="tab"
            class:active={tab === item.id}
            aria-selected={tab === item.id}
            tabindex={tab === item.id ? 0 : -1}
            onclick={() => (tab = item.id)}
          >
            <svg viewBox={item.box ?? '0 0 16 16'} aria-hidden="true">
              <path d={item.icon} />
            </svg>
            <span>{item.label}</span>
          </button>
        {/each}
      </div>

      <div
        class="body"
        class:logs={tab === 'logs'}
        id="settings-panel"
        role="tabpanel"
        aria-labelledby="settings-tab-{tab}"
      >
        {#if tab === 'appearance'}
          <section class="section">
            <h3>Appearance</h3>
            <div class="field stacked">
              <span>Theme</span>
              <div class="themes" role="radiogroup" aria-label="Theme">
                {#each THEMES as theme (theme)}
                  {@const preview = THEME_PREVIEW[theme]}
                  <label class="theme-card" class:active={ui.editorTheme === theme}>
                    <input
                      type="radio"
                      name="theme"
                      value={theme}
                      bind:group={ui.editorTheme}
                    />
                    <span
                      class="preview"
                      aria-hidden="true"
                      style="--p-bg: {preview?.bg}; --p-fg: {preview?.fg}; --p-accent: {preview?.accent}"
                    >
                      <span class="bar title"></span>
                      <span class="bar line"></span>
                      <span class="bar accent"></span>
                    </span>
                    <span class="name">{preview?.name ?? theme}</span>
                  </label>
                {/each}
              </div>
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
          </section>
        {:else if tab === 'sync'}
          <section class="section">
            <h3>Automatic sync</h3>
            <label class="field">
              <span>Push automatically after each save</span>
              <Check bind:checked={ui.autoSync} />
            </label>
            <label class="field">
              <span>Autosave delay</span>
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
            </label>
            <label class="field" class:disabled={!ui.autoSync}>
              <span>Push delay</span>
              <div class="inline">
                <input
                  type="number"
                  min="5"
                  max="300"
                  disabled={!ui.autoSync}
                  value={Math.round(ui.pushIdleMs / 1000)}
                  onchange={(e) =>
                    clamp(e.currentTarget.valueAsNumber, 5, 300, (n) => (ui.pushIdleMs = n * 1000))}
                />
                <span class="hint">
                  {ui.autoSync ? 'seconds after the last edit' : 'only with automatic push'}
                </span>
              </div>
            </label>
          </section>

          <section class="section">
            <h3>Files</h3>
            <label class="field">
              <span>Sort by</span>
              <select
                bind:value={ui.sortMode}
                onchange={() => (ui.sortDesc = defaultDesc(ui.sortMode))}
              >
                <option value="name">Name</option>
                <option value="modified">Modified</option>
                <option value="type">Type</option>
              </select>
            </label>
            <label class="field">
              <span>Descending</span>
              <Check bind:checked={ui.sortDesc} />
            </label>
            <label class="field">
              <span>Folders first</span>
              <Check bind:checked={ui.foldersFirst} />
            </label>
            <label class="field">
              <span>Show hidden files</span>
              <Check bind:checked={ui.showHidden} />
            </label>
          </section>

          <section class="section">
            <h3>Repository</h3>
            <div class="repo">
              <p class="folder">
                <span class="label">Folder</span>
                <span class="path">{ui.repoPath || 'Default app folder'}</span>
              </p>
              <div class="actions">
                <button
                  type="button"
                  class="btn"
                  use:tip={'Open an existing folder; a plain folder is initialised with git'}
                  disabled={localBusy}
                  onclick={useLocalRepo}
                >
                  {localBusy ? 'Opening…' : 'Use another folder…'}
                </button>
                <button
                  type="button"
                  class="btn"
                  use:tip={'Move the folder on disk to another location'}
                  disabled={moving || !ui.repoPath}
                  onclick={moveRepo}
                >
                  {moving ? 'Moving…' : 'Move folder…'}
                </button>
                <button
                  type="button"
                  class="btn"
                  aria-expanded={importOpen}
                  onclick={() => (importOpen = !importOpen)}
                >
                  {importOpen ? 'Cancel import' : 'Import…'}
                </button>
              </div>
              {#if importOpen}
                <div class="import">
                  <input
                    type="text"
                    class="url"
                    placeholder="https://github.com/user/repo.git"
                    aria-label="Repository URL to import"
                    bind:value={importUrl}
                    onkeydown={(e) => {
                      if (e.key === 'Enter') void importRepo()
                    }}
                  />
                  <div class="pick">
                    <span class="hint path">
                      {importFolder || 'Default folder (next to your documents)'}
                    </span>
                    <button
                      type="button"
                      class="btn"
                      disabled={importing}
                      onclick={chooseImportFolder}
                    >
                      Choose…
                    </button>
                  </div>
                  <div class="actions">
                    <button
                      type="button"
                      class="btn btn-primary sweep"
                      disabled={importing || !importUrl.trim()}
                      onclick={importRepo}
                    >
                      {importing ? 'Cloning…' : 'Clone as active repository'}
                    </button>
                    <span class="hint">Open tabs close: they belong to the current repository.</span>
                  </div>
                </div>
              {/if}
            </div>
          </section>
        {:else if tab === 'editor'}
          <section class="section">
            <h3>Font</h3>
            <label class="field">
              <span>Font size</span>
              <div class="inline">
                <input
                  type="range"
                  min="8"
                  max="32"
                  step="1"
                  value={ui.fontSize}
                  oninput={(e) => (ui.fontSize = e.currentTarget.valueAsNumber)}
                />
                <span class="value">{ui.fontSize}px</span>
              </div>
            </label>
            <label class="field">
              <span>Font family</span>
              <select class="wide" bind:value={ui.fontFamily}>
                <option value={DEFAULT_FONT}>Default (monospace)</option>
                {#each fontOptions as font (font)}
                  <option value={font}>{font}</option>
                {/each}
              </select>
            </label>
            <p
              class="font-sample"
              style="font-family: {ui.fontFamily}; font-size: {ui.fontSize}px"
            >
              The quick brown fox
            </p>
          </section>

          <section class="section">
            <h3>Markdown</h3>
            <div class="field">
              <span id="editor-preview-label">Markdown preview</span>
              <div
                class="segmented"
                role="radiogroup"
                aria-labelledby="editor-preview-label"
              >
                {#each PREVIEW_MODES as mode (mode.value)}
                  <label class="segment" class:active={ui.previewMode === mode.value}>
                    <input
                      type="radio"
                      name="editor-preview"
                      value={mode.value}
                      bind:group={ui.previewMode}
                    />
                    <span>{mode.label}</span>
                  </label>
                {/each}
              </div>
            </div>
            <p class="note">The same setting as the editor's preview buttons.</p>
          </section>

          <section class="section">
            <h3>Keybindings</h3>
            <div class="field">
              <span id="editor-keybindings-label">Mode</span>
              <div
                class="segmented"
                role="radiogroup"
                aria-labelledby="editor-keybindings-label"
              >
                {#each KEYBINDINGS as mode (mode)}
                  <label class="segment" class:active={ui.keybindings === mode}>
                    <input
                      type="radio"
                      name="editor-keybindings"
                      value={mode}
                      bind:group={ui.keybindings}
                    />
                    <span>{KEYBINDING_LABELS[mode]}</span>
                  </label>
                {/each}
              </div>
            </div>
            <p class="note">
              {ui.keybindings === 'vscode'
                ? 'Standard Monaco shortcuts.'
                : `${KEYBINDING_LABELS[ui.keybindings]} keybindings are applied to the editor; Ctrl+P still opens the palette.`}
            </p>
          </section>
        {:else if tab === 'extensions'}
          <section class="section">
            <h3>File types</h3>
            {#if extensions.length === 0}
              <p class="empty">
                No custom file types yet. Add one below to override the language GitPad uses.
              </p>
            {:else}
              <ul class="extensions">
                {#each extensions as [ext, lang] (ext)}
                  <li>
                    <span class="ext">.{ext}</span>
                    <span class="lang">{lang}</span>
                    <button
                      class="icon-btn lg remove"
                      aria-label="Remove .{ext}"
                      onclick={() => removeExtension(ext)}
                    >
                      ×
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
            <div class="add">
              <label class="add-field">
                <span class="label">Extension</span>
                <input
                  placeholder="e.g. rs"
                  bind:value={newExt}
                  onkeydown={(e) => {
                    if (e.key === 'Enter') addExtension()
                  }}
                />
              </label>
              <label class="add-field">
                <span class="label">Language</span>
                <select bind:value={newLang}>
                  {#each LANGUAGES as lang (lang)}
                    <option value={lang}>{lang}</option>
                  {/each}
                </select>
              </label>
              <button class="btn btn-primary sweep" onclick={addExtension}>Add</button>
            </div>
            {#if extMessage}
              <p class="note" role="status">{extMessage}</p>
            {/if}
          </section>
        {:else if tab === 'logs'}
          <section class="section">
            <LogsPane />
          </section>
        {:else if tab === 'about'}
          <section class="section">
            <div class="about-card">
              <div class="about-head">
                <div class="about-id">
                  <h3 class="about-name">GitPad</h3>
                  {#if aboutVersion}
                    <span class="about-version">v{aboutVersion}</span>
                  {/if}
                </div>
                <button
                  class="btn sweep"
                  disabled={updateState === 'checking'}
                  onclick={checkForUpdates}
                >
                  {updateState === 'checking' ? 'Checking…' : 'Check for updates'}
                </button>
              </div>
              {#if updateState === 'newer'}
                <div class="inline">
                  <button class="btn btn-primary" onclick={() => void openExternal(releaseUrl)}>
                    Get GitPad {latestVersion}
                  </button>
                </div>
              {:else if updateState === 'current'}
                <p class="note" role="status">You are on the latest version.</p>
              {:else if updateState === 'failed'}
                <p class="note" role="status">Could not reach GitHub. Try again later.</p>
              {/if}
              <div class="about-links">
                <button class="link" onclick={() => void openExternal('https://github.com/fedeaielli')}>
                  @fedeaielli
                </button>
                <button
                  class="link"
                  onclick={() => void openExternal('https://github.com/fedeaielli/gitpad')}
                >
                  Source
                </button>
                <button
                  class="link"
                  onclick={() => void openExternal('https://github.com/fedeaielli/gitpad/releases')}
                >
                  Releases
                </button>
              </div>
            </div>
            {#if changelog}
              <h3>Changelog</h3>
              <pre class="changelog" aria-label="Changelog for this version">{changelogText}</pre>
            {/if}
          </section>
        {:else}
          {#if user}
            <section class="section">
              <h3>Profile</h3>
              <div class="profile-card">
                <button class="avatar" aria-label="Open GitHub profile" onclick={openProfile}>
                  <img src={user.avatarUrl} alt="" />
                </button>
                <div class="identity">
                  <button class="profile-name" onclick={openProfile}>{displayName}</button>
                  {#if displayName !== user.login}
                    <span class="handle">@{user.login}</span>
                  {/if}
                  {#if user.bio?.trim()}
                    <p class="bio">{user.bio.trim()}</p>
                  {/if}
                  {#if memberSince}
                    <p class="joined">{memberSince}</p>
                  {/if}
                </div>
              </div>
            </section>

            <section class="section">
              <h3>Repository</h3>
              {#if gitStatus.configured}
                <div class="repo account-repo">
                  <div class="field">
                    <span>Folder</span>
                    <span class="path">{ui.repoPath || 'Default app folder'}</span>
                  </div>
                  {#if gitStatus.branch}
                    <div class="field">
                      <span>Branch</span>
                      <span class="mono">{gitStatus.branch}</span>
                    </div>
                  {/if}
                  <div class="field"><span>Changes</span><span>{changesLabel}</span></div>
                  <div class="field">
                    <span>Remote</span>
                    <span>{gitStatus.hasRemote ? 'Connected' : 'Local only'}</span>
                  </div>
                  {#if gitStatus.hasRemote}
                    <div class="field"><span>Sync</span><span>{syncLabel}</span></div>
                    <div class="field"><span>Last push</span><span>{lastPushLabel}</span></div>
                  {/if}
                  {#if gitStatus.conflicts.length > 0}
                    <div class="field conflict">
                      <span>Conflicts</span>
                      <span>{gitStatus.conflicts.length} to resolve</span>
                    </div>
                  {/if}
                  {#if !gitStatus.hasRemote}
                    <div class="field">
                      <span>GitHub repository</span>
                      <div class="inline">
                        <span class="hint">Local only — commits stay on this machine</span>
                        <button class="btn" disabled={connecting} onclick={attachToGithub}>
                          {connecting ? 'Connecting…' : 'Connect'}
                        </button>
                      </div>
                    </div>
                  {/if}
                </div>
              {:else}
                <div class="repo-empty">
                  <p class="note">
                    No repository yet. Set one up in Sync to start committing your notes.
                  </p>
                  <button class="btn" onclick={() => (tab = 'sync')}>Open Sync</button>
                </div>
              {/if}
            </section>

            <section class="section">
              <h3>Session</h3>
              <div class="field">
                <span>Profile data</span>
                <button class="btn" disabled={accountPending} onclick={refreshUser}>
                  {accountPending ? 'Refreshing…' : 'Refresh account'}
                </button>
              </div>
              {#if keyringOk !== null}
                <div class="field">
                  <span>Token storage</span>
                  <span>{keyringOk ? 'OS keyring' : 'Protected file'}</span>
                </div>
                {#if !keyringOk}
                  <p class="note">
                    The OS keyring is unavailable: the token is kept in a protected file on this
                    machine.
                  </p>
                {/if}
              {/if}
              <div class="field">
                <span>GitHub tokens</span>
                <button
                  class="link"
                  onclick={() => void openExternal('https://github.com/settings/tokens')}
                >
                  Manage on GitHub
                </button>
              </div>
              <div class="field">
                <span>End this session. Your notes stay on this machine.</span>
                <button class="btn btn-danger" onclick={logout}>Logout</button>
              </div>
            </section>
          {:else}
            <section class="section">
              <div class="field">
                <span>GitHub login</span>
                <GithubLogin onsuccess={refreshUser} />
              </div>
              <p class="note">Or paste a personal access token (scope: repo).</p>
              <div class="field">
                <span>Personal access token</span>
                <div class="inline">
                  <input
                    type="password"
                    placeholder="ghp_…"
                    aria-label="Personal access token"
                    aria-invalid={!!accountError}
                    aria-describedby={accountError ? 'pat-account-error' : undefined}
                    bind:value={pat}
                    onkeydown={(e) => e.key === 'Enter' && signIn()}
                  />
                  <button class="btn btn-primary" disabled={!pat.trim() || signingIn} onclick={signIn}>
                    {signingIn ? 'Signing in…' : 'Sign in'}
                  </button>
                </div>
              </div>
              {#if accountError}
                <p class="note" id="pat-account-error" role="alert">{accountError}</p>
              {/if}
            </section>
          {/if}
        {/if}
      </div>
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
    animation: gp-fade-in var(--t-base) var(--ease-out);
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
    width: min(960px, 94vw);
    height: min(660px, 90vh);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--r-overlay);
    background: var(--surface-1);
    box-shadow: var(--shadow-2);
    animation: gp-panel-in var(--t-spring) var(--ease-spring);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-overlay) var(--r-overlay) 0 0;
    background: var(--surface-1);
  }
  h2 {
    margin: 0;
    color: var(--text);
    font-size: var(--fs-lg);
    font-weight: 600;
  }
  .close {
    font-size: var(--fs-xl);
  }
  .layout {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  /* The sections as a column: one row each, the same height, the rail's full
     width. Selection is an accent tint on the row, never a type step. */
  .rail {
    display: flex;
    flex: 0 0 200px;
    flex-direction: column;
    gap: var(--sp-0);
    overflow-y: auto;
    padding: var(--sp-2);
    border-right: 1px solid var(--divider);
    background: var(--surface-1);
  }
  .tab {
    display: flex;
    flex: 0 0 56px;
    align-items: center;
    gap: var(--sp-25);
    width: 100%;
    padding: 0 var(--sp-3);
    border: none;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--muted);
    font-size: var(--fs-lg);
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }
  .tab svg {
    flex: 0 0 16px;
    width: 16px;
    height: 16px;
    fill: currentColor;
  }
  .tab span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .tab:hover {
    background: color-mix(in srgb, var(--surface-2) 55%, transparent);
    color: var(--text);
  }
  /* The same accent tint as a toast: the selected section reads as a tinted
     surface, not as a stripe on its edge. */
  .tab.active {
    background: color-mix(in srgb, var(--accent) 12%, var(--surface-1));
    color: var(--text);
  }
  .tab.active svg {
    color: var(--accent-fg);
  }
  .body {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--sp-4);
    min-width: 0;
    overflow-y: auto;
    padding: var(--sp-4) var(--sp-6) var(--sp-6);
    background: var(--surface-0);
    /* One type scale for every tab: rows, values and controls read at 16 px,
       the hints and group titles sit a step below. */
    font-size: var(--fs-lg);
  }
  .body.logs {
    overflow: hidden;
  }
  /* The log owns the height left under its toolbar. */
  .body.logs .section {
    flex: 1;
    min-height: 0;
  }
  /* Rows run the panel's full width: the label opens the row and the control
     closes it, so the eye scans one left edge and one right edge. */
  .field {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--sp-4);
    min-height: 32px;
  }
  .field > :last-child {
    justify-self: end;
  }
  /* The reason is in the hint; the row does not pretend to be editable. */
  .field.disabled {
    opacity: 0.6;
  }
  /* Stacked rows: the label opens the line, the control owns the width below. */
  .field.stacked {
    grid-template-columns: minmax(0, 1fr);
    align-items: start;
    gap: var(--sp-2);
  }
  .field.stacked > :last-child {
    justify-self: stretch;
  }
  /* Grouped settings: one heading names what the rows below decide. */
  .section {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    width: 100%;
  }
  .section > h3 {
    margin: 0;
    color: var(--accent-fg);
    font-size: var(--fs-sm);
    font-weight: 500;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  /* Divider between groups only: the first one opens the tab under its title. */
  .section + .section {
    padding-top: var(--sp-3);
    border-top: 1px solid var(--divider);
  }
  .modal input[type='range'] {
    accent-color: var(--accent);
  }
  .field > span:first-child {
    color: var(--muted);
  }
  /* One control width: the value it sets, not the whole dialog. */
  .field select,
  .field input[type='text'],
  .field input[type='number'],
  .field .inline,
  .field .btn {
    max-width: 340px;
  }
  /* The native stepper is a fourth style; the range already sets the value. */
  .modal input[type='number'] {
    appearance: textfield;
  }
  .modal input[type='number']::-webkit-inner-spin-button {
    display: none;
  }
  .inline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: var(--sp-25);
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
    max-width: 340px;
  }
  /* One line at the chosen family and size: dragging the slider must not
     reflow the tab, so the box keeps its height and clips. */
  .font-sample {
    display: flex;
    align-items: center;
    min-height: 68px;
    margin: 0;
    padding: var(--sp-3) var(--sp-4);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
    color: var(--text);
    line-height: 1.3;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  /* Three choices at most: a track beats a select that hides two of them. */
  .segmented {
    display: inline-flex;
    gap: var(--sp-0);
    padding: var(--sp-0);
    border: 1px solid var(--border);
    border-radius: var(--r-pill);
    background: var(--surface-1);
    box-shadow: var(--shadow-1);
  }
  .segment {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 26px;
    padding: 0 var(--sp-2);
    border-radius: var(--r-pill);
    color: var(--muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
    cursor: pointer;
    transition:
      background var(--t-base) var(--ease-out),
      color var(--t-base) var(--ease-out),
      box-shadow var(--t-base) var(--ease-out);
  }
  .segment:hover {
    color: var(--text);
  }
  .segment.active {
    background: var(--text);
    color: var(--surface-0);
    box-shadow: var(--shadow-1);
  }
  .segment:focus-within {
    z-index: 1;
    outline: var(--focus-ring);
    outline-offset: 2px;
  }
  .segment input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }
  .hint,
  .note {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .path {
    overflow-wrap: anywhere;
  }
  /* About and Account share the one card surface: identity first, then detail. */
  .about-card,
  .profile-card {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: var(--surface-1);
  }
  .about-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
  }
  .about-id {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
  }
  /* The app names itself here: not the uppercase section eyebrow. */
  .about-card .about-name {
    margin: 0;
    color: var(--text);
    font-size: var(--fs-xl);
    font-weight: 600;
    letter-spacing: -0.01em;
    text-transform: none;
  }
  .about-version {
    padding: var(--sp-0) var(--sp-2);
    border-radius: var(--r-pill);
    background: var(--surface-2);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .profile-card {
    flex-direction: row;
    align-items: flex-start;
  }
  .avatar {
    flex: 0 0 auto;
    width: 64px;
    height: 64px;
    padding: 0;
    overflow: hidden;
    border: none;
    border-radius: 50%;
    background: none;
    cursor: pointer;
  }
  .avatar img {
    display: block;
    width: 100%;
    height: 100%;
  }
  .avatar:hover {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .identity {
    display: flex;
    flex-direction: column;
    gap: var(--sp-05);
    min-width: 0;
  }
  .profile-name {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: var(--fs-lg);
    font-weight: 600;
    letter-spacing: -0.01em;
    text-align: left;
    cursor: pointer;
  }
  .profile-name:hover {
    color: var(--accent-fg);
    text-decoration: underline;
  }
  .handle {
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .bio {
    display: -webkit-box;
    margin: 0;
    overflow: hidden;
    color: var(--text);
    font-size: var(--fs-md);
    line-height: 1.45;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }
  .joined {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .avatar:focus-visible,
  .profile-name:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }
  .about-card .note {
    margin: 0;
  }
  .about-links {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-3);
    padding-top: var(--sp-3);
    border-top: 1px solid var(--divider);
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
    text-decoration: underline;
  }
  .changelog {
    margin: 0;
    padding: var(--sp-3);
    border: 1px solid var(--divider);
    border-radius: var(--r-md);
    background: var(--surface-1);
    color: var(--text);
    font-size: var(--fs-sm);
    line-height: 1.55;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  /* The repository as one object: where it lives, then what can move it. */
  .repo {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: var(--surface-1);
  }
  .account-repo .path {
    max-width: 60%;
    min-width: 0;
    font-family: ui-monospace, monospace;
    font-size: var(--fs-sm);
    text-align: right;
  }
  .account-repo .mono {
    font-family: ui-monospace, monospace;
    font-size: var(--fs-sm);
  }
  .field.conflict > :last-child {
    color: var(--err);
  }
  .repo-empty {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border: 1px solid var(--border);
    border-radius: var(--r-lg);
    background: var(--surface-1);
  }
  .repo-empty .note {
    margin: 0;
  }
  .folder {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    margin: 0;
    min-width: 0;
  }
  .folder .label {
    flex: 0 0 auto;
    color: var(--muted);
    font-size: var(--fs-2xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .folder .path {
    min-width: 0;
    font-family: ui-monospace, monospace;
    font-size: var(--fs-sm);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
  }
  /* Import is a well inside the card: this is the field you fill in. */
  .import {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    padding: var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-0);
  }
  .import .url {
    width: 100%;
    max-width: none;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
  }
  .pick .hint {
    flex: 1;
    min-width: 0;
  }
  .accent-picker {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: var(--sp-2);
    max-width: 100%;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--sp-2);
  }
  /* A radio each, so arrow keys work for free; the card is the visible control. */
  .theme-card {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
    padding: var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
    cursor: pointer;
  }
  .theme-card:hover {
    border-color: var(--border-strong);
  }
  .theme-card.active {
    border-color: var(--accent-fg);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface-1));
  }
  .theme-card:focus-within {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }
  .theme-card input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }
  /* A window in miniature: ground, text lines, the theme's accent. */
  .preview {
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    width: 56px;
    height: 36px;
    padding: 0 var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--p-bg);
  }
  .bar {
    height: 3px;
    border-radius: var(--sp-0);
    background: var(--p-fg);
  }
  .bar.title {
    width: 24px;
  }
  .bar.line {
    width: 34px;
    opacity: 0.55;
  }
  .bar.accent {
    width: 18px;
    background: var(--p-accent);
  }
  .name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .swatch {
    width: 24px;
    height: 24px;
    /* Centred inside the 30 px row the other controls set. */
    margin: var(--sp-0) 0;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    background: var(--swatch-color);
    cursor: pointer;
  }
  .swatch.active,
  .swatch:focus-visible {
    border-color: var(--text);
    box-shadow: 0 0 0 2px var(--accent-fg);
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
    margin: var(--sp-3) 0 0;
  }
  .value {
    min-width: 44px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .extensions {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
  }
  .extensions li {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr) 32px;
    align-items: center;
    gap: var(--sp-3);
    min-height: 44px;
    padding: 0 var(--sp-2) 0 var(--sp-3);
  }
  .extensions li + li {
    border-top: 1px solid var(--divider);
  }
  .ext,
  .lang {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .ext {
    font-family: ui-monospace, monospace;
  }
  .lang {
    color: var(--muted);
  }
  .remove:hover:not(:disabled) {
    background: color-mix(in srgb, var(--err) 12%, transparent);
    color: var(--err);
  }
  .empty {
    margin: 0;
    padding: var(--sp-4);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--surface-1);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .add {
    display: grid;
    grid-template-columns: 110px minmax(0, 1fr) auto;
    align-items: end;
    gap: var(--sp-2);
  }
  .add-field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-05);
    min-width: 0;
  }
  .add-field .label {
    color: var(--accent-fg);
    font-size: var(--fs-xs);
  }
  .add input,
  .add select {
    box-sizing: border-box;
    width: 100%;
    height: 32px;
    min-width: 0;
  }
  /* The window's minimum is 720 px: the rail gives 40 px back before the
     labels start wrapping. */
  @media (max-width: 820px) {
    .rail {
      flex-basis: 160px;
    }
  }
</style>
