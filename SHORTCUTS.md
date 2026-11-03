# GitPad shortcuts

GitPad saves automatically (debounced, default 2 s) and pushes automatically
(default 30 s after you stop typing). `Ctrl+S` skips both waits: it saves every
open file now and pushes immediately. Autosave keeps running as before.

## Keyboard

| Shortcut | Action |
|---|---|
| `Ctrl+S` | Save every open file now, then push |
| `Ctrl+N` | New file: type a name like `notes.md` |
| `Ctrl+O` | Import: opens the palette on `Import file…` / `Import folder…` |
| `Ctrl+P` | Toggle the command palette (files, `>` commands, `>search`) |
| `Ctrl+Shift+P` | Cycle the markdown preview: off → split → full (markdown tabs only) |
| `Ctrl+Shift+F` | Find in files: opens the palette on full-text search |
| `Ctrl+F` | Open or close find in the current file |
| `Ctrl+R` | Open or close find and replace in the current file |
| `Ctrl+G` | Go to line (while the editor is focused) |
| `Ctrl+/` | Toggle a line comment |
| `Shift+Alt+A` | Toggle a block comment |
| `Ctrl+H` | Open or close the file history |
| `Ctrl+W` | Close the current tab |
| `Ctrl+K` | Show or hide the keyboard shortcut list |
| `Esc` | Close the palette, the file history, or a dialog |

In the palette:

| Key | Action |
|---|---|
| `↑` / `↓` | Move the selection |
| `Enter` | Open the selected file, run the selected command, or submit a prompt |
| `Esc` | Close the palette |

In Vim or Emacs keybindings, `Ctrl+N` and `Ctrl+O` stay with the editor while it
has focus (completion / jump back in Vim, next line / open line in Emacs). Use
`>` → `New file` or `Import file…` from the palette there, or press the shortcut
with the focus outside the editor.

Imports from `Ctrl+O` land in the repository root. To import into a subfolder,
right-click that folder in the tree and use **Import file** / **Import folder**.

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
- `Import file…` / `Import folder…` — copy into the repository root; same names
  are renamed to `name (1).ext`, nothing is overwritten
- `Search in files` — global search across the repository
- `File history` — same as `Ctrl+H`
- `Keyboard shortcuts` — same as `Ctrl+K`
- `Open settings` — same as the `⚙` button
