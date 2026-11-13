# Manual checks

The automated suite is `npm test`, `npm run check` and
`cargo test --manifest-path src-tauri/Cargo.toml`. Two behaviours are not
covered by it and are checked by hand before a release.

## Close path

To exercise the unsaved-changes dialog, make the notes repository read-only:

```sh
chmod -R a-w ~/Documents/gitpad_notes
```

Edit a file and close the window. GitPad should show
**“Some changes could not be saved. Exit anyway?”**. Cancel, restore write
permissions, then repeat and confirm:

```sh
chmod -R u+w ~/Documents/gitpad_notes
```

A 5-second force-close watchdog only applies if the webview hangs; it is not
active while the dialog is open.

If the wizard put the repository somewhere else, use that path.

## PDF and office viewers

The PDF and office viewers run inside WebKitGTK, where pdf.js and mammoth are
not covered by the unit tests. With a display:

1. `npm run tauri dev`
2. Open a `.pdf`: pages render, scrolling loads the next pages, no blank canvas.
3. Open a `.docx` and an `.odt`: text and headings render, images do not break the layout.
4. Toggle the Markdown preview on a `.md` file (`Ctrl+Shift+P`).

## Windows smoke test

The Windows shell has no unit-test coverage. Run this on a real Windows 10
(1803 or newer) or Windows 11 x64 machine, using the installer from the CI
artifact, before tagging a release.

1. Install: no administrator prompt (per-user install), then launch from the
   Start menu. SmartScreen may say "Windows protected your PC": **More info →
   Run anyway**.
2. Title bar: drag moves the window, double-click maximizes, the maximize
   button restores, Win+Left/Right snaps it, the close button asks to save
   pending edits.
3. Onboarding: sign in with a PAT, restart: still signed in (Credential Manager
   holds the `com.gitpad.app` / `github_token` entry). Sign out, then sign in
   with the device flow: the browser opens.
4. Clone a repository, then open a local folder: both show in the tree and the
   status bar shows the branch.
5. Open a note, type, wait 2 s: `git log -1` shows `gitpad: update ...`, and a
   push from the Git panel succeeds.
6. `Ctrl+P`, `Ctrl+S` and `Ctrl+Shift+F` work. `F5`, `Ctrl+F` and `Ctrl+P` must
   not reload the page, open WebView2's find bar or print; right-click shows
   GitPad's menu, not Edge's. If Edge's menu appears, the log carries
   `webview settings:` from the failed COM call.
7. Markdown preview renders images and a table; a `.pdf` and a `.docx` open
   read-only.
8. Settings lists the installed font families; changing one applies at once.
9. Launch GitPad again: the open window is focused, no second window appears.
10. Right-click a file → reveal: Explorer selects it.
11. Edit an open file in Notepad: the tab reloads (or turns stale when it has
    unsaved edits) and the tree refreshes.
12. Pull a change made elsewhere: the open tab reloads, and a click in the Git
    panel does not open a duplicate tab.
13. Move the repository to another drive from Settings → Sync → *Move
    repository*; the dialog shows the final path, and the app keeps working.
14. Use a repository whose path has spaces and non-ASCII characters.
15. Create `CON.txt`, `a<b.md` and `note.`: each is refused with a clear error.
16. Open a folder that holds a nested repository (a clone inside Documents):
    saves still commit, and the nested folder is not listed as a change.
17. Close the window with a pending edit: the change is committed and pushed
    before the process exits.
