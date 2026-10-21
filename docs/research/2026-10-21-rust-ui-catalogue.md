# rust-ui catalogue fact sheet

Answers [002: Fact sheet for rust-ui's component catalogue](../plans/rust-ui-migration/issues/002-rust-ui-catalogue-facts.md).

- Source: [rust-ui/ui](https://github.com/rust-ui/ui) `main` pinned at
  [`7120636e68556ddd023a4ec8e083663c7b745318`](https://github.com/rust-ui/ui/tree/7120636e68556ddd023a4ec8e083663c7b745318)
  (fetched 2026-10-21 with `gh api repos/rust-ui/ui/commits/main`). Every link below is pinned to that SHA.
- Method: read from the tarball at that SHA. A script pulled the mechanical columns
  (colour classes, `icons` imports, `crate::ui::*` and `crate::hooks::*` imports, enums,
  `web_sys`/`js_sys`/`eval`/inline-script use, ARIA attributes, key names). I wrote the purpose,
  behaviour, keyboard and flag columns after reading the source. I read the interactive
  components (button, dialog, select, tabs, switch, tooltip, slider, radio, popover, toast,
  charts, checkbox, accordion, collapsible, toolbar, toggle group, theme toggle) in full;
  the rest from their signatures, scripts and doc comments.
- Not run: nothing was compiled or rendered. Behaviour claims come from reading the source;
  [001](../plans/rust-ui-migration/issues/001-compile-rust-ui-against-our-build.md) should confirm them at runtime.

## Headline facts

- **86 components**, not 87: the [`ui/`](https://github.com/rust-ui/ui/tree/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui)
  directory has 87 files and one is `mod.rs`. Two of them (`date_picker_state`,
  `date_picker_dual_state`) are plain Rust state helpers with no UI. `data_table` only re-exports `table`.
- **Crates:** registry [`Cargo.toml`](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/Cargo.toml):
  `dioxus 0.7` (router, fullstack), `tw_merge`, `icons 0.18` (crates.io, features `dioxus`,
  `dioxus_animated`; names follow Lucide: `ChevronDown`, `LoaderCircle`, ...), `strum`, `serde`/`serde_json`,
  `time`, `gloo-timers`, `js-sys`, `wasm-bindgen`, `web-sys`, `heck`. The workspace
  [`Cargo.toml`](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/Cargo.toml) pins `dioxus = "=0.7.9"`.
  BoardStudio uses 0.7.10.
- **Behaviour model.** About 18 components (dialog, sheet, drawer, popover, select,
  multi_select, dropdown_menu, context_menu, menubar, command, hover_card, navigation_menu,
  carousel, header, sidenav, chips, input_otp, toast) contain JavaScript strings. The overlays
  get their open/close behaviour from an inline `script { dangerous_inner_html }` block, which finds nodes by generated id,
  polls with `setTimeout(…, 50)` until they exist, adds `document` listeners that are never
  removed, and flips `data-state`. These components are **uncontrolled**: they have no `open`
  prop and no `on_open_change`, so Rust cannot open or close them. Popover and hover card also
  need the browser to support the Popover API and CSS anchor positioning. The `window.ScrollLock`
  global the scripts call comes from the `use_scroll_lock` hook and is optional (the scripts check
  for it first).
- **Keyboard.** Command has the only full keyboard model: Up/Down/Enter/Esc, Ctrl/Cmd+K, and
  combobox ARIA. Every menu, select and overlay handles Escape (plus outside click) and nothing
  else: no arrow-key navigation, no focus trap, no focus return, no roving tabindex.
  Carousel has arrow keys; data_grid and workflow have their own shortcuts.

## Red flags

| Flag | Where |
|---|---|
| Dialog/Sheet/Drawer: no `role=dialog`, no `aria-modal`, no focus trap/return, cannot be opened from Rust | [dialog.rs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/dialog.rs), sheet, drawer, alert_dialog |
| Select: options are `li tabindex=0` with click handlers only, so Enter/Space likely do nothing; no arrow keys; `SelectValue` shows the raw value string, not the option label; no controlled `value` | [select.rs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/select.rs) |
| Tabs: no `tablist`/`tab`/`tabpanel` roles, no `aria-selected`, no arrow keys, no `value`/`on_change` | [tabs.rs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/tabs.rs) |
| Switch: internal state only; `checked` sets only the initial value and there is **no change callback** | [switch.rs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/switch.rs) |
| Tooltip: pure CSS `group-hover`; does not show on keyboard focus; no `role=tooltip` | [tooltip.rs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/tooltip.rs) |
| Toggle group: `role=radio` without `aria-checked`; no group value | [toggle_group.rs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/toggle_group.rs) |
| Not self-contained: Toast needs the site's `public/app_components/toast.js`; AreaChart needs ApexCharts and `chart_init.js`; InputOTP loads `/components/otp.js`, **which is not in the repo** | toast, charts, input_otp |
| `auto_form` is a stub (the Leptos derive macro was not ported) | [auto_form.rs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/auto_form.rs) |
| Hard-coded Tailwind palette instead of tokens | status (`bg-green-300`, ...), card_carousel (`bg-gray-200`), drag_and_drop (`bg-neutral-600`), dropzone (`bg-red-500`, `from-black`), mask (`from-white`), button Destructive (`text-white`) |
| Invalid ARIA role `radio-button-group` | radio_button_group |
| Token gaps: see the next section (`surface`, `destructive-foreground`, `sidenav-*` vs `sidebar-*`, `font-mono`) | callout, badge, dropzone, sidenav |
| Marketing or demo-only (not workbench material) | animate, marquee, mask, footer, header, navigation_menu, bottom_nav, card_carousel, drag_and_drop, workflow, plus chat UI: bubble, message, marker, attachment, input_prompt |

## Tokens (for the Theme ticket)

There are three sources, and they **disagree**:

1. The docs site's [`tailwind.css`](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/tailwind.css)
   (`:root` and `.dark`, oklch values, mapped through `@theme inline`). This is the set the components were written against.
2. What `ui init` writes into a consumer project:
   [`crates/ui-cli/src/command_init/template.rs`](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/crates/ui-cli/src/command_init/template.rs)
   and [`colors.rs`](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/crates/ui-cli/src/command_init/colors.rs).
   It covers only the shadcn core: 7 base palettes (Neutral, Stone, Zinc, Mauve, Olive, Mist, Taupe)
   × 18 accents. An accent overrides `primary`, `secondary`, `chart-1..5` and `sidebar-primary*`.
3. [`public/registry/themes/*.json`](https://github.com/rust-ui/ui/tree/7120636e68556ddd023a4ec8e083663c7b745318/public/registry/themes)
   (gray, neutral, slate, stone, zinc): the old shadcn v3 format with **HSL triplets**
   (`"0 0% 3.9%"`), core colours plus `chart-1..5`. These look like leftovers: neither file above uses them.

**Colour names the components use** (number of class uses across `ui/*.rs`):
muted-foreground 107, accent 70, primary 67, destructive 67, background 65, foreground 64,
ring 57, accent-foreground 45, muted 44, input 32, card 26, border 26, sidenav-accent-foreground 19,
popover-foreground 18, sidenav-accent 15, primary-foreground 13, secondary 10, warning 7,
sidenav-ring 6, sidenav-foreground 6, sidenav-border 5, success 4, sidenav 4,
secondary-foreground 4, info 4, card-foreground 4, warning-light 3, info-light 3,
warning-foreground 2, warning-dark 2, success-light 2, success-foreground 2, popover 2,
info-dark 2, destructive-foreground 2, surface 1, surface-foreground 1, success-dark 1, info-foreground 1.
No component uses `chart-*` (AreaChart reads them through ApexCharts JS).
`dark:` variants appear 50 times. They need the `.dark` class variant
(`@custom-variant dark (&:is(.dark *))`), which [theme_toggle](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/theme_toggle.rs) sets on `<html>`.

**Full CSS variable set in the site's `tailwind.css`** (each has a `.dark` value):

- Core (shadcn): `--background --foreground --card --card-foreground --popover --popover-foreground
  --primary --primary-foreground --secondary --secondary-foreground --muted --muted-foreground
  --accent --accent-foreground --destructive --border --input --ring`
- Extras: `--primary-light --primary-dark --destructive-light --destructive-dark`
- Status: `--success --success-foreground --success-light --success-dark`, the same four for `--warning-*` and `--info-*`
- Sidebar: `--sidenav --sidenav-foreground --sidenav-primary --sidenav-primary-foreground
  --sidenav-accent --sidenav-accent-foreground --sidenav-border --sidenav-ring`
- Charts: `--chart-1..5 --chart-primary --chart-muted --chart-grid --chart-1-rgb --chart-2-rgb`
- Misc: `--selection --selection-foreground --surface --surface-foreground --slider-track
  --scrollbar-thumb --scrollbar-track`
- Radius: `--radius: 0.625rem`, mapped to `--radius-sm` (−4px), `--radius-md` (−2px),
  `--radius-lg` (=), `--radius-xl` (+4px). Components use `rounded-md` 79×, `rounded-sm` 30×,
  `rounded-full` 27×, `rounded` 25×, `rounded-lg` 14×, `rounded-xl` 10×, `rounded-2xl` 4×.
  `rounded` and `rounded-2xl` are not tied to `--radius`, so they keep Tailwind's defaults.
- Fonts: `--font-sans: 'Inter', sans-serif`. `@theme inline` declares `--font-mono: var(--font-mono)`,
  but no `--font-mono` value is defined anywhere, so the reference is circular.

**Gaps that matter when we define our Themes:**

- `--surface`/`--surface-foreground` are defined in `:root` but have no `--color-surface*` in `@theme`,
  so `bg-surface` (the callout Default variant) generates no style.
- `--destructive-foreground` is used by badge and dropzone. The site CSS never defines it; the CLI
  template maps `--color-destructive-foreground` but never sets the variable.
- The CLI template omits success/warning/info, sidenav, surface, selection and the `-light`/`-dark`
  extras. Button (Warning/Success), Badge, Callout, Toast and Sidenav therefore render unstyled
  in a fresh `ui init` project. The CLI also writes `--sidebar-primary*` (shadcn's name), but the
  components read `sidenav-*`.
- The site's base CSS uses `hsl(var(--border))` for scrollbars while `--border` is an oklch value.
  That declaration is invalid. It is cosmetic only, but don't copy it.

So the Theme ticket should start from the site `tailwind.css` set above, not from the CLI template.

## Component table

`(n)` after a name is the number of `#[component]`s in the file. "Colour names" lists the
token suffixes the file uses (`bg-`, `text-`, `border-`, `ring-`, … prefixes removed).
"Registry deps" lists other `ui/` modules it imports; hooks are under `app_crates/registry/src/hooks/`.

| Component | Purpose | Native or own behaviour | Keyboard / ARIA | Variants and sizes | Colour names | `icons` | JS / crates / hooks | Registry deps | Flags |
|---|---|---|---|---|---|---|---|---|---|
| [accordion](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/accordion.rs) (8) | Expand/collapse sections | Own, CSS-only: visually hidden `input type=checkbox` + `peer-checked` grid rows | Checkbox is focusable, Space toggles; no `aria-expanded`/region | - | input, muted, muted-foreground, ring | - | - | - | - |
| [alert](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/alert.rs) (3) | Static message box | Plain `div` | `role=alert` | AlertVariant: Default, Destructive | background, destructive, foreground | - | - | - | - |
| [alert_dialog](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/alert_dialog.rs) (10) | Confirm dialog | Wraps `Dialog` with `close_on_backdrop_click: false` | Inherits Dialog: Esc only; no `role=alertdialog` | - | - | - | - | dialog | Inherits Dialog flags |
| [animate](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/animate.rs) (3) | Entrance/hover animation wrappers | CSS classes (~60 hover variants) | None | AnimateVariant: Default, FadeUp, AnimateScrollFadeOut, AnimateScrollBigger; AnimateHoverVariant: Default, Blink, BlurredFadeIn, BounceFadeIn, BounceHorizontal, BounceVertical, BounceCustom, ContractHorizontally, ContractVertic ... | - | - | strum | - | Marketing/demo |
| [aspect_ratio](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/aspect_ratio.rs) (1) | Fixed-ratio box | `div` with inline `aspect-ratio` | None | - | - | - | - | - | - |
| [attachment](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/attachment.rs) (9) | File attachment card (chat) | Composition; upload states | `aria-label` on actions | AttachmentSize: Default, Sm, Xs; AttachmentOrientation: Horizontal, Vertical; AttachmentState: Done, Idle, Uploading, Processing, Error; AttachmentMediaVariant: Icon, Image | card, card-foreground, destructive, foreground, muted, muted-foreground, ring | - | strum | button, dialog | Chat UI |
| [auto_form](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/auto_form.rs) (1) | Form generated from a struct | Stub: Leptos derive macro not ported | None | - | - | - | - | - | **Unfinished stub** |
| [avatar](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/avatar.rs) (6) | User image + fallback + badge/group | Native `img` | `alt` optional (defaults empty) | AvatarSize: Sm, Default, Lg | background, border, muted, muted-foreground, primary, primary-foreground | - | strum | - | - |
| [badge](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/badge.rs) (1) | Label pill | `span` | None | BadgeVariant: Default, Secondary, Accent, Muted, Destructive, Outline, Success, Warning, Info; BadgeSize: Default, Sm, Lg | accent, accent-foreground, destructive, destructive-foreground, foreground, info-dark, info-light, muted, muted-foreground, primary, primary-foreground, ring, secondary, secondary-foreground, success-dark, success-light, warning-dark, warning-light | - | - | - | - |
| [bottom_nav](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/bottom_nav.rs) (4) | Mobile bottom navigation | `button`s | `aria-current` | - | background, border, muted-foreground, primary | - | - | - | Mobile app chrome |
| [breadcrumb](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/breadcrumb.rs) (7) | Path trail | Native `nav`/links | `aria-label`, `aria-current`, ellipsis `aria-haspopup` | - | foreground, muted-foreground | ChevronRight, Ellipsis | - | - | - |
| [bubble](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/bubble.rs) (4) | Chat message bubble | `div` | `aria-label` on reactions | BubbleVariant: Default, Secondary, Muted, Tinted, Outline, Ghost, Destructive; BubbleAlign: Start, End; BubbleReactionsSide: Bottom, Top | background, border, card, destructive, foreground, input, muted, primary, primary-foreground, ring, secondary, secondary-foreground | - | strum | - | Chat UI |
| [button](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/button.rs) (1) | Button, or router `Link` when `href` set | Native `button` (type defaults `button`) | Native; `aria_label`, `title` props | ButtonVariant: Default, Destructive, Warning, Success, Outline, Secondary, Ghost, Link; ButtonSize: Default, Sm, Lg, Icon, IconSm, IconXs | accent, accent-foreground, background, destructive, input, primary, primary-foreground, ring, secondary, secondary-foreground, success, success-foreground, warning, warning-foreground | - | - | - | `button_type: String` TODO; no `loading` |
| [button_action](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/button_action.rs) (1) | Press-and-hold to confirm | Composes `Button` + `use_press_hold` hook | Pointer events only | - | - | - | hook use_press_hold | button | No keyboard path to activate |
| [button_group](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/button_group.rs) (3) | Joined button row | `div` + `Separator` | `role=group` | ButtonGroupOrientation: Horizontal, Vertical | muted | - | - | separator | - |
| [callout](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/callout.rs) (1) | Info/warning note | `div` | None | CalloutVariant: Default, Info, Warning | border, foreground, info, info-dark, info-light, muted-foreground, surface, surface-foreground, warning, warning-dark, warning-light | - | - | - | Default variant uses `bg-surface`/`text-surface-foreground`, not registered in `@theme` |
| [card](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/card.rs) (9) | Surface container with header/footer/action | `div`s | None | CardSize: Default, Sm | card, card-foreground, muted-foreground | - | - | - | - |
| [card_carousel](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/card_carousel.rs) (9) | Image card slider | `button`/`img` | `aria-current`, `aria-disabled` | - | accent | - | - | - | Hard-coded `bg-gray-200`, 320px; demo |
| [carousel](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/carousel.rs) (6) | Scroll-snap slider | Inline `<script>` | Arrow keys; `role=region` + `aria-roledescription` | CarouselOrientation: Horizontal, Vertical | accent, accent-foreground, background, muted-foreground | ChevronLeft, ChevronRight | - | - | - |
| [charts](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/charts.rs) (1) | `AreaChart` | Emits `data-chart-*` attrs only; drawn by site's ApexCharts + `chart_init.js` | None | ChartCurve: Smooth, Straight, Stepline | - | - | strum, hook use_random | - | **Not self-contained** (external JS) |
| [checkbox](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/checkbox.rs) (1) | Checkbox | Own: `button role=checkbox` | Native button keys; `aria-checked`, default `aria-label` "Checkbox" | - | destructive, input, primary, primary-foreground, ring | Check | - | - | Controlled `checked` + `on_checked_change`; no indeterminate |
| [chips](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/chips.rs) (2) | Toggleable chips | Native checkbox `input` + `label` | Native | - | card, muted, muted-foreground, warning | - | - | - | No `tw_merge`/`class` merge; demo |
| [collapsible](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/collapsible.rs) (3) | Show/hide region | Rust signal; `button` trigger, grid rows | No `aria-expanded`/`aria-controls` | - | - | - | - | - | Optional controlled `open` |
| [command](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/command.rs) (15) | Command palette / searchable list | Inline `<script>` (filtering, open/close) | Up/Down/Enter/Esc, Ctrl/Cmd+K; `role=combobox`/`option`, `aria-expanded`/`controls`/`selected` | - | accent, accent-foreground, background, foreground, input, muted, muted-foreground, popover-foreground | Check | inline JS | button | Best keyboard support in catalogue |
| [context_menu](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/context_menu.rs) (12) | Right-click menu with submenus | Inline `<script>` + `web_sys`, popover attr | Esc, outside click; no `role=menu`, no arrow keys | - | accent, accent-foreground, card, popover-foreground | ChevronRight | web-sys, inline JS, hook use_random | - | - |
| [data_grid](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/data_grid.rs) (19) | Virtualised editable grid: sort, pin, resize, select | Rust hooks (`use_virtual_scroll`, `use_cell_edit`) + `web_sys` | `role=grid`/`row`/`gridcell`, `aria-sort`/`rowindex`/...; Enter/Esc edit | SortDirection: None, Asc, Desc | accent, background, border, muted-foreground, primary, ring | ArrowDownWideNarrow, ArrowUpNarrowWide, ChevronDown, CircleX, EyeOff, PanelLeft, PanelLeftClose | web-sys, strum, hook use_cell_edit, hook use_virtual_scroll | checkbox, dropdown_menu | 913 lines; most complex |
| [data_table](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/data_table.rs) (0) | Alias of `table` | Re-export | None | - | - | - | - | table | Name only |
| [date_picker](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/date_picker.rs) (11) | Month calendar | Rust; `table` of `button`s; `time` crate | `aria-label`/`current`/`disabled`; no arrow keys | - | accent, accent-foreground, background, card, card-foreground, input, muted-foreground, primary, primary-foreground, ring | - | time | - | State in `date_picker_state` |
| [date_picker_dual_state](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/date_picker_dual_state.rs) (0) | Range-picker state helper | Plain Rust, no UI | n/a | - | - | - | time | - | Not a component |
| [date_picker_state](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/date_picker_state.rs) (0) | Calendar state helper | Plain Rust, no UI | n/a | - | - | - | time | - | Not a component |
| [dialog](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/dialog.rs) (10) | Modal dialog | Own: uncontrolled, inline `<script>` toggles `data-state`; no `<dialog>` | Esc, backdrop click; **no `role=dialog`/`aria-modal`, no focus trap/return** | - | accent, accent-foreground, background, muted-foreground, primary, primary-foreground, ring | X | inline JS | - | No `open` prop or `on_open_change` |
| [direction_provider](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/direction_provider.rs) (1) | LTR/RTL wrapper | `div dir=` | None | Direction: Ltr, Rtl | - | - | strum | - | - |
| [drag_and_drop](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/drag_and_drop.rs) (3) | Sortable list by dragging | `web_sys`/`js_sys` document event delegation | Pointer only | - | card, input | - | web-sys, js-sys | - | Hard-coded `bg-neutral-600`, `dragabble__container`; demo |
| [drawer](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/drawer.rs) (10) | Edge drawer with drag handle | Inline `<script>` | Esc; no dialog roles | DrawerPosition: Bottom, Top, Left, Right; DrawerVariant: Default, Floating | accent, accent-foreground, background, muted-foreground, ring | - | inline JS | button | Same pattern as Dialog |
| [dropdown_menu](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/dropdown_menu.rs) (14) | Menu from trigger, submenus, radio items | Inline `<script>`, popover attr | Esc, outside click; `menuitemradio` only; no `role=menu`, no arrow keys | DropdownMenuAlign: Start, StartOuter, End, EndOuter, Center; DropdownMenuPosition: Auto, Top, Bottom; DropdownMenuActionVariant: Default, Destructive | accent, accent-foreground, background, card, destructive, input, muted-foreground, popover-foreground, ring | Check, ChevronRight | inline JS, hook use_random | separator | - |
| [dropzone](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/dropzone.rs) (9) | File drop + list/grid preview | `web_sys` File API + `eval` | `aria-label` | ViewMode: List, Grid | accent, background, border, destructive, destructive-foreground, foreground, muted, muted-foreground, primary | FileArchive, FileAudio, FileCode, FileImage, FileSpreadsheet, FileText, LayoutGrid, LayoutList | web-sys, eval | - | Hard-coded `bg-red-500`, `from-black` |
| [empty](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/empty.rs) (6) | Empty-state block | `div`s | None | EmptyMediaVariant: Default, Icon | foreground, muted, muted-foreground | - | - | - | - |
| [field](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/field.rs) (10) | Form field layout, legend, error | `div`s, `label` | Error `role=alert` | FieldLegendVariant: Legend, Label; FieldVariant: Vertical, Horizontal, Responsive | background, destructive, muted-foreground, primary | - | - | separator | - |
| [footer](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/footer.rs) (16) | Site footer | `footer`/links | `role=contentinfo` | - | foreground, primary | - | - | - | Marketing chrome |
| [form](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/form.rs) (14) | Form with validation context | `use_form` hook; native `input` | `aria-invalid`, error `role=alert` | FormLegendVariant: Legend, Label; FormFieldVariant: Vertical, Horizontal, Responsive | background, destructive, foreground, input, muted, muted-foreground, primary, primary-foreground, ring | - | strum, hook use_form | label, separator | - |
| [header](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/header.rs) (18) | Site header + nav menu | Popover attr, `use_data_scrolled` | `aria-expanded`/`controls` | - | accent, background, border, foreground, muted, muted-foreground, popover, ring | ChevronDown | hook use_data_scrolled | - | Marketing chrome; sets `--color-popover` inline |
| [hover_card](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/hover_card.rs) (3) | Hover preview card | Popover API (`showPopover`) + CSS anchor positioning, inline `<script>` | Hover only (150 ms delay); no focus trigger | HoverCardSide: Top, Bottom, Left, Right | card | - | - | - | Needs anchor-positioning browser support |
| [image](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/image.rs) (1) | Image | Native `img` | Required `alt` | - | - | - | - | - | - |
| [input](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/input.rs) (1) | Text-like input | Native `input`; `InputType` enum | Native; `aria-invalid` styling | InputType: Text, Email, Password, Number, Tel, Url, Search, Time, DatetimeLocal, Date, Month, Week, Hidden, File, Checkbox, Radio, Color, Range | destructive, foreground, input, muted, muted-foreground, primary, primary-foreground, ring | - | strum | - | `value` + `oninput`; no `onchange`/`onkeydown` props |
| [input_group](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/input_group.rs) (6) | Input with addons/buttons | Native `input`/`textarea`/`button` | `role=group` | InputGroupAddonAlign: InlineStart, InlineEnd, BlockStart, BlockEnd; InputGroupButtonVariant: Ghost; InputGroupButtonSize: Xs, Sm, IconXs, IconSm | destructive, input, muted-foreground, ring | - | strum | input | Own button enums, not `Button` |
| [input_otp](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/input_otp.rs) (4) | One-time-code boxes | Native `input`; loads `script src=/components/otp.js` | Depends on missing JS | - | destructive, foreground, input, muted-foreground, ring | Minus | - | - | **`otp.js` not in repo** |
| [input_phone](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/input_phone.rs) (2) | Phone input with country picker | Composes `Command`+`Popover`+`Input`; `serde` | Via Command | - | foreground, muted, muted-foreground | ChevronsUpDown, Search, X | serde | command, input, popover | - |
| [input_prompt](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/input_prompt.rs) (5) | Chat prompt box | Auto-grow `textarea` | Enter submits, Shift+Enter newline | - | muted-foreground | - | - | button, input_group | Chat UI |
| [item](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/item.rs) (10) | List row layout (media/title/actions) | `div`s | Separator `role=separator` | ItemVariant: Default, Outline, Muted; ItemSize: Default, Sm, Xs; ItemMediaVariant: Default, Icon, Image | accent, border, muted, muted-foreground, primary, ring | - | - | - | - |
| [kbd](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/kbd.rs) (2) | Keyboard key glyph | `kbd` | None | - | background, muted, muted-foreground | - | - | - | - |
| [label](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/label.rs) (1) | Form label | Native `label` | Native | - | - | - | - | - | - |
| [link](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/link.rs) (1) | Router link with active-path matching | Dioxus router `Link`; `web_sys` location | Native | PathMatchType: StartsWith, Exact, Contains, Custom(String), StartsWithExcept(String, Vec<String>), MatchAny(Vec<String>) | - | - | web-sys | - | No `tw_merge` |
| [marker](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/marker.rs) (3) | Chat timeline marker | `a`/`button`/`div` | Optional `role` | MarkerVariant: Default, Separator, Border | border, foreground, muted-foreground | - | strum | - | Chat UI |
| [marquee](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/marquee.rs) (3) | Scrolling ticker | CSS animation | None | - | background | - | - | - | Marketing |
| [mask](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/mask.rs) (2) | Edge fade mask | CSS gradient | None | MaskSide: Default, Left, Right, Top, Bottom | background | - | - | - | `from-white` hard-coded light side |
| [menubar](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/menubar.rs) (15) | Desktop menu bar, submenus, check/radio items | Inline `<script>`, popover attr | Esc, outside click; `menuitemcheckbox`/`radio`; no arrow keys | - | accent, accent-foreground, background, card, muted, muted-foreground, popover-foreground | Check, ChevronRight | inline JS, hook use_random | separator | - |
| [message](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/message.rs) (6) | Chat message layout | `div`s | None | MessageAlign: Start, End | muted, muted-foreground | - | strum | - | Chat UI |
| [multi_select](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/multi_select.rs) (8) | Multi-value dropdown | Inline `<script>`, `HashSet` state | Esc; `role=listbox`/`option`, `aria-selected`; no arrow keys | MultiSelectAlign: Start, Center, End | accent, accent-foreground, background, card, input, muted-foreground, popover-foreground, ring | Check, ChevronDown, ChevronUp | inline JS, hook use_can_scroll_vertical, hook use_random | - | - |
| [navigation_menu](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/navigation_menu.rs) (6) | Site mega-menu | Inline `<script>`, fixed-position panels | Esc | - | accent, accent-foreground, background, foreground, popover | ChevronDown | inline JS, hook use_random | - | TODO in source; marketing |
| [pagination](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/pagination.rs) (6) | Page links | `use_pagination` hook; `a` | `aria-current`, `aria-label` | PageDirection: Previous, Next | accent, accent-foreground, muted-foreground, primary, primary-foreground | - | hook use_pagination | - | - |
| [popover](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/popover.rs) (5) | Floating panel from trigger | Own: CSS anchor positioning + inline `<script>` toggling `data-state` | Esc, outside click; content `role=dialog`; no focus move | PopoverAlign: Start, StartOuter, End, EndOuter, Center | accent, accent-foreground, background, card, input, muted-foreground, ring | - | inline JS | - | Uncontrolled; anchor-positioning support needed |
| [pressable](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/pressable.rs) (1) | Press scale effect | Pointer events on `div` | None | - | - | - | - | - | - |
| [progress](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/progress.rs) (1) | Progress bar | `div` | `role=progressbar`, `aria-value*` | - | primary, secondary | - | - | - | - |
| [radio_button](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/radio_button.rs) (2) | Radio group (`RadioGroup`/`RadioGroupItem`) | Own: `button role=radio`, `Signal<String>` context | Click only; no arrow keys/roving tabindex | - | input, primary, ring | - | - | - | - |
| [radio_button_group](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/radio_button_group.rs) (3) | Card-style radio buttons | Native `input type=radio` + `label` | Native | - | input, primary | - | - | - | Invalid `role="radio-button-group"` |
| [scroll_area](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/scroll_area.rs) (7) | Scroll container (+ snap variant) | CSS overflow | None | ScrollBarOrientation: Vertical, Horizontal; SnapAreaVariant: Center; SnapVariant: Center | border, ring | - | - | - | No custom scrollbar logic |
| [select](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/select.rs) (7) | Single-value dropdown | Own: inline `<script>` open/close; `li role=option tabindex=0` | Esc only; options are click-only, so Enter/Space likely do nothing; no arrow keys | SelectPosition: Below, Above | accent, accent-foreground, background, card, input, muted-foreground, popover-foreground, ring | Check, ChevronDown, ChevronUp | inline JS, strum, hook use_can_scroll_vertical, hook use_random | - | `SelectValue` shows raw value; uncontrolled (`default_value`) |
| [separator](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/separator.rs) (1) | Divider | `div` | `role=separator` | SeparatorOrientation: Horizontal, Vertical | border | - | - | - | - |
| [sheet](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/sheet.rs) (9) | Side panel modal | Inline `<script>` | Esc; no dialog roles/focus trap | SheetDirection: Top, Bottom, Left, Right | accent, accent-foreground, background, card, muted-foreground, ring | X | inline JS | button | Same pattern as Dialog |
| [shimmer](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/shimmer.rs) (1) | Loading shimmer | CSS | None | - | - | - | - | - | - |
| [sidenav](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/sidenav.rs) (22) | App sidebar: collapsible, resizable, menu | Rust + `eval` (localStorage width) + `web_sys` | `aria-current`/`disabled` | SidenavVariant: Sidenav, Floating, Inset; SidenavSide: Left, Right; SidenavCollapsible: Offcanvas, None, Icon; SidenavState: Expanded, Collapsed; SidenavMenuButtonVariant: Default, Outline; SidenavMenuButtonSize: Default, Sm, Lg | accent, accent-foreground, background, destructive, foreground, input, muted, muted-foreground, primary, primary-foreground, ring, sidenav, sidenav-accent, sidenav-accent-foreground, sidenav-border, sidenav-foreground, sidenav-ring | - | web-sys, eval, strum | - | Own `--sidenav-*` tokens; CLI writes `--sidebar-*` |
| [skeleton](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/skeleton.rs) (1) | Loading placeholder | `div` | None | - | muted | - | - | - | - |
| [slider](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/slider.rs) (1) | Range slider | Native `input type=range` | Native | SliderVariant: Round, Flat | primary | - | strum | - | Single thumb; `value` + `oninput` |
| [spinner](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/spinner.rs) (2) | Loading spinner | `icons` Loader | No `role=status`/label | - | - | Loader, LoaderCircle | - | - | - |
| [status](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/status.rs) (1) | Status dot | `div` | None | StatusVariant: Default, Active, Inactive, Normal | - | - | - | - | Hard-coded palette (`bg-green-300` etc.) |
| [stepper](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/stepper.rs) (7) | Multi-step indicator | `use_stepper` hook; `button`s | `aria-current` | StepperOrientation: Horizontal, Vertical | background, border, foreground, muted, muted-foreground, primary, primary-foreground, ring | Check | strum, hook use_stepper | - | - |
| [switch](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/switch.rs) (2) | On/off switch | Own: `button role=switch` | Native button keys; `aria-checked` | - | background, input, primary, ring | - | - | - | **No change callback**; `checked` is initial only |
| [table](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/table.rs) (9) | Data table markup | Native `table` | Native | - | card, muted, muted-foreground | - | - | - | TODO(css-parity) |
| [tabs](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/tabs.rs) (4) | Tabbed panels | Rust signal; `button` triggers | **No `tablist`/`tab`/`tabpanel` roles, no `aria-selected`, no arrow keys** | TabsVariant: Default, Line; TabsOrientation: Horizontal, Vertical | background, foreground, input, muted, muted-foreground, ring | - | - | - | Uncontrolled (`default_value`), no `on_change` |
| [textarea](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/textarea.rs) (1) | Multi-line text | Native `textarea` | Native | - | destructive, input, muted-foreground, ring | - | - | - | - |
| [theme_toggle](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/theme_toggle.rs) (1) | Light/dark toggle | `eval`: toggles `.dark` on `<html>`, localStorage `darkmode` | `aria-label` | - | accent | Moon, Sun | eval | - | Hard-wires `.dark` class + key |
| [toast](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/toast.rs) (4) | Notifications | Markup + `eval`; behaviour in site's `public/app_components/toast.js` | No live region in Rust source | ToastType: Default, Success, Error, Warning, Info, Loading; ToastPosition: TopLeft, TopCenter, TopRight, BottomRight, BottomCenter, BottomLeft; ToastDirection: TopDown, BottomUp | destructive, info, info-foreground, primary, primary-foreground, ring, secondary, secondary-foreground, success, success-foreground, warning, warning-foreground | - | eval, serde, strum | - | **Not self-contained** (external JS) |
| [toggle_group](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/toggle_group.rs) (3) | Segmented toggle buttons | `button role=radio` | No `aria-checked`; no arrow keys | ToggleGroupVariant: Default, Outline; ToggleGroupOrientation: Horizontal, Vertical | accent, accent-foreground, destructive, muted, muted-foreground, ring | - | - | - | No group value; `pressed` set by caller |
| [toolbar](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/toolbar.rs) (8) | Toolbar with toggle items | `button`s, `ul` | `role=toolbar`, `aria-pressed`; no roving tabindex | ToolbarButtonVariant: Default, Ghost, Outline | accent, accent-foreground, background, input, muted, muted-foreground, ring | - | - | separator | `multiple` prop stored, caller controls `pressed` |
| [tooltip](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/tooltip.rs) (3) | Hover label | Pure CSS `group-hover` | **Hover only, no focus, no `role=tooltip`** | TooltipPosition: Top, Left, Right, Bottom | background, foreground | - | - | - | `TooltipProvider` renders nothing |
| [workflow](https://github.com/rust-ui/ui/blob/7120636e68556ddd023a4ec8e083663c7b745318/app_crates/registry/src/ui/workflow.rs) (11) | Node-graph editor canvas | Rust; `use_workflow` hook; SVG | Arrows/Delete/Backspace/Enter/Esc shortcuts | - | accent, background, border, card, card-foreground, destructive, foreground, muted-foreground, primary, secondary | Clipboard, Copy, Pencil, Plus, SquareDashedMousePointer, Trash2 | hook use_workflow | context_menu | 888 lines; app-specific demo |
