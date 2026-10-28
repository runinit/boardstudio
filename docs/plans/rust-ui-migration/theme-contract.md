# Theme contract: Themes, Appearance and tokens

Ticket: [005](issues/005-theme-structure.md). Decided with the user, 2026-10-21 (all seven proposals accepted as drafted).
Terms are in the [glossary](../../../GLOSSARY.md): a **Theme** is Original or shadcn; **Appearance** is light, dark or system.
Inputs: [ADR-0039](../../adr/0039-adopt-rust-ui-components-with-two-themes.md), the rust-ui token list in
`research/rust-ui-catalogue`, and today's tokens in `web/assets/m1.css` and `web/tailwind.css`.

## 1. How today's mechanism works

- Appearance preference (`system`, `light`, `dark`) is stored in localStorage `boardstudio:v2:theme` and resolved to `light` or
  `dark` (system follows `prefers-color-scheme` and listens for changes).
- The resolved value is written as `data-theme="light|dark"` on `<html>`. CSS has a `:root` block and a
  `:root[data-theme='dark']` block of `--wb-*` variables; `tailwind.css` aliases them into Tailwind colour names.
- The 2D canvases read CSS tokens (39 uses of `--wb-geometry-*`, `--wb-grid-*`, `--wb-keymap-*`). The **3D renderer does not**:
  it receives the string `"dark"` or `"light"` and switches hard-coded palettes (`renderer/src/wasm.rs`).

Two consequences: the attribute `data-theme` currently holds *Appearance*, clashing with the glossary; and a Theme cannot reach
the 3D views through CSS.

## 2. Structure

| Piece | Design |
| --- | --- |
| Attributes on `<html>` | `data-appearance="light|dark"` (resolved) and `data-theme="original|shadcn"`. Copied `dark:` classes are keyed with one line, `@custom-variant dark (&:where([data-appearance=dark], [data-appearance=dark] *))`, so no `.dark` class is needed. |
| Token contract | rust-ui's names are the base; BoardStudio adds the short extras in section 4; `--wb-*` retires. |
| What a Theme sets | colours, `--radius`, fonts, shadows and the icon stroke width (`--icon-stroke`). Never layout metrics (section 5). |
| Blocks | one `[data-theme=original]` block and one `[data-theme=shadcn]` block, each with light values and a `[data-appearance=dark]` override |
| Preference storage | Appearance: `boardstudio:v2:appearance` (reads the legacy `boardstudio:v2:theme` once). Theme: `boardstudio:v2:workbench-theme`, default `original`. Browser preferences, not in the project file. |
| 3D renderer | receives Appearance only; its palettes are the same in both Themes. |
| Header control | one button with a Dropdown Menu holding two radio groups, Appearance and Theme ([ADR-0040](../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md)) |

## 3. Core token mapping (Original Theme, light / dark)

Our names were chosen differently: our `accent` is the brand colour and our `muted` is muted *text*, whereas rust-ui's `accent` is a
hover surface and `muted` a quiet surface. Mapping by meaning, not by name:

