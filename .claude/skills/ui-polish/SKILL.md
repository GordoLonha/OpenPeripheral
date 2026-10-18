---
name: ui-polish
description: Design system and workflow for OpenPeripheral's desktop UI (Tauri + React, app/src). Use before changing or adding any screen, panel, form control or CSS in app/src, when the user says the interface looks ugly, cramped or inconsistent, or asks to polish, redesign or "melhorar a interface".
---

# OpenPeripheral UI

Dark, calm, precise: the devices' own lighting is the colour on screen, the chrome stays
quiet. Interface text is Brazilian Portuguese, short and concrete ("Taxa de polling",
"Clique esquerdo"), sentence case, no exclamation marks.

## See it before and after
1. `npm --prefix app run dev` and open http://localhost:5173 (or `preview_start`
   `ui-preview`). In a plain browser `src/devmock.ts` answers every backend command with
   sample data (devices connected, a macro on the M65 back button), so every page renders
   without hardware. Keep the mock in step when you add a command or a field.
2. Screenshot each page you touch at **1280×780** (default window) and **980×640**
   (minimum window, `minWidth`/`minHeight` in tauri.conf.json). Nothing may scroll
   sideways, clip, or wrap a short label onto two lines.
3. `npx tsc -b` and `npm run lint` (no new warnings) before handing back.

## Tokens (index.css `:root`) — never hard-code these values
- Surfaces: `--bg` window, `--panel` cards and sidebar, `--panel-2` inputs, list rows,
  pressed states, `--panel-3` hover. Lines: `--line`, `--line-strong` (hover, focus edges).
- Text: `--text`, `--muted` (descriptions, units, hints), `--faint` (disabled, numbering).
- Accent `--accent` (cyan) with `--accent-ink` text on it and `--accent-soft` for active
  backgrounds. Status: `--ok`, `--warn`, `--danger`. Accent marks the one thing that is
  selected or on — never decoration.
- Spacing: `--s1` 4, `--s2` 8, `--s3` 12, `--s4` 16, `--s5` 24, `--s6` 32 px.
- Radius: `--r-sm` 8 (inputs, chips, rows), `--r` 12 (cards), `--r-lg` 16 (stages).

## Type
| Use | Style |
|---|---|
| Page title `h1` | 24px / 600 |
| Page subtitle `.page-sub` | 13px muted, one line |
| Card title `.card-title` | 15px / 600, text colour |
| Group label `.eyebrow` | 11px uppercase, 0.08em, muted — only for groups inside a card |
| Body | 14px; descriptions 13px muted |
| Numbers | `font-variant-numeric: tabular-nums` |

## Layout
- `.main` content is centred with `max-width: var(--content)` (1240px); every page uses
  the full content width — no narrow one-off `max-width`.
- Page header `.page-head`: title + subtitle left; page tabs or actions right.
- Navigation inside a page: `.tabs` (underlined). Choosing a value: `.seg` (segmented).
  Never use `.seg` to switch pages.
- Group related settings in one `.card` with `.list` rows (`.list-row`: text left,
  control right, 1px divider between rows) instead of one card per setting.
- Grids collapse to fewer columns below 1150px; check 980px.

## Components (App.css / controls.tsx)
- `.card` (+ `.card-head` with `.card-title` and optional actions on the right).
- `Switch` for on/off, `.seg` for 2–5 exclusive values, `select` (`.input`) for longer
  lists, `.chip` for effect/preset pickers, `.tile` for assignment types.
- `ColorField` / `.color-dot`: round 28px swatch with a 2px ring, hex in muted mono.
- Buttons: `.primary` (one per area), `.ghost`, `.icon-btn`; destructive actions
  `.ghost.danger`.
- Inputs: `.input` for text, number and select — same height (34px), same radius.
- Lists of items to pick (assignments, buttons, profiles): `.pick-list` rows; the selected
  row gets `--accent-soft` background and an accent edge.
- Empty states: one muted sentence saying what to do next.

## Checklist before handing back
- Same spacing between cards everywhere (`--s4`), same padding inside cards (`--s5`).
- One visual weight per level: page title > card title > labels > descriptions.
- Every control has a visible label or `aria-label`; focus ring visible (`:focus-visible`).
- Disabled and empty states look intentional.
- Screenshots at both sizes attached to the report.
