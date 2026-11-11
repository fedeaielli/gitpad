# Changelog

## 1.2.0 — 2026-11-11

The editor notices changes made outside it, the tree creates things where you
click, and the app moves.

### Added
- External changes: a debounced watch on the active repository reloads a tab
  when its file changes on disk. Dirty, busy and deleted tabs are handled
  instead of clobbered, autosave's own write is ignored, and the watch follows
  you to another repository.
- Create files and folders from the palette; the context menu prefills the
  folder you right-clicked.
- Entrance motion for dialogs, menus, toasts, folder expansion and the empty
  state. The onboarding wizard cascades its steps, pops the success check and
  fades out on close.
- Remote images render in the Markdown preview (the CSP allows them).

### Changed
- Tabs share the strip equally; file tree rows are inset from the panel edge.

### Fixed
- A caret moved by a command (`Enter` on a list, `Tab` across table cells, the
  code-block action) scrolls into view, with air above and below it.
- Autosave no longer marks a tab stale or echoes back as an external change.
- The onboarding token field draws its focus ring around the whole control;
  the wizard overlay is a window drag region.

## 1.0.6 — 2026-11-07

Visual pass: the "Warm Graphite" palette, rounded surfaces, Hanken Grotesk as
the UI font, and matching scrollbars and file icons.

### Added
- File icons from the vscode-icons set (vscode-icons-js + vendored MIT SVGs),
  replacing the five hand-drawn glyphs.
- Thin scrollbars (8px) in the theme's own colours: light grey on dark,
  dark grey on light, cyan on high contrast; always visible, brightening on
  hover. Editor scrollbars keep step.
- Drag handles on every splitter: a 4px bar inside the gap, transparent at
  rest, border-strong under the pointer, accent while dragging. The timeline
  height handle keeps a visible pill.
- History and File Timeline: hash + date on one line, message on the next;
  Restore / Revert reach the 32px desktop target.
- Settings: controls capped at 280px, tab content on surface-0 under a
  surface-1 dialog, the dark-only override block removed (tokens invert by
  themselves).

### Changed
- Warm Graphite surfaces, borders, text and muted tones across all three
  themes (dark, light, hc-black).
- Radii scale: 6 / 9 / 14 / 18, panel gutters 8px, tab bar 40px, status bar
  26px, title bar 38px.
- Type scale: 10 / 11 / 12.5 / 14 / 16 / 20 px via CSS variables, all 71
  hardcoded sizes replaced.
- UI font: Hanken Grotesk (variable 400–700, latin subset) bundled and
  preloaded, system-ui fallback.
- Monaco themes aligned to the palette (dark, light; hc-black unchanged).
- Markdown preview warmed (headings, tables, blockquotes, links, inline code)
  via scoped overrides; syntax palette untouched.
- Dialogs and overlays use the 18px overlay radius with the two-level
  shadow; menus and toasts use the first level.
- Active row in the file tree, palette and tabs marked by an inset accent
  bar (2px) instead of a background fill.

### Fixed
- The keyring probe runs off the main thread; the onboarding no longer
  waits for it before polling.
- Light theme sidebar surface now one step darker than the editor so the
  two panels read as separate layers.
- Command palette list retriggers after backspace; `/h1`..`/h6` accept a
  digit after the hash; `/sublist` and `/subnumber` work.
- File timeline restore buttons no longer flicker between half and full
  opacity; they are always fully visible.
- Settings hover tab no longer looks identical to the active tab.

## 1.1.0 — 2026-11-09

Tabs that read like the editor they open, and Markdown that types itself.

### Added
- The `/` menu in Markdown notes: type `/` at the start of a line to insert a
  heading, emphasis, a code block with its language, a table, a task list and
  the rest of GitHub-flavoured Markdown, nested lists included. With text
  selected the command wraps it instead of inserting a placeholder.
- Markdown tables edit themselves: `Tab` and `Shift+Tab` walk the cells and add
  a row at the end, and `/row`, `/col`, `/delrow`, `/delcol` inside a cell
  change the shape. Each change re-renders the table with its columns aligned.

### Changed
- The tab bar follows VS Code: the open tab merges into the editor through
  concave shoulders, the strip shares the side panel surface, and the toolbar
  holds icons only.
- The `/` menu wears the same card treatment as the rest of the app.
- Scrollbars are warm taupe with rounded sliders, in both themes.

### Fixed
- The sidebar resize gutter no longer covers the sidebar scrollbar.
- `Enter` continues a Markdown list and clears an empty item.
- `/sublist` and `/subnumber` indent three spaces, so the preview nests them
  instead of flattening the list.

## 1.0.5 — 2026-11-06

Browser sign-in shows its code again.

### Fixed
- "Login with GitHub" no longer freezes the window. Opening the browser was
  waiting on the main thread, so the device code was never painted and the
  polling never started; the opener now runs off that thread, and the code and
  the polling start before the browser is up.

## 1.0.4 — 2026-11-06

Notes land next to your documents, and the editor chrome got a pass.

### Added
- New repositories are cloned next to your documents
  (`~/Documents/gitpad_notes`) instead of a hidden app-data folder. The wizard
  shows the path before it starts and still lets you pick another folder.
- The command palette splits a path into the folder and the file name, so the
  name keeps its weight and only the leading folders truncate.
- The setup wizard is one card per step, and a missing repository is an
  ordinary state instead of an error.

### Fixed
- Restoring or reverting from the sidebar History no longer closes the open
  tab: the editor re-reads the file in place.
- The undo and redo arrows follow the undo stack and stay disabled when there
  is nothing to undo or redo.
- The disabled Restore in the file timeline no longer lights up on hover, and
  disabled arrows read as disabled.
- Settings controls share one height (the inputs were two pixels taller than
  the select next to them), and the selected tab no longer looks like a
  hovered one.

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
- The Arch/source package now compiles the device-flow client id, so both
  sign-in paths work outside the AppImage.

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
