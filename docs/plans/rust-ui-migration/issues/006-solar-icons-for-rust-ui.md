# 006: Solar replacements for rust-ui's icons

Status: resolved
Type: research
Blocked by: None
Parent: [Map](../map.md)
Findings: branch `research/rust-ui-icons`, `docs/research/2026-10-21-rust-ui-icons.md`

## Question

Which icons do rust-ui components use from its `icons` crate, how are they wired into
each component, and what is the Solar equivalent available through BoardStudio's
existing icon wrapper (and its local asset set) for each?

## Answer

BoardStudio has no Solar icons or icon wrapper: Solar was planned in the UI design lane
but never imported, and today's icons are hand-drawn inline SVGs (`TabIcon` and others).
rust-ui (at `7120636e`) uses 35 distinct Lucide icons (MIT) across 21 of 87 components,
each a component taking only `class`; none animated. Every one has a Solar counterpart in
Iconify's `solar` set (CC BY 4.0, attribution required in `web/assets/licenses/README.txt`),
but none is vendored. The "keep Solar" premise was therefore wrong; the icon set is
reopened as [008](008-choose-icon-set.md). Full table: branch `research/rust-ui-icons`,
`docs/research/2026-10-21-rust-ui-icons.md`.
