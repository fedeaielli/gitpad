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
