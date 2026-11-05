# Changelog

## 1.0.3 — 2026-11-05

Sign-in without a keyring, a reworked history, and a safer editor.

### Added
- The GitHub token now survives restarts without an OS keyring: it is kept in
  `auth.token` (mode 0600) under the app data directory, and removed on logout
  or when the keyring accepts it again.
- **Restore removed folders** from History: a commit that only deletes files
  gets a "Restore N deleted items" action.
- History is grouped by day (Today/Yesterday/date), commits are cards coloured
  by action, and the expander has its own rail.
- The editor and the file history are separate, resizable frames with a centred
  drag grip.
- Confirmation dialogs are in-app and centred (restore, revert, discard, exit).
- Folders-first is independent of the sort order.
- Notifications have warning/error/info severity colours.

### Fixed
- Discarding a conflicted file no longer deletes it; discard refuses and points
  to the conflict dialog. Staged deletions are restored from HEAD.
- A slow file-history request can no longer overwrite the current tab or the
  newly expanded commit.
- Global shortcuts no longer fire behind the confirmation dialog, and Escape no
  longer closes the history behind it.
- The history panel is clamped to the editor column when the window shrinks.
- `dompurify` is forced to 3.4.17 for Monaco, closing a critical advisory.

## 1.0.2 — 2026-11-05

The Arch package started with a blank window asking for a dev server.

### Fixed
- The `PKGBUILD` now builds with the `tauri/custom-protocol` feature. Without it
  the installed binary is treated as a development build and loads
  `http://localhost:1420`, which is why 1.0.0 and 1.0.1 from `makepkg` showed
  `Could not connect to localhost: Connection refused`. The AppImage was never
  affected: the Tauri CLI sets that feature itself.
- CI compiles that same packaging build on every push, so the two build paths
  cannot drift apart again.

## 1.0.1 — 2026-11-05

Signing in on a machine whose keyring is unavailable, and opening links from a
minimal desktop.

### Fixed
- **GitHub login no longer fails without a keyring.** When the Secret Service is
  missing, locked, or unreachable, the token is kept for the current session
  instead of aborting the device flow, and the app says so with a toast at
  startup and a note under the login form.
- **The session bus is found again.** If `DBUS_SESSION_BUS_ADDRESS` is not set,
  GitPad falls back to `$XDG_RUNTIME_DIR/bus`, so a launcher that drops the
  environment no longer breaks the keyring.
- **Links open on a bare desktop.** "Create a token", the device-flow page and
  profile links try `xdg-open`, then `gio open`, and copy the URL with a notice
  when neither works. Every attempt is written to the log.
- Failing to open the browser no longer cancels the device flow.

## 1.0.0 — 2026-11-04

First stable release. GitPad is a desktop editor for markdown notes kept in a
Git repository you own.

### Editor
- Monaco-based editor with automatic save after two seconds of inactivity.
- One editor instance shared by every tab, with per-file view state.
- Undo and redo arrows in the title bar, in addition to `Ctrl+Z` / `Ctrl+Y`.
- Find and replace: `Ctrl+F` and `Ctrl+R`, themed and no longer shifting the
  first line of the document.

### Git and sync
- Automatic commit on save (`gitpad: update {file} {ISO 8601}`), then push.
- Pull is fetch + fast-forward, falling back to a merge; never rebase, never
  force-push. Conflicts surface in a modal instead of silently failing.
- Flush on window blur and on shutdown, so the last edit is not lost.
- File history with a side-by-side diff, and a commit diff in the bottom panel
  reachable from the history list.

### Repositories
- Clone or create the notes repository from onboarding.
- Adopt any local folder as the active repository, initializing it when needed.
- Repositories without an `origin` remote commit locally and skip sync.
- Move and import repositories without losing the clone.

### GitHub
- Sign in with the OAuth device flow or a personal access token; the token
  lives in the OS keyring.
- Account menu in the activity bar: open the GitHub profile, or log out.

### Search and navigation
- Command palette (`Ctrl+P`) for files, `>` commands, and full-text search.
- Find in files (`Ctrl+Shift+F`), go to line (`Ctrl+G`) from the editor.
- Keyboard shortcuts dialog (`Ctrl+K`) and menu bar with File and Edit.

### Settings and logs
- Appearance, Sync, Editor, Extensions and Account tabs.
- Session logs in *Settings → Logs*: current and previous sessions, filterable
  by severity, category (editor, git, app) and free text. Rotated at 5 MB with
  the last five files kept.
- Startup and shutdown, every commit, pull and push outcome, conflicts,
  authentication, GitHub calls, file operations and UI errors are logged — never
  tokens, file contents or request bodies.

### Notes on this release
- Linux only (AppImage and Arch package).
- The logout button clears the token from the keyring; it does not revoke it on
  GitHub, which the device flow cannot do without a client secret.