| rust-ui token | From today's token | Light | Dark |
| --- | --- | --- | --- |
| `background` (the "desk" the cards sit on) / `foreground` | `canvas` / `ink` | #E9EEF2 / #182331 | #181B21 / #EDF1F7 |
| `card`, `popover` (+ `-foreground`) | `panel` / `ink` | #FFFFFF / #182331 | #242831 / #EDF1F7 |
| `primary` / `primary-foreground` | `accent` / `on-accent` | #3858D6 / #FFFFFF | #9AADFF / #101725 |
| `primary-dark` / `primary-light` | `accent-dark` / `selection-surface` | #2945b6 / #E9EDFF | #b6c4ff / #303B56 |
| `secondary` | `surface-soft` | #edf0f8 | #2C323D |
| `muted` / `muted-foreground` | `field` / `muted` | #f7f8fa / #576678 | #20242D / #AFBCCB |
| `accent` / `accent-foreground` (hover surface) | `raised` / `ink` | #e9eef2 / #182331 | #373E4B / #EDF1F7 |
| `border` / `input` | `rule` / `rule-control` | #e2e7ed / #7D8B9A | #30343B / #626D7B |
| `ring` | `accent` | #3858D6 | #9AADFF |
| `destructive` (+ `-light`, `-dark`) | `status-error` and `-surface` | #a2191f / #fff1f1 | #ff8389 / #491217 |
| `success`, `warning` `-light` / `-dark` | `status-*-surface` / `status-*` | e.g. #e2f2e9 / #226c50 | e.g. #18352d / #6ab99d |
| `selection` / `selection-foreground` (text highlight only, rust-ui's meaning) | `selection-surface` / `selection-ink` | #E9EDFF / #253EAC | #303B56 / #D5DDFF |
| `sidenav-*` | aliases of `card`, `foreground`, `primary`, `accent`, `border`, `ring` | | |

**New values to author** (Graphite has none): the saturated fills for `success`, `warning`, `info` and their foregrounds, the whole
`info` set, `destructive-foreground`, `slider-track`, `scrollbar-thumb` / `-track`, `surface`. `chart-*` are not needed (Charts are
not adopted). `--radius` is 10px for Original (rust-ui's `md` = 8px matches today's 8px controls; toolbars and pills lose 2px).
Fonts: Original keeps Source Sans 3 and Source Code Pro.

## 4. BoardStudio extras (no rust-ui equivalent)

| Token | Today's | Why it stays |
| --- | --- | --- |
| `canvas` | `canvas` | the inset drawing surface inside the canvas card (same value as `background` in Original) |
| `selected`, `selected-foreground` | `selection-surface`, `selection-ink` | the selected object, row or key. Original: #E9EDFF / #253EAC light, #303B56 / #D5DDFF dark; shadcn: `accent` / `accent-foreground`. rust-ui's `selection` is saturated orange in the shadcn palette and rendered the selected row loud orange in the prototype |
| `field` | `field` | fill of inputs and select triggers. Original: #f7f8fa light, #20242D dark; shadcn: transparent (5% white in dark), as rust-ui ships |
| `border-strong` | `rule-strong` | stronger rule used by tables and focused groups |
| `toolbar`, `toolbar-border` | `toolbar`, `toolbar-rule` | the tinted canvas toolbars |
| `geometry-outline`, `-fill`, `-fill-muted`, `-key`, `-part`, `-label`, `-body`, `-board`, `-graphic`, `-zone`, `-text` | `geometry-*` | 2D drawing colours |
| `grid-small`, `grid-large` | `grid-*` | canvas grid |
| `keycap-face`, `-ink`, `-rule` | `keymap-key-*` | keymap key faces |

Dropped as redundant: `accent-secondary` (same value as `geometry-part`), `teal` (alias), `topbar`, `topbar-ink`, `subtle`
(same as `muted`), `display` font.

## 5. Metrics shared by both Themes

Control height 40px, compact 36px, touch 44px; header 56px ([ADR-0040](../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md)),
footer, dock widths, spacing scale, type scale, motion durations. They lose the `wb-` prefix and are **not** Theme-dependent: a
Theme that changed control heights would move every toolbar and panel. Copied rust-ui sizes (`h-9`, `h-8`) are edited to read these
tokens.

## 6. shadcn Theme

Source: rust-ui's own `tailwind.css` neutral palette (oklch) at the pinned commit, verbatim, with its three gaps defined
(`destructive-foreground`, `surface`, `font-mono`). Font: Inter, bundled as a variable woff2 (OFL) so the offline app does not
fetch it.

## 7. Open build details

Whether to apply the preference before first paint (the app currently sets the attribute after load, so a flash is possible);
the exact `info` and chart-free extras; migrating the 80 `--wb-*` references; the look of both Themes is checked in the prototype
[013](issues/013-prototype-theme-look.md).

## 8. Cards style (decided from prototype 013)

Decided with the user, 2026-10-21, after [013](issues/013-prototype-theme-look.md): surfaces are **cards on a desk**, separated by fill and
elevation, not outlines ("the white outline around everything" was the one thing the user disliked).

- The page (`background`) is a darker desk; the header, Objects dock, canvas and Inspector are rounded cards (`card`, `rounded-xl`,
  soft shadow). The drawing surface is an inset rounded panel inside the canvas card. Dark mode relies on the lighter card fill, as shadows
  are invisible there.
- Controls are filled, not outlined: Secondary and Ghost buttons (the Outline variant is not used in the workbench), a filled Toggle Group
  and tool groups, filled input fields, tinted Alerts, soft inner cards for Inspector sections.
- **The built-in borders of the copied rust-ui components are removed in the copies** (Card, Alert, Input Group, Select trigger, Toggle Group,
  Outline Button), including their `dark:`-prefixed classes, which a plain override does not replace. Focus rings stay.
- The header is a **56px borderless card** (the prototype used 48px; ADR-0040's height is kept).
- Prototype: `.scratch/theme-proto`, branch `prototype/theme-look`.
