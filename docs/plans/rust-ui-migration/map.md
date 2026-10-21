# Map: Migrate the workbench UI to rust-ui

Label: `wayfinder:map`

## Destination

A migration spec, ready to hand off as build tickets: the target structure of the workbench,
every existing building block and plain HTML control mapped to one rust-ui component (or an
approved composition), the Original and shadcn Themes defined, and the single-branch lane plan
decided.

## Notes

- Decisions so far are recorded in [ADR-0034](../../adr/0034-adopt-rust-ui-components-with-two-themes.md)
  and the glossary's **Theme** and **Appearance**. They are the current answer, not a
  lock: any ticket may revise them, the ADR, or this destination.
- Planning only: tickets produce decisions; the migration itself is later build work.
- Upstream: [rust-ui/ui](https://github.com/rust-ui/ui), components in
  `app_crates/registry/src/ui/`. The `ui mcp` server (from `cargo install ui-cli`, local
  `.mcp.json`, started from `web/`) currently lists only hooks and cannot fetch component
  source, so read the GitHub source instead.
- Find code with Gortex (`explore`, `search_text`) before reading files.
- Skills: `grilling` and `domain-modeling` for grilling tickets; `impeccable` for Theme
  look-and-feel; `research` for research tickets.
- Settled scope (2026-10-21, user): rust-ui supplies the look and `dioxus-primitives` the
  behaviour; no inline scripts or ownerless document listeners in copied code; restructuring
  the app (workspaces, panels, navigation) is in scope, canvases are not; call components
  directly and delete the `Board*` wrappers; copy source to `web/crates/ui-shared/src/ui/<name>.rs` with an
  upstream-commit provenance note and manual re-sync; rust-ui colour names only, `--wb-*`
  and legacy CSS retired; icon set open (see 008); Theme is a saved preference, Original by
  default; tests follow the new components, product behaviour
  (selection, Undo, shortcuts, precise values) does not change; a missing match is
  composed from rust-ui first, custom only with user approval; one long-lived branch with
  per-workspace lanes, merged once.

## Tickets

<!-- every ticket, open or resolved; status lives in the ticket -->

- [001: Compile rust-ui components against our Dioxus 0.7.10 web build](issues/001-compile-rust-ui-against-our-build.md) — `wayfinder:task`; blocked by: None
- [002: Fact sheet for rust-ui's component catalogue](issues/002-rust-ui-catalogue-facts.md) — `wayfinder:research`; blocked by: None
- [003: Document the existing UI building blocks and control sites](issues/003-inventory-existing-ui.md) — `wayfinder:task`; blocked by: None
- [004: Map each building block and control pattern to one rust-ui component](issues/004-map-building-blocks-to-rust-ui.md) — `wayfinder:grilling`; blocked by: 003, 011
- [005: How Themes and Appearance are structured](issues/005-theme-structure.md) — `wayfinder:grilling`; blocked by: 002
- [006: Solar replacements for rust-ui's icons](issues/006-solar-icons-for-rust-ui.md) — `wayfinder:research`; blocked by: None
- [008: Choose the workbench icon set](issues/008-choose-icon-set.md) — `wayfinder:grilling`; blocked by: None
- [009: How to close rust-ui's control and accessibility gaps](issues/009-close-rust-ui-behaviour-gaps.md) — `wayfinder:grilling`; blocked by: None
- [010: Measure keyboard and ARIA conformance of the controls we would adopt](issues/010-measure-keyboard-and-aria-conformance.md) — `wayfinder:task`; blocked by: None
- [011: Target structure of the workbench around rust-ui patterns](issues/011-target-workbench-structure.md) — `wayfinder:grilling`; blocked by: 003
- [007: Branch, lane and test-rewrite plan for the single merge](issues/007-branch-and-lane-plan.md) — `wayfinder:grilling`; blocked by: 004, 005

## Decisions so far

<!-- one line per resolved ticket: linked title, then a one-line gist -->

- [006: Solar replacements for rust-ui's icons](issues/006-solar-icons-for-rust-ui.md): no Solar exists in the app; rust-ui uses 35 Lucide icons; icon set reopened as 008.
- [003: Document the existing UI building blocks and control sites](issues/003-inventory-existing-ui.md): [inventory](inventory.md) of 210 components, 397 `Board*` uses and 331 raw controls with an 88-file sweep checklist; wrappers cover about half the controls.
- [001: Compile rust-ui components against our Dioxus 0.7.10 web build](issues/001-compile-rust-ui-against-our-build.md): button, popover, select and dialog compile and run on 0.7.10 with `tw_merge`, `strum`, `icons` and one `Link` edit; inline scripts execute; keyboard and ARIA gaps confirmed in a browser.
- [009: How to close rust-ui's control and accessibility gaps](issues/009-close-rust-ui-behaviour-gaps.md): rust-ui supplies the look, `dioxus-primitives` the behaviour; no inline scripts; accessibility measured (010); restructuring added to scope (011).
- [002: Fact sheet for rust-ui's component catalogue](issues/002-rust-ui-catalogue-facts.md): 86 components; overlays/Select/Tabs/Switch uncontrolled with thin keyboard and ARIA (→ 009); shadcn token set plus extras, with gaps.

## Not yet specified

- Per-gap equivalence decisions: each building block with no direct rust-ui match gets
  its own ticket once the mapping names them.
- Theme look: what the shadcn Theme's base palette is and how Original's Graphite values
  land in rust-ui's names; likely a prototype once the Theme structure is decided.
- Compact density: whether BoardStudio's compact control sizes become a rust-ui size
  variant or a Theme concern.
- Native-to-custom control shifts (Select, number fields, `details`): what precise-entry
  and keyboard behaviour must be re-proven, feeding the test-rewrite rules.
- Legacy CSS retirement order across lanes, and layout rules in `m1.css` that are not
  component styling.
- Sweep checklist for the ~200 app-specific components, per workspace.

## Out of scope

- Canvas and 3D surfaces (`Scene`, `CaseViewer`, `KeycapsCanvas`, `PcbScene`,
  `KeymapCanvas`, `FootprintGraphics`, canvas overlays): rust-ui has nothing for them;
  their toolbars and menus are in scope.
- Fixing upstream's MCP registry.
