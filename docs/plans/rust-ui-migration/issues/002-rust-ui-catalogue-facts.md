# 002: Fact sheet for rust-ui's component catalogue

Status: resolved
Type: research
Blocked by: None
Parent: [Map](../map.md)
Findings: branch `research/rust-ui-catalogue`, `docs/research/2026-10-21-rust-ui-catalogue.md`

## Question

For each of the 87 components in `rust-ui/ui` `app_crates/registry/src/ui/` at a pinned
commit: what it is for; whether it wraps a native element or implements its own
behaviour; keyboard and ARIA support; props and variants/sizes; colour names it
consumes; crates and `icons` it uses; and which other registry components it depends
on. Flag components that look unfinished or demo-only.

## Answer

At `7120636e`, `ui/` holds 86 components (two are date-picker state helpers, `data_table`
re-exports `table`); about 15 are marketing, demo or chat UI. Read from source only.

- **Behaviour:** Dialog, Sheet, Drawer, Popover, Select and the menus are uncontrolled:
  inline `<script>` blocks open and close them, with no `open` prop or callback. Overlays
  handle only Escape and outside click: no arrow keys, no focus trap, Dialog lacks
  `role=dialog`. Select options are click-only; Tabs has no ARIA, arrows or change
  callback; Tooltip is hover-only; Switch has no change callback. Command alone is fully
  keyboard-capable. Toast, AreaChart and InputOTP need JS absent from the components.
- **Tokens:** shadcn core (`background`…`ring`), plus `primary/destructive-light/-dark`,
  `success`/`warning`/`info` sets, `sidenav-*`, `chart-*`, `selection`, `surface`,
  `slider-track`, `scrollbar-*`; `--radius` 0.625rem, `--font-sans` Inter; oklch values
  from the docs site's `tailwind.css`. Gaps: `surface` has no `--color-surface`,
  `destructive-foreground` and `--font-mono` are undefined, `ui init` writes `sidebar-*`
  while components read `sidenav-*`.

The behaviour gaps are taken up in [009](009-close-rust-ui-behaviour-gaps.md); tokens feed
[005](005-theme-structure.md). Full sheet: branch `research/rust-ui-catalogue`,
`docs/research/2026-10-21-rust-ui-catalogue.md`.
