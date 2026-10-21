---
status: accepted
---

# Adopt rust-ui components with two Themes

The workbench's controls come from [rust-ui](https://github.com/rust-ui/ui), copied
into `web/crates/ui-shared/src/ui/` and called directly, replacing the 2026-10-11
choice of DioxusLabs' Dioxus Components and the `Board*` wrapper layer built on
`dioxus-primitives`. The user chose this on 2026-10-21 to gain rust-ui's much wider
catalogue (87 components) in one open-code library rather than maintaining two.

Components consume only rust-ui's colour names (`primary`, `accent`, `muted`, `ring`…).
Two Themes fill them: Original, which carries today's Graphite look, and shadcn. Each has
light and dark values, selected independently by the existing Appearance setting.
The `--wb-*` variables and the legacy `m1.css` / `.board-*` / `.m1-*` rules are retired
when the migration completes. Solar icons stay; rust-ui's `icons` crate is not adopted.

Interaction details follow rust-ui where they differ (for example a custom Select
instead of a native one), and tests are rewritten to match. Product behaviour —
selection, Undo, shortcuts, precise values — is unchanged. A building block without a
direct rust-ui match is first composed from existing rust-ui components; a custom
component requires a shown absence of any equivalent and explicit user approval.

The migration lands at once from one long-lived branch with per-workspace lanes;
old and new controls do not coexist on `main`. Canvas and 3D surfaces stay custom.
The [rust-ui migration map](../plans/rust-ui-migration/map.md) charts the route.
