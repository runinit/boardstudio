# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ 55790e0a
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 49/61 accepted, 3 implementing, 9 planned
- Criteria: unassessed 21, missing 2, implemented 7, verified 192; 222 total
  - functional: missing 1, unassessed 3, verified 177
  - release: implemented 7, missing 1, unassessed 16, verified 9
  - visual: unassessed 2, verified 6

## Served candidate
- Build: frontend-case-parts-cap-selection-20261005
- Source commit: 55790e0a9c30b3e53cbd9690dc418fec23021d11
- Root: http://127.0.0.1:34824/
- Subpath: http://127.0.0.1:34824/boardstudio/

## Functional readiness
- Ready functional criteria: 2 investigate, 1 implement, 0 qualify (Layout 0 investigate/0 implement/0 qualify; PCB 0 investigate/1 implement/0 qualify; Keymap 0 investigate/0 implement/0 qualify; Keycaps 0 investigate/0 implement/0 qualify; Case 0 investigate/0 implement/0 qualify; Parts+Project 0 investigate/0 implement/0 qualify; Shared 2 investigate/0 implement/0 qualify)
- Functionally verified parents awaiting formal acceptance: 1
- Parents with unmet final joins: 5
  Layout: unassessed 0, missing 0, implemented 0, verified 27; ready investigate 0, implement 0, qualify 0
  PCB: unassessed 0, missing 1, implemented 0, verified 25; ready investigate 0, implement 1, qualify 0
    Implement: F5.7-C04
  Keymap: unassessed 0, missing 0, implemented 0, verified 7; ready investigate 0, implement 0, qualify 0
  Keycaps: unassessed 0, missing 0, implemented 0, verified 13; ready investigate 0, implement 0, qualify 0
  Case: unassessed 0, missing 0, implemented 0, verified 39; ready investigate 0, implement 0, qualify 0
  Parts+Project: unassessed 0, missing 0, implemented 0, verified 39; ready investigate 0, implement 0, qualify 0
  Shared: unassessed 3, missing 0, implemented 0, verified 27; ready investigate 2, implement 0, qualify 0
    Investigate: F8.6-C02, F8.6-C03
Functional criteria verified; formal acceptance still outstanding:
  F7.8: visual/release F7.8-C02, F7.8-C03, F7.8-C04
Exact unmet final joins:
  F8.6 → F3.7, F5.8, F7.8
  F9.2 → F3.7, F5.8, F7.8, F8.6
  F9.3 → F9.2
  F9.4 → F8.6
  F9.5 → F9.2

## Parents
- Accepted (49): INT.1, INT.2, BND.1, BND.2, F2.1, F2.2, F2.3, F2.4, F3.1, F3.2, F3.3, F3.4, F3.5, F3.6, F3.8, F4.1, F4.2, F4.3, F4.4, F4.5, F4.6, F5.1, F5.2, F5.3, F5.4, F5.5, F5.6, F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5, F6.6, F7.1, F7.2, F7.3, F7.4, F7.5, F7.6, F7.7, F8.1, F8.2, F8.3, F8.4, F8.5
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

Requiredfindingsgate3111403 nowTERMINAL0: native247/1ignored,page/reachability,strictheadless4/0fail/0excluded/incomplete96.13s. Sourcehashes intact, same soleindependentreviewCLEAR. Explicitownedcommitmanifest /tmp/case-findings-bounds-commit-paths.json;6unrelatedindexentries preserved. Commit/build/publicexpandedfindingsGREEN next; no wholeCaseacceptancebeforepublicresult.
