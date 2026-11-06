# 024: Delete the old layer

Status: resolved
Owner: lane/024-delete-the-old-layer (.worktrees/lane-024-delete-the-old-layer)
Type: build
Blocked by: [014: Shell glue sweep](014-shell-glue-sweep.md), [016: Layout sweep](016-layout-sweep.md), [018: Case settings, part 2: mounts, openings, battery, manufacturing and pairs](018-case-settings-2.md), [019: Case rest](019-case-rest.md), [020: Parts library and inspector](020-parts-library-inspector.md), [021: Parts editors](021-parts-editors.md), [022: PCB, Keymap and Keycaps](022-pcb-keymap-keycaps.md), [023: Library and Export](023-library-and-export.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [branch plan](../../rust-ui-migration/branch-plan.md), [ticket 005](005-old-css-check.md), [shared contract](../map.md#shared-contract)

## What to build

The `Board*` wrappers and the legacy styling are gone, because no caller remains.

## Acceptance criteria

- [x] `Board*` and its module are deleted; every `.board-*`, `.m1-*` rule and `--wb-*` variable is removed, and layout rules that were not component styling live in the new styling
- [x] The old-CSS check in strict mode passes
- [ ] The full build and the native, browser and page checks pass; the native test-name baseline is updated deliberately (full browser remains red on the adjudicated pre-existing drag timeout; see Outcome)
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Current

Resolved; see [Outcome](#outcome) and the coordinator's [Comments](#comments).

## Outcome

2026-11-05: the old `Board*` layer and legacy styling were retired; the three
sub-lanes reported no unmappable styling rules. Per-area records and needs-home
assessments are `.evidence/024/sub-{a,b,c}-{outcome,needs-home}.md`:

- **Shell and Case (A):** migrated shell, canvas, inspector, export and
  workspaces plus Case viewer, bodies, assembly layers and mechanical settings
  to existing utilities and Theme tokens. Dock overflow preserves the upward
  popup placement; fixture stylesheet and semantic/geometry selectors were
  updated to exercise the migrated surfaces.
- **Layout (B):** migrated canvas overlays, objects, inspectors, setup and
  fixtures, including SVG, touch and form placement styling. The paired finding
  selectors were integrated with A and the previously blocked page test passed.
- **PCB, Keymap, Keycaps, Parts, Library and shared docks (C):** migrated
  wiring, inspectors, SVG scenes, previews, Library and docks. Integration
  replaced the project-menu class dependency with a semantic data hook; its
  mounted close/focus regression went red then green, and both mutations were
  killed (`.evidence/024/integrator-menu-mutations.log`).
- **Integration:** removed remaining old rules and variables, moved live
  document defaults, fonts and finding outlines to `workbench-base.css`, and
  preserved coarse-pointer Library controls and the hidden Export footer. The
  final class sweep found legacy **ID** references inside utility selectors,
  not legacy class tokens; stable IDs, ARIA targets and domain IDs are outside
  the styling migration and intentionally remain unchanged. Reference audit:
  `.evidence/024/integrator-selector-reference-audit.log`.

### Verification and adjudicated gates

- `python3 scripts/check-old-css.py --strict`: **0 `.board-*`/`.m1-*`
  selectors / 0 `--wb-*` variables** (`.evidence/024/integrator-strict.log`).
- `python3 scripts/check.py build`, `lint`, and `test` passed; all **2,110**
  baselined native test names ran. No test was renamed, so no baseline update
  was needed (`.evidence/024/integrator-{base-build,lint-final,native-final}.log`).
- The affected eight-crate browser subset passed, as did **51** affected page
  tests including paired Layout findings (`.evidence/024/integrator-affected-{browser,page}.log`).
  Project menu, Library/Export, Shell, Parts preview, Layout workflows and
  focused finding styles also passed. A separate full-browser run at
  `374afdc5` ran all launches: crate tests, CAD smokes and the complete **91**
  page tests passed, but drag failed on retired scene selectors
  (`.evidence/024/integrator-full-browser.log`). Scene selectors were migrated
  in `d485094d`; the focused recheck reached the stale `.m1-save-state` timeout
  and did not exercise drag beyond that wait (`.evidence/024/integrator-drag-recheck.log`).
  The coordinator adjudicated this timeout pre-existing and cleared 024; see
  [Comments](#comments). The browser acceptance checkbox remains unchecked
  because the full browser gate is not literally green.
- The default full-gate probe at `d485094d` stopped fail-fast at the unchanged
  tooling registration guard (21 unregistered migration scripts), before the
  remaining steps (`.evidence/024/integrator-full-gate.log`,
  `web/target/test-results/024/full-receipt.json.md`); this pre-existing
  signature is recorded in [022 Comments](022-pcb-keymap-keycaps.md#comments).
  The old ToastCtx and dock-page failures did not recur in the complete page
  suite; the Layout fixture and dock files differ from the feature tip, and a
  dock-page adjudication record was not found in the cited 022/023 records.
- The **96-view** before/after matrix covers eight surfaces, two Themes, two
  Appearances and 1440/960/390px: `web/target/test-results/024/{before,after}/`,
  with `.evidence/024/integrator-visual-compare.log`. Library captures set Theme
  attributes directly; workspace captures reload saved preferences. The Split
  42 demo switch lacks a courtyard, so the populated Case 3D canvas is not
  covered; this limitation was accepted in [Comments](#comments).

### Follow-ups

Modernize the drag save query and re-verify the full drag path; update the
collision/qualification fixtures and historical opt-in Graphite selectors.
Close the populated Case 3D visual-matrix gap and obtain final human visual
approval. The combined Spec + Standards review is deferred to the Phase 1
close-out review over base `6fc3956f5`, per [Comments](#comments). The
requested test-granularity research document was absent from this checkout and
the inspected feature tip.

## Comments

2026-11-05 (coordinator adjudication, drag save-state timeout): ruled PRE-EXISTING, not caused by 024. Evidence: the header save-state component renders `class: "header-save"` with NO `.m1-save-state` element identically on feat tip and lane (migrated by 010, long before 024); the drag script's save-query function (`document.querySelector('.m1-save-state')?.dataset.state === 'saved'`) is byte-identical on feat tip and lane; `d485094d` changed only scene selectors (9 lines: `.m1-scene-part`→`[data-geometry=scene-part]` etc.), never the save path. The 20s timeout is therefore deterministic and identical on both branches — a stale script selector from the 010 header migration, matching the signatures in 022 Comments, 023 Outcome, cad-backend-replacement/010 and the ci-wall-time baseline. The scene-selector updates in `d485094d` were owned fallout (024 retired those classes) and are verified working (recheck proceeds past scene interactions to the save wait). Follow-ups (NOT 024 gates): modernize the drag save query (`.header-save`/`[data-state]`, per the drag-smoke research note) and the collision/qualification + Graphite fixture selectors; then re-verify the full drag path. Visual-matrix limitation accepted and recorded: populated Case 3D canvas unverified (Split 42 demo switch lacks a courtyard — pre-existing demo limitation; WebGL canvas pixels are outside a class migration's blast radius; all 8 chrome surfaces covered 2 Themes × 2 Appearances × 3 widths). Combined Spec+Standards review deferred to the Phase 1 close-out review per the run brief (single review over base 6fc3956f5). 024 is cleared to resolve and fast-forward to feat.
