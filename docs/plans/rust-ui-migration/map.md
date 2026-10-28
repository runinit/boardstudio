# Map: Migrate the workbench UI to rust-ui

Label: `wayfinder:map`

## Destination

A migration spec, ready to hand off as build tickets: the target structure of the workbench,
every existing building block and plain HTML control mapped to one rust-ui component (or an
approved composition), the Original and shadcn Themes defined, and the single-branch lane plan
decided.

**Reached 2026-10-21:** the [spec](spec.md) is assembled and ready to hand off as build tickets. All 13 decision tickets are resolved.

## Notes

- Decisions so far are recorded in [ADR-0039](../../adr/0039-adopt-rust-ui-components-with-two-themes.md)
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
  the app shell follows ADR-0040 and canvases are out of scope; call components
  directly and delete the `Board*` wrappers; copy source to `web/crates/ui-shared/src/ui/<name>.rs` with an
  upstream-commit provenance note and manual re-sync; rust-ui colour names only, `--wb-*`
  and legacy CSS retired; icons are vendored Lucide (008); Theme is a saved preference, Original by
  default; tests follow the new components, product behaviour
  (selection, Undo, shortcuts, precise values) does not change; a missing match is
  composed from rust-ui first, custom only with user approval; one long-lived branch with
  per-workspace lanes, merged once.

## Tickets

<!-- every ticket, with its type and status; the answers are in the tickets -->

- [001: Compile rust-ui components against our Dioxus 0.7.10 web build](issues/001-compile-rust-ui-against-our-build.md) — `wayfinder:task`; resolved
- [002: Fact sheet for rust-ui's component catalogue](issues/002-rust-ui-catalogue-facts.md) — `wayfinder:research`; resolved
- [003: Document the existing UI building blocks and control sites](issues/003-inventory-existing-ui.md) — `wayfinder:task`; resolved
- [004: Map each building block and control pattern to one rust-ui component](issues/004-map-building-blocks-to-rust-ui.md) — `wayfinder:grilling`; resolved
- [005: How Themes and Appearance are structured](issues/005-theme-structure.md) — `wayfinder:grilling`; resolved
- [006: Solar replacements for rust-ui's icons](issues/006-solar-icons-for-rust-ui.md) — `wayfinder:research`; resolved
- [007: Branch, lane and test-rewrite plan for the single merge](issues/007-branch-and-lane-plan.md) — `wayfinder:grilling`; resolved
- [008: Choose the workbench icon set](issues/008-choose-icon-set.md) — `wayfinder:grilling`; resolved
- [009: How to close rust-ui's control and accessibility gaps](issues/009-close-rust-ui-behaviour-gaps.md) — `wayfinder:grilling`; resolved
- [010: Measure keyboard and ARIA conformance of the controls we would adopt](issues/010-measure-keyboard-and-aria-conformance.md) — `wayfinder:task`; resolved
- [011: Target structure of the workbench around rust-ui patterns](issues/011-target-workbench-structure.md) — `wayfinder:grilling`; resolved
- [012: Command palette contents and shortcut hints](issues/012-command-palette-contents.md) — `wayfinder:grilling`; resolved
- [013: Prototype the Original and shadcn Themes on copied components](issues/013-prototype-theme-look.md) — `wayfinder:prototype`; resolved

## Decisions so far

<!-- one line per resolved ticket, in ticket order: linked title, then a one-line gist -->

- [001: Compile rust-ui components against our Dioxus 0.7.10 web build](issues/001-compile-rust-ui-against-our-build.md): button, popover, select and dialog compile and run on 0.7.10 with `tw_merge`, `strum`, `icons` and one `Link` edit; inline scripts execute; keyboard and ARIA gaps confirmed in a browser.
- [002: Fact sheet for rust-ui's component catalogue](issues/002-rust-ui-catalogue-facts.md): 86 components; overlays/Select/Tabs/Switch uncontrolled with thin keyboard and ARIA (→ 009); shadcn token set plus extras, with gaps.
- [003: Document the existing UI building blocks and control sites](issues/003-inventory-existing-ui.md): [inventory](inventory.md) of 210 components, 397 `Board*` uses and 331 raw controls with an 88-file sweep checklist; wrappers cover about half the controls.
- [004: Map each building block and control pattern to one rust-ui component](issues/004-map-building-blocks-to-rust-ui.md): [mapping](mapping.md) decided; no new custom components; sizes by pixels, Select vs Combobox, Stepper, message and Tooltip rules, file inputs stay native.
- [005: How Themes and Appearance are structured](issues/005-theme-structure.md): [theme-contract](theme-contract.md) decided: `data-appearance` and `data-theme`, rust-ui token names mapped by meaning plus a few extras, a Theme sets colour / radius / font only, shared layout metrics, Inter for shadcn, 3D keeps its own palettes.
- [006: Solar replacements for rust-ui's icons](issues/006-solar-icons-for-rust-ui.md): no Solar exists in the app; rust-ui uses 35 Lucide icons; icon set reopened as 008.
- [007: Branch, lane and test-rewrite plan for the single merge](issues/007-branch-and-lane-plan.md): [branch plan](branch-plan.md): one `feat/rust-ui-migration` branch from `dev`, a Foundation lane then Shell, Layout and Case in parallel (max 3), role-and-name tests, an accessibility floor, an old-CSS check, and a six-part merge gate.
- [008: Choose the workbench icon set](issues/008-choose-icon-set.md): Lucide, vendored per icon (not the `icons` crate), one set for both Themes, custom glyphs only where no Lucide match, decorative by default with a Theme-settable stroke.
- [009: How to close rust-ui's control and accessibility gaps](issues/009-close-rust-ui-behaviour-gaps.md): rust-ui supplies the look, `dioxus-primitives` the behaviour; no inline scripts; accessibility measured (010); restructuring added to scope (011).
- [010: Measure keyboard and ARIA conformance of the controls we would adopt](issues/010-measure-keyboard-and-aria-conformance.md): [accessibility floor](accessibility-floor.md) measured on 15 controls; 9 pass, Select, Dropdown Menu, Toolbar, Slider and Toggle Group need fixes in our copy first.
- [011: Target structure of the workbench around rust-ui patterns](issues/011-target-workbench-structure.md): rebuild the header like rust-ui.com's, docks on Sidenav with a non-modal Sheet on narrow screens, drop auto-hide, add a Command palette and Toasts ([ADR-0040](../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md)).
- [012: Command palette contents and shortcut hints](issues/012-command-palette-contents.md): [command palette](command-palette.md): Ctrl/Cmd+K, existing actions only (Go to, Project, Edit, View, current Workspace, object search), never destructive edits, only Ctrl/Cmd+K added as a shortcut.
- [013: Prototype the Original and shadcn Themes on copied components](issues/013-prototype-theme-look.md): both Themes approved in a cards style (cards on a desk, filled controls, no borders); contract adds `selected` and `field`, `background` becomes the desk, built-in borders removed in the copies; header is a 56px borderless card.

## Not yet specified

Nothing. The way is clear: the shell build details became the Shell and Foundation build tickets in the [spec](spec.md).

## Out of scope

- Canvas and 3D surfaces (`Scene`, `CaseViewer`, `KeycapsCanvas`, `PcbScene`,
  `KeymapCanvas`, `FootprintGraphics`, canvas overlays): rust-ui has nothing for them;
  their toolbars and menus are in scope.
- Fixing upstream's MCP registry.
