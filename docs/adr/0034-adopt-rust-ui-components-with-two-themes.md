---
status: accepted
---

# Adopt rust-ui components with two Themes

The workbench's controls take their look from [rust-ui](https://github.com/rust-ui/ui), copied
into `web/crates/ui-shared/src/ui/` and called directly; the `Board*` wrapper layer goes. The user
chose this on 2026-10-21 for rust-ui's wide catalogue (86 components) in one open-code library.
Reading its source showed it supplies almost no behaviour or accessibility: eleven overlay and
menu components run on inline scripts with leaking document listeners, and keyboard and ARIA
support is partial and untested. So `dioxus-primitives` (the 2026-10-11 choice of DioxusLabs'
Dioxus Components) stays as the behaviour layer for every control it covers, restyled with
rust-ui classes. Where primitives has no counterpart (Sheet, Drawer, Command, Navigation Menu)
we write the behaviour in Rust. Copied code ships no inline scripts or ownerless document
listeners. Fixes stay in our copy.

Components consume only rust-ui's colour names (`primary`, `accent`, `muted`, `ring`…).
Two Themes fill them: Original, which carries today's Graphite look, and shadcn. Each has
light and dark values, selected independently by the existing Appearance setting.
A Theme sets colours, corner radius, fonts and shadows only; layout sizes are shared by both Themes, and the 3D renderer follows
Appearance only ([theme contract](../plans/rust-ui-migration/theme-contract.md)). The `--wb-*` variables and the legacy `m1.css` / `.board-*` / `.m1-*` rules are retired
when the migration completes. Icons are Lucide, vendored one component per icon; rust-ui's `icons` crate is not adopted (ticket 008).

Product behaviour (selection, Undo, shortcuts, precise values) is unchanged. A building block
without a direct rust-ui match is first composed from existing rust-ui components; a custom
component requires a shown absence of any equivalent and explicit user approval. The effort
also restructures the app (workspaces, panels, navigation), not only its components.

The migration lands at once from one long-lived branch with per-workspace lanes;
old and new controls do not coexist on `main`. Canvas and 3D surfaces stay custom.
The [rust-ui migration map](../plans/rust-ui-migration/map.md) charts the route.
