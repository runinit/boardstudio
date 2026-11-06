# 024: Delete the old layer

Status: claimed
Owner: lane/024-delete-the-old-layer (.worktrees/lane-024-delete-the-old-layer)
Type: build
Blocked by: [014: Shell glue sweep](014-shell-glue-sweep.md), [016: Layout sweep](016-layout-sweep.md), [018: Case settings, part 2: mounts, openings, battery, manufacturing and pairs](018-case-settings-2.md), [019: Case rest](019-case-rest.md), [020: Parts library and inspector](020-parts-library-inspector.md), [021: Parts editors](021-parts-editors.md), [022: PCB, Keymap and Keycaps](022-pcb-keymap-keycaps.md), [023: Library and Export](023-library-and-export.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [branch plan](../../rust-ui-migration/branch-plan.md), [ticket 005](005-old-css-check.md), [shared contract](../map.md#shared-contract)

## What to build

The `Board*` wrappers and the legacy styling are gone, because no caller remains.

## Acceptance criteria

- [ ] `Board*` and its module are deleted; every `.board-*`, `.m1-*` rule and `--wb-*` variable is removed, and layout rules that were not component styling live in the new styling
- [ ] The old-CSS check in strict mode passes
- [ ] The full build and the native, browser and page checks pass; the native test-name baseline is updated deliberately
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Current

2026-11-05: implementation is committed through `d485094d`; **hold claimed,
do not merge** pending coordinator adjudication and the required combined
Spec + Standards review. The three merged sub-lanes migrated Shell/Case,
Layout, and PCB/Keymap/Keycaps/Parts/Library/shared docks respectively; their
per-area evidence and needs-home assessments remain in
`.evidence/024/sub-{a,b,c}-{outcome,needs-home}.md`. All three reported no
unmappable styling rules.

- Integration retired the remaining old rules and variables, moved the live
  document defaults, fonts and finding outlines to `workbench-base.css`,
  and preserved coarse-pointer Library controls and the hidden Export footer.
  The project-menu close and focus path uses a semantic data hook; its mounted
  regression went red then green and both close/focus mutations were killed.
  The final class sweep found only legacy **ID** references inside utility
  selectors, not legacy class tokens; IDs and ARIA targets are intentionally
  out of scope and remain unchanged.
- `python3 scripts/check-old-css.py --strict` reports **0 selectors / 0
  variables**. `check.py build`, `check.py lint`, and `check.py test` pass;
  all 2,110 baselined native test names ran, with no rename or baseline update.
  The affected eight-crate browser subset and 51 page tests passed, including
  the paired Layout findings test. The Project menu, Library/Export, Shell,
  Parts preview, Layout adoption/chooser/field/point/view workflows and focused
  finding styles also passed. Logs are in `.evidence/024/integrator-*.log`.
- The 96-view before/after matrix covers eight surfaces, both Themes, both
  Appearances, and 1440/960/390px in `web/target/test-results/024/{before,after}/`;
  comparisons are logged in `.evidence/024/integrator-visual-compare.log`.
  The Split 42 demo's Case preview cannot prepare a 3D scene because its demo
  switch lacks a courtyard, so that matrix does **not** establish visual
  equivalence of the populated Case canvas. Library captures set Theme
  attributes directly; the workspace captures reload saved preferences.
- The one default full-gate probe at clean `d485094d` stopped at the tooling
  registration guard: 21 migration test scripts remain unregistered
  (`.evidence/024/integrator-full-gate.log`,
  `web/target/test-results/024/full-receipt.json.md`). This is the signature
  adjudicated in [022 Comments](022-pcb-keymap-keycaps.md#comments); the
  registration guard itself is unchanged from the feature tip. Fail-fast means
  the default probe did not run the remaining planned steps.
- A separate full-browser run at `374afdc5` executed all launches: all crate
  tests and the complete 91-test page suite passed, as did the CAD smokes; drag
  initially failed on newly retired scene classes. Those scene selectors were
  migrated in `d485094d`, and a focused drag recheck now reaches the
  `.m1-save-state` timeout previously recorded in [022 Comments](022-pcb-keymap-keycaps.md#comments)
  and [023 Outcome](023-library-and-export.md#outcome). The save-query function
  is unchanged from the feature tip, but **the same drag script file is
  changed**, so the requested unchanged-path condition is not met and this
  needs adjudication. No drag was exercised past that wait. Collision and
  qualification fixtures still contain retired class selectors beyond the
  blocked path; the historical opt-in Graphite script also retains them.
  The old ToastCtx and dock-page failures did not recur in the complete page
  suite; Layout fixture and dock files do differ from the feature tip, and a
  dock-page adjudication record was not found in the cited 022/023 records.
- The reference audit is `.evidence/024/integrator-selector-reference-audit.log`;
  screenshots and receipt details are under `web/target/test-results/024/`.
  The requested test-granularity research document was absent from this
  checkout and the inspected feature tip. Final human visual approval and the
  independent combined review remain open.
