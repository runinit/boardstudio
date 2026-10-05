# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ 38641dd1
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 40/61 accepted, 11 implementing, 10 planned
- Criteria: unassessed 24, missing 2, implemented 16, verified 180; 222 total
  - functional: implemented 6, missing 1, unassessed 3, verified 171
  - release: implemented 7, missing 1, unassessed 19, verified 6
  - visual: implemented 3, unassessed 2, verified 3

## Served candidate
- Build: frontend-routed-case-cancellation-20261005
- Source commit: 38641dd1353bcfb6230d52c1d5b84c7ad7908e7a
- Root: http://127.0.0.1:34823/
- Subpath: http://127.0.0.1:34823/boardstudio/

## Functional readiness
- Ready functional criteria: 1 investigate, 1 implement, 6 qualify (Layout 0 investigate/0 implement/0 qualify; PCB 0 investigate/1 implement/0 qualify; Keymap 0 investigate/0 implement/0 qualify; Keycaps 0 investigate/0 implement/0 qualify; Case 0 investigate/0 implement/0 qualify; Parts+Project 0 investigate/0 implement/5 qualify; Shared 1 investigate/0 implement/1 qualify)
- Functionally verified parents awaiting formal acceptance: 4
- Parents with unmet final joins: 8
  Layout: unassessed 0, missing 0, implemented 0, verified 27; ready investigate 0, implement 0, qualify 0
  PCB: unassessed 0, missing 1, implemented 0, verified 25; ready investigate 0, implement 1, qualify 0
    Implement: F5.7-C04
  Keymap: unassessed 0, missing 0, implemented 0, verified 7; ready investigate 0, implement 0, qualify 0
  Keycaps: unassessed 0, missing 0, implemented 0, verified 13; ready investigate 0, implement 0, qualify 0
  Case: unassessed 0, missing 0, implemented 0, verified 39; ready investigate 0, implement 0, qualify 0
  Parts+Project: unassessed 0, missing 0, implemented 5, verified 34; ready investigate 0, implement 0, qualify 5
    Qualify: F2.2-C03, F4.1-C02, F4.3-C02, F4.4-C01, F4.5-C03
  Shared: unassessed 3, missing 0, implemented 1, verified 26; ready investigate 1, implement 0, qualify 1
    Investigate: F8.6-C03
    Qualify: F6.6-C04
Functional criteria verified; formal acceptance still outstanding:
  F2.4: visual/release F2.4-C04
  F4.6: joins F4.4
  F7.8: joins F4.4; visual/release F7.8-C02, F7.8-C03, F7.8-C04
  F8.3: joins F4.3
Exact unmet final joins:
  F4.6 → F4.4
  F7.8 → F4.4
  F8.3 → F4.3
  F8.6 → F2.2, F3.7, F4.5, F4.6, F5.8, F6.6, F7.8
  F9.2 → F2.2, F2.4, F3.7, F4.5, F4.6, F5.8, F6.6, F7.8, F8.6
  F9.3 → F9.2
  F9.4 → F8.6
  F9.5 → F9.2

## Parents
- Accepted (40): INT.1, INT.2, BND.1, BND.2, F2.1, F2.3, F3.1, F3.2, F3.3, F3.4, F3.5, F3.6, F3.8, F4.2, F5.1, F5.2, F5.3, F5.4, F5.5, F5.6, F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5, F7.1, F7.2, F7.3, F7.4, F7.5, F7.6, F7.7, F8.1, F8.2, F8.4, F8.5
- Hold (0): none

