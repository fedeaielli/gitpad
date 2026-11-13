# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

<!-- Tauri 2 desktop shell (Linux and Windows), Monaco editor inside a webview; web is the design language. -->

## Users

Primary: a single technical author who writes Markdown notes, edits config files, and does light coding on Linux (Arch-first, Wayland/X11) or Windows 10/11. Works keyboard-first, offline, and already uses Git and GitHub. Distributed to other Linux users via AUR/AppImage and to Windows users via an NSIS installer, so the interface must be legible to a stranger with the same habits, not only to its author.

## Product Purpose

A local-first desktop editor for Markdown notes and small text files that live in a Git repository the user owns. It pairs Monaco with a focused GitHub workflow: edit, save automatically, commit locally, push when the user pauses. Success is writing and editing without thinking about sync, while every note remains an ordinary file the user can inspect, back up, and read without GitPad.

## Positioning

Saving is a git commit. No proprietary note database, no sync service, no account beyond GitHub. One user, one repository, a small surface area — the opposite of a workspace platform.

## Operating Context

Linux or Windows desktop, often in long writing or config-editing sessions; keyboard shortcuts and a command palette carry most navigation. The repository is the document store, so Git state (clean, ahead, behind, conflict) is part of the editing surface. Work continues offline; sync happens on save, idle, blur, and close.

## Capabilities and Constraints

Monaco editor with tabs, find/replace, slash commands, and self-formatting Markdown tables; Markdown preview with scroll sync; file tree with create/rename/delete/import; command palette with file search and full-text search; GitHub device-flow or PAT auth; automatic commit and push; conflict diff; commit history; settings (theme, font, sync timings, keybindings, language mappings); session logs; credentials in the OS keyring.

Constraints: Tauri 2 + Rust backend + Svelte 5 + TypeScript; no business-logic changes in the UI refactor; three themes must keep working (dark default, light, high-contrast); user-selectable accent colour; custom titlebar and window controls; Monaco must keep editing correctly. No integrated terminal, no plugin system, no branching UI in v1.

## Brand Commitments

Name: GitPad. Existing mark at `docs/icon.png` and `/logo.png`; GPL-3.0. Dark is the default surface. The layout takes VS Code's patterns as a starting point, but the visual language must be GitPad's own, not a VS Code replica. Linux-native, plain-file ethos, low ceremony.

Pinned visual direction (from the user): VS Code Dark+ familiarity, made warmer and softer — generous but controlled corner radii, subtle elevation, clearly separated surfaces, and slightly airier chrome. Not a literal VS Code copy; the identity lives in material, type, and restraint rather than a new layout.

## Evidence on Hand

Real screenshots at `docs/screenshot.png` and `docs/social-preview.png`; product copy and feature list in `README.md`; specification in `PROJECT.md`; shortcut reference in `SHORTCUTS.md`. The screenshot carries real note content (release notes, tables, code). No testimonials, benchmarks, or user counts exist and none may be invented.

## Product Principles

1. The file is the truth: the interface never hides that notes are plain files in a Git repository.
2. Keyboard first; every frequent action has a shortcut and a visible, searchable command.
3. Low ceremony: editing, saving, and syncing should feel like one act, not three workflows.
4. Trust is visible: sync state, conflicts, and history are legible at a glance, never inferred.
5. Small surface, high density: few features, well made, in a compact workspace that disappears into the task.

## Accessibility & Inclusion

Keyboard-only operation must remain complete. Visible focus rings, contrast that survives the dark, light, and high-contrast themes, reduced-motion support, and a high-contrast mode are existing commitments.
