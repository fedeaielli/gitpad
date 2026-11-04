# Changelog

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
