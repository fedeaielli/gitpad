# GitPad

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey.svg)](#install)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB.svg)](https://tauri.app)
[![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00.svg)](https://svelte.dev)

GitPad is a local-first desktop editor for Markdown notes and small text files that live in a Git repository you own. It pairs the Monaco editor with a focused GitHub workflow: edit, save automatically, commit locally, and push when you stop typing.

Built with **Tauri 2**, **Rust**, **Svelte 5**, and **Monaco**. Linux first.

> **Status:** early development (`v0.1.x`). GitPad writes to a real Git repository, so keep backups and use a dedicated notes repository while the app is still young.

---

## Why GitPad

Most note apps keep your writing in a proprietary database. GitPad keeps it in plain files inside a Git repository:

- **Local-first.** Your notes are ordinary files on disk. You can edit them with any editor, inspect them with `git`, and read them without GitPad.
- **GitHub-native.** Pull at startup, commit on save, push when you pause. No separate sync service.
- **Markdown-friendly.** Split or full-screen preview with GitHub-flavored Markdown, syntax highlighting, and scroll sync.
- **Keyboard-first.** Command palette, file search, tabs, and configurable VS Code/Vim/Emacs keybindings.
- **Small surface area.** One local user, one repository, no plugin runtime.

---

## Features

- **Monaco editor** with syntax highlighting for the languages Monaco supports, tabs, find/replace, and a dark default theme.
- **Automatic save and sync.** Edits are written after a short debounce (2 s by default), committed locally, and pushed after you stop typing (30 s by default), on window blur, or on close.
- **GitHub integration.** Connect with a personal access token, create a private `gitpad_notes` repository by default or clone an existing one, pull at startup, and push from the Git panel.
- **Conflict resolution.** When a pull conflicts, GitPad opens a diff dialog with local and remote versions. Accept one side, edit manually, then confirm the merge.
- **Markdown preview.** Toggle off, split, or full screen with `Ctrl+Shift+P`; GitHub-flavored Markdown, syntax highlighting, and editor/preview scroll sync.
- **File tree.** Create, rename, delete, import files or folders, and jump between notes. Folder creation keeps a `.gitkeep`.
- **Commit history.** Browse the repository log, inspect changed files, restore a file from a commit, or revert a commit.
- **Command palette.** `Ctrl+P` for files, `>` for commands, and `>search` for full-text search.
- **Settings.** Theme, font size, auto-sync, debounce/idle timing, keybindings, and file-extension-to-language mappings.
- **Credentials in the OS keyring.** The GitHub token is never written to `config.json`.

---

## Install

### AppImage (recommended)

Download the latest AppImage from the [releases page](https://github.com/fedeaielli/gitpad/releases), then:

```sh
chmod +x GitPad_*.AppImage
./GitPad_*.AppImage
```

The AppImage needs `webkit2gtk-4.1` and `libsecret` on the host.

### Arch Linux

A `PKGBUILD` is included in the repository root. It is **not published on the AUR yet**.

```sh
git clone https://github.com/fedeaielli/gitpad.git
cd gitpad
makepkg -si
```

### From source

Requirements:

- Rust **1.90+**
- Node.js **20+** (current LTS recommended)
- Linux build dependencies: `webkit2gtk-4.1`, `gtk3`, `libsoup3`, `openssl`, `libsecret`, `libgit2`

On Arch Linux:

```sh
sudo pacman -S --needed base-devel webkit2gtk-4.1 gtk3 libsoup3 openssl libsecret libgit2 nodejs npm rust
```

Then:

```sh
npm ci
npm run tauri dev      # development
npm run tauri build    # release build
```

The AppImage is written to `src-tauri/target/release/bundle/`.

---

## First start

1. **Connect GitHub.** The onboarding wizard asks for a personal access token (classic) with the `repo` scope. Create one at <https://github.com/settings/tokens>. The token is stored in the OS keyring, not in a config file.
2. **Choose a repository.** By default GitPad looks for a private repository named `gitpad_notes`, creates it if it does not exist, and clones it into the app data directory.
3. **Start writing.** With auto-sync on (the default), files are committed and pushed automatically. You can also skip the wizard and clone an existing repository from the Git panel.

GitHub sign-in currently uses a PAT; the OAuth device-flow code path is present but not configured in the app yet.

---

## How sync works

With auto-sync enabled (the default):

1. You type. After the save debounce, GitPad writes the file atomically and creates a local commit:
   `gitpad: update <filename> <ISO-8601 timestamp>`
2. After the push idle timeout, on window blur, or on close, GitPad pushes the local commits.
3. At startup GitPad pulls the configured repository. If the working tree has conflicts, it opens the conflict dialog instead of overwriting anything.
4. The Git panel also exposes manual **Sync**, **Pull**, **Push**, and **Commit** actions.

GitPad never force-pushes and never rebases. Pulls fast-forward when possible, otherwise they create a merge commit.

---

## Shortcuts

| Action | Shortcut |
|---|---|
| Command palette | `Ctrl+P` |
| Cycle Markdown preview: off → split → full | `Ctrl+Shift+P` |
| Move selection in the palette | `Up` / `Down` |
| Open, run, or submit | `Enter` |
| Close the palette | `Esc` |

There is no save shortcut: saving is automatic.

Mouse and UI actions:

- Right-click a file in the tree to create, rename, or delete.
- Middle-click a tab to close it.
- Click the conflict badge in the status bar to reopen the merge dialog.
- Open settings from the gear in the status bar.

See [SHORTCUTS.md](SHORTCUTS.md) for the full list.

---

## Where things are stored

On Linux, under `~/.local/share/com.gitpad.app/`:

| Path | Contents |
|---|---|
| `repos/gitpad_notes/` | Local Git clone of your notes |
| `config.json` | Theme, font size, sync settings, extension mappings |
| OS keyring | GitHub token |

Deleting `config.json` resets settings but leaves the notes repository untouched.

---

## Limits worth knowing

- **One repository at a time.** Switching repositories currently means removing the existing clone by hand.
- **External edits are focus-based.** Changes made outside GitPad appear after the window regains focus; an open tab is not reloaded automatically.
- **Remote images are blocked in preview.** The content security policy allows local/data images and GitHub avatars, not arbitrary remote images.
- **No filesystem watcher.** The file tree refreshes on focus, not through inotify.
- **Crash safety.** `SIGTERM` commits and pushes files already written before exit. `SIGKILL` or a crash cannot be caught; the last edits remain local until the next start.
- **Linux only.** The current release bundles an AppImage. Windows is not supported yet.
- **Not a full Git client.** There is no branch UI, terminal, plugin system, or multi-user collaboration.

---

## Privacy

- GitPad talks to GitHub only for the repository operations you configure.
- The token lives in the OS keyring and is never written to `config.json`.
- There is no analytics, telemetry, or third-party crash reporting.
- Monaco and all front-end assets are bundled locally; the app does not load them from a CDN.

---

## Development

```sh
npm ci
npm run tauri dev
```

Useful checks:

```sh
npm test                                  # Vitest
npm run check                             # svelte-check + TypeScript
cargo test --manifest-path src-tauri/Cargo.toml
npm run build                             # front-end production build
npm run tauri build                       # AppImage
```

### Manual close-path check

To exercise the unsaved-changes dialog:

```sh
chmod -R a-w ~/.local/share/com.gitpad.app/repos/gitpad_notes
```

Edit a file and close the window. GitPad should show **“Some changes could not be saved. Exit anyway?”**. Cancel, restore write permissions, then repeat and confirm:

```sh
chmod -R u+w ~/.local/share/com.gitpad.app/repos/gitpad_notes
```

A 5-second force-close watchdog only applies if the webview hangs; it is not active while the dialog is open.

---

## Project layout

```text
src/                    Svelte 5 front end
  lib/components/       editor, sidebar, Git panel, dialogs, settings
  lib/stores/           tabs, Git state, auth, UI state
  lib/utils/            language map, debounce, save guard, pull reload
src-tauri/src/          Rust back end
  fs.rs                 file reads/writes, atomic saves, imports
  git.rs                status, pull/push, history, conflicts
  github.rs             GitHub API
  auth.rs               OS keyring
  search.rs             repository search
```

---

## Contributing

Issues and pull requests are welcome. This is a personal project, so please open an issue before large changes.

Before sending a PR, run:

```sh
npm test
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## License

GitPad is released under the [GNU General Public License v3.0](LICENSE).

Copyright © 2026 Fede.
