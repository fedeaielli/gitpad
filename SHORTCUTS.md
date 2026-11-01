# GitPad shortcuts

GitPad saves automatically (debounced, default 2 s) and pushes automatically
(default 30 s after you stop typing). `Ctrl+S` skips both waits: it saves every
open file now and pushes immediately. Autosave keeps running as before.

## Keyboard

| Shortcut | Action |
|---|---|
| `Ctrl+S` | Save every open file now, then push |
| `Ctrl+P` | Open the command palette (files, `>` commands, `>search`) |
| `Ctrl+Shift+P` | Cycle the markdown preview: off → split → full (markdown tabs only) |
| `↑` / `↓` | Move the palette selection |
| `Enter` | Open the selected file, run the selected command, or submit a prompt |
| `Esc` | Close the palette |

## Mouse

| Action | Result |
|---|---|
| Right-click the file tree | New file, new folder, rename or delete |
| Middle-click a tab | Close the tab |
| Click the `⚙` in the status bar | Open settings |
| Click the conflict badge | Reopen the merge conflict dialog |

## Command palette commands

Type `>` first to list them:

- `Save and sync` — same as `Ctrl+S`
- `Sync` — push the current notes now
- `Toggle markdown preview` — same as `Ctrl+Shift+P`
- `Theme: cycle` — rotate between the available editor themes
- `New file` / `New folder` — create an entry by path
- `Search in files` — global search across the repository
- `Open settings` — same as the `⚙` button
