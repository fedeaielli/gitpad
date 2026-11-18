# Design

The visual contract for GitPad. Settings is the reference surface: every other
screen borrows its controls, surfaces and rhythm. Tokens live in
`src/app.css`; this file explains how to use them.

## Surfaces

| Token | Use |
|---|---|
| `--surface-base` | window floor behind floating panels |
| `--surface-0` | page and dialog body; inputs |
| `--surface-1` | panels, cards, dialog shell, selected tint base |
| `--surface-2` | controls at rest, hover rows |
| `--surface-3` | popovers, menus, hover on a control |

- Dialog: `--scrim` backdrop, shell `--surface-1`, body `--surface-0`,
  `--r-overlay`, `--shadow-2`.
- Card or section: `--surface-1`, 1px `--border`, `--r-lg`.
- Popover or menu: `--surface-3`, 1px `--border`, `--r-md`, `--shadow-1`.
- Never stack a `--surface-3` card on another surface-3; nested wells use
  `--surface-0` inside `--surface-1`.

## Controls

- Form controls (button, input, select) are 32 px tall, `--r-md`, padding
  `0 var(--sp-3)` for buttons and `0 var(--sp-2)` for inputs.
- Buttons at rest: `--surface-2` fill, `--border-strong` border.
- Inputs: `--surface-0` fill, `--border-strong` border.
- Icon buttons: 24 px in chrome, 28 px (`.icon-btn.lg`) in dialogs,
  `--r-sm`/`--r-md`, no border.
- Editor chrome keeps its compact rows (22-26 px, tabs 32 px, status 26 px);
  32 px belongs to forms, dialogs, cards and the onboarding wizard.

### Button hierarchy

| Class | Meaning |
|---|---|
| `.btn` | secondary action |
| `.btn-primary` | the one filled action on the screen |
| `.btn-danger` | destructive action, `--err` border |
| `.btn-ghost` | quiet action or close control |
| `.text-btn` | inline text action inside a row |
| `.icon-btn` | square icon-only control |

At most one `.btn-primary` per screen. No green "recommended" text: emphasis
comes from the filled accent. A destructive confirm keeps `--err`.

## Type

- `--fs-lg` 16 px: dialog and onboarding body, settings rows.
- `--fs-md` 14 px: panel content, file and commit rows.
- `--fs-sm` 12.5 px: secondary text, compact chrome labels, paths.
- Controls inherit the size of their surface; do not set a font size on a
  button or input unless the surface demands it.
- `--fs-xs` 11 px: meta, hints, uppercase group labels.
- `--fs-2xs` 10 px: badges and counts.

No literal font sizes in components. Group labels use `--accent-fg`,
uppercase, `letter-spacing: 0.08em`, weight 500.

## Spacing and shape

- Horizontal and vertical rhythm uses `--sp-*` only. No raw 3/5/6/10/14/24 px.
- Radii: `--r-sm` rows and chips, `--r-md` controls and inner cards,
  `--r-lg` cards and sections, `--r-overlay` modals. Never a literal radius.
- Elevation: `--shadow-1` controls and popovers, `--shadow-2` modals.

## Selection and emphasis

- Persistent selection (file, tab, panel entry, settings section) is a tint:
  `color-mix(in srgb, var(--accent) 12%, var(--surface-1))`, with the icon or
  label switching to `--accent-fg`.
- Full accent fill is transient: command palette cursor, menu row hover.
- No 2 px accent stripes on panel edges; the tint carries the state.
- Focus stays a 2 px `--accent-fg` ring with `outline-offset: -2px`.

## Motion

- Hover, press and colour changes: `--t-fast` (150 ms) with `--ease-out`.
- Panels, tints and layout shifts: `--t-base` (180 ms) with `--ease-out`.
- Enter animations that should settle like a physical object use
  `--t-spring` (420 ms) with `--ease-spring` (critically damped, no bounce).
  Bounce is reserved for momentum carried by a gesture; nothing here has one.
- Overlays enter with `gp-fade-in` + `gp-panel-in`; menus with `gp-menu-in`;
  popovers with `gp-pop-in`.
- One shared `.spin` for busy icons. Never a second keyframe for rotation.
- Press feedback is the global 1 px sink on `button`; do not add another.
- `prefers-reduced-motion: reduce` must collapse to a cross-fade; progress
  indicators stay animated.

## Themes

Dark (default), light (`vs`) and high contrast (`hc-black`) all read the same
tokens. The accent tint must stay visible on high contrast: if it does not,
keep the border instead of raising the tint. Colours are defined only in
`src/app.css`.

## Checks before shipping a UI change

1. `npm run check`, `npm test`, `npm run build`.
2. Look at the screen in all three themes at 720 px and at full width.
3. Keyboard through the screen: focus visible on every control.
4. Turn on reduced motion: no slides, no springs, content still appears.
