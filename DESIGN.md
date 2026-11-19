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
- Select popup: same recipe as a menu, because GTK draws the native list in its
  own colours. The closed field copies a form control: `--surface-0`,
  `--border-strong`, `--r-md`, 32 px. A field caps it with `--select-max`.
- Toast: `--surface-3`, 1px `--border`, `--r-lg`, `--shadow-1`, padding
  `--sp-3 --sp-4`. Cards are separate and stack collapsed at a `--sp-3` peek;
  hover or focus expands them, newest in front. Severity uses the matching icon
  colour (`--ok`, `--accent-fg`, `--warn`, `--err`) plus a 7% surface wash; the
  border stays neutral.
- Never nest a `--surface-3` card inside another surface-3; nested wells use
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
- Those six steps are declared in `rem`, so they follow the user's base font
  size; the px figures above are the size at the 16 px default.

No literal font sizes in components. Group labels use `--accent-fg`,
uppercase, `letter-spacing: 0.08em`, weight 500.

## Spacing and shape

- Horizontal and vertical rhythm uses `--sp-*` only. No raw 3/5/6/10/14/24 px.
- Radii: `--r-sm` rows and chips, `--r-md` controls and inner cards,
  `--r-lg` cards and sections, `--r-overlay` modals. Never a literal radius.
- Elevation: `--shadow-1` controls and popovers, `--shadow-2` modals.

## Frozen metrics

Two dense, scrollable surfaces keep their pre-refactor sizes because the
uniform 32 px control reads heavy there. These are the only naming exceptions
to the rules above.

- Command palette field: ~37 px tall, own padding (`--sp-2`/`--sp-25`) and
  `--surface-2` fill. The list body stays `--surface-0`.
- History cards: 13 px counts and empty title (0.8125rem), 12 px day label and
  empty hint (0.75rem), 3 px row gap, 28 px inline `Restore`, 26 px refresh,
  4 px badge radius, and the 1 px accent stripe on the active commit with
  `--surface-3` fills. Buttons keep the shared classes and only override the
  metrics they need.

## Selection and emphasis

- Persistent selection (file, tab, panel entry, settings section) is a tint:
  `color-mix(in srgb, var(--accent) 12%, var(--surface-1))`, with the icon or
  label switching to `--accent-fg`.
- Full accent fill is transient: command palette cursor, menu row hover.
- No 2 px accent stripes on panel edges; the tint carries the state. The
  active history commit keeps its 1 px inset stripe (see Frozen metrics).
- Focus stays a 2 px `--accent-fg` ring with `outline-offset: -2px`.

## Motion

- Hover, press and colour changes: `--t-fast` (150 ms) with `--ease-out`.
- Panel contents, tints and hover surfaces: `--t-base` (180 ms) with `--ease-out`.
- Structural geometry — sidebar width, timeline height, timeline split — tracks
  the pointer 1:1 while it is down and settles with `--t-spring` on release. It
  does not animate during the drag: Monaco relayouts on every frame anyway, and
  a transition there fights the pointer.
- Enter animations that should settle like a physical object use
  `--t-spring` (420 ms) with `--ease-spring` (critically damped, no bounce).
  Bounce is reserved for momentum carried by a gesture; nothing here has one.
- Overlays enter with `gp-fade-in` + `gp-panel-in`; menus with `gp-menu-in`;
  popovers with `gp-pop-in`.
- One shared `.spin` for busy icons. Never a second keyframe for rotation.
- Press feedback is the global 1 px sink on `button`; do not add another.
- `prefers-reduced-motion: reduce` collapses to an instant appearance: no slide,
  no spring, no draw. Progress indicators stay animated.

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