## Criteria on hold or blocked
- F5.8-C05 [unassessed]: Record both named viewer integration joins after their current provider capabilities are qualified.
- F8.6-C01 [unassessed]: Join six-workbench project-to-export journey after each required provider/consumer acceptance is ready.
- F8.6-C02 [unassessed]: Combine existing bounded receipts with remaining authored/generated/portable outputs and per-operation snapshot proofs.
- F8.6-C03 [unassessed]: Run focused coordinator races/failure/retry/cleanup and route return on the frozen joined candidate.
- F9.2-C01 [unassessed]: Complete per-workflow and aggregate paired qualification after required workflow joins; reuse only attributable unchanged evidence.
- F9.4-C01 [unassessed]: Run cross-frontend archive round-trip and actual storage-write checks on final routes.
- F9.5-C01 [unassessed]: Map each existing budget to eligible current evidence; preserve ineligible/failed baselines for applicability review.
- F9.5-C02 [unassessed]: Qualify applicable interaction/rendering performance on final integrated source.
- F9.5-C03 [unassessed]: Attribute worker, URL and resource cleanup to final output paths; leave unmeasured/ineligible checks open.
- F9.6-C01 [unassessed]: After F9.1–F9.5 and F2–F8 exits, reconcile every remaining placeholder/React island against inventory.
- F9.6-C02 [unassessed]: Prepare reproducible candidate provenance plus copied-data cutover and rollback dry-run after qualification gates.
- F9.7-C01 [unassessed]: After the concrete F9.6 patch is reviewed, obtain explicit approval for actual cutover, then qualify launch/compatibility/critical journey.
- F9.7-C02 [unassessed]: Retain React reference and rollback until adoption approval; retire only inventory-covered entrypoints after approved cutover.

## Open RF findings (33)
- RF-001: Shared presentation and Runtime are integration hotspots
- RF-002: Internal browser host types are exposed as crate APIs
- RF-003: CAD engine capabilities and host protocols drift apart
- RF-004: Immutable export tokens do not model export-owned commits
- RF-005: Geometric edit planning lives in frontend helper policy
- RF-006: Canonical, physical-instance and isolated sample scopes are easy to conflate
- RF-007: Runtime observation currently supports one subscriber
- RF-008: Archive packing capability is split from its UI options and asset resolution
- RF-009: Parity accounting and acceptance evidence are scattered
- RF-010: Cancellation has different guarantees at worker and kernel boundaries
- RF-011: CAD revision envelope has a JavaScript safe-integer ceiling
- RF-012: Renderer host relies on reflective method names and partial capability wrappers
- RF-013: Object tree containers have invalid required-child semantics
- RF-014: Host callback ownership must survive browser terminal events
- RF-015: Shared viewer controls can read a different source than the rendered scene
- RF-016: Rapid reference form edits can display values that were not accepted
- RF-017: Import completion can outlive the selected definition in the TypeScript reference
- RF-018: Canvas draft input and outline geometry use implicit competing coordinate frames
- RF-019: Layout 3D scene projection omits mounted module bodies
- RF-020: Imported project definitions leak into reusable Parts choices
- RF-021: Saved outline entity IDs depend on a page-local operation counter
- RF-022: Native frontend test harness duplicates the production presentation module tree
- RF-023: Worker-owned provider failures lack a deterministic public test seam
- RF-024: PCB finding overlay advertises interaction that its pointer policy disables
- RF-025: Case gesture ownership conflates accepted source with provisional render identity
- RF-026: Dioxus delegated pointer handlers cannot assume a DOM currentTarget
- RF-027: Generator validation exposes raw JavaScript stack text in the Parts UI
- RF-028: Selection liveness conflates a present context with a resolved target
- RF-029: Layout keyboard gestures depend on SVG focus rather than workbench scope
- RF-030: Dynamic select options can display defaults over retained accepted values
- RF-031: Mechanical profile targets and family defaults are split across presentation and controller rules
- RF-032: PCB-coupled Case readiness prevents authored-body preview and STEP delivery
- RF-033: 63 plain #[test]s in wasm-only presentation modules never run natively; one pre-existing mounted ...

## Qualification scope
- Scope: frontend-case-export-20261005 - Complete desktop Case generation and local export journey
- Journeys: case-local-step-scope
- State: qualifying

Last retro checkpoint: 2026-10-04T15:56:00Z

Combined gate3074104 TERMINAL0: native247/1ignored, page/reachability, strictheadless54/0fail/0excluded/0incomplete939.96s. All11frozenhashes intact;6unrelatedstagedidentities preserved. F5.1accepted andF3.7C04verified from same consolidatedreview. F2longname/System/guide-panel focus public observations now viewer-actions shell-* receipts; reviewer checking exactclause. Source integration commit then immutablepackage next, no newpublic GREEN claimed. Corrected Layout/PCB inventory proposal evidence-only in progress; no compiler/sourcewrites.
