# GitPad

A desktop editor for markdown notes that live in a Git repository you own.
Svelte 5 + Monaco front end, Tauri 2 + Rust back end, Linux first.

- One repository, GitHub push/pull, no merge tool required for daily use.
- Automatic save (debounced) and automatic push after you stop typing.
- Markdown preview with scroll sync.
- Merge conflicts resolved from a diff dialog (local / remote / edit manually).
- Settings for theme, font, keybindings (VS Code, Vim, Emacs) and file extensions.
- Credentials live in the OS keyring, never in a config file.

## Install

### AppImage

Download the `.AppImage` from the releases page, then:

```sh
chmod +x GitPad_*.AppImage
./GitPad_*.AppImage
```

No installation step; move it wherever you like. It needs `webkit2gtk-4.1` and
`libsecret` on the system (installed by default on most desktops).

### Arch Linux

`PKGBUILD` is in the repository root but **not yet published on the AUR**.
Build it locally:

```sh
git clone https://github.com/fedeaielli/gitpad.git
cd gitpad
makepkg -si
```

### From source

Requirements: Rust (stable), Node.js 20+, `webkit2gtk-4.1`, `gtk3`,
`libsoup3`, `openssl`, `libsecret`, `libgit2`.

```sh
npm ci
npm run tauri dev      # development window
npm run tauri build    # AppImage in src-tauri/target/release/bundle/
```

## First start

The onboarding wizard appears on first launch:

1. **Connect GitHub** — paste a personal access token with `repo` scope
   (create one at <https://github.com/settings/tokens>). The token is stored in
   the OS keyring. GitHub's device flow is wired up but needs a client id in
   `src-tauri/src/github.rs`, so the token path is the supported one.
2. **Repository** — GitPad looks for a private repository named `gitpad_notes`
   on your account and creates it if missing, then clones it into the app data
   directory.
3. Done. Files created from the tree are committed and pushed automatically.

You can skip the wizard and clone an existing repository from the Git panel
instead.

## Where things are stored

| Path | Contents |
|---|---|
| `~/.local/share/com.gitpad.app/repos/gitpad_notes/` | Your notes (a normal Git clone) |
| `~/.local/share/com.gitpad.app/config.json` | Theme, font, sync timings, extensions |
| OS keyring (Secret Service) | GitHub token |

Deleting `config.json` resets the settings; the repository is untouched.

## Shortcuts

See [SHORTCUTS.md](SHORTCUTS.md). The main ones: `Ctrl+P` for the command
palette, `Ctrl+Shift+P` for the markdown preview, right-click in the file tree
to create, rename or delete.

## Limits worth knowing

- A single repository at a time; switching means removing the clone by hand.
- Changes made to files outside GitPad appear after the window regains focus,
  but an already open tab is not reloaded.
- The markdown preview does not render remote images (CSP blocks them).
- The file tree watch is focus-based, not inotify-based.

## Testing

`npm test` runs the frontend unit tests (vitest), `cargo test` the Rust ones.

### Closing with unsaved changes (manual)

The save-failure path cannot be automated because it needs the OS dialog:

1. `chmod -R a-w ~/.local/share/com.gitpad.app/repos/gitpad_notes`
2. Edit a file in GitPad and close the window right away.
3. A dialog appears: "Some changes could not be saved. Exit anyway?"
4. Cancel keeps the app open with the edit still in the tab; Confirm exits.
5. `chmod -R u+w ~/.local/share/com.gitpad.app/repos/gitpad_notes` to restore.

The window force-closes after 5 s only if the webview stops answering
(crash or hang). That watchdog is not active while the dialog is open.
