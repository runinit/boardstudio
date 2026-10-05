# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ 8fffe72c
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 58/60 accepted, 1 implementing, 1 planned
- Criteria: unassessed 2, missing 1, implemented 0, verified 218; 221 total
  - functional: verified 181
  - release: missing 1, unassessed 2, verified 29
  - visual: verified 8

## Served candidate
- Build: frontend-layout-pointer-cache-20261005
- Source commit: f956c0dfe9ec942cb79b9905565e9c404fb96732
- Root: http://127.0.0.1:34830/
- Subpath: http://127.0.0.1:34830/boardstudio/

## Functional readiness
- Ready functional criteria: 0 investigate, 0 implement, 0 qualify (Layout 0 investigate/0 implement/0 qualify; PCB 0 investigate/0 implement/0 qualify; Keymap 0 investigate/0 implement/0 qualify; Keycaps 0 investigate/0 implement/0 qualify; Case 0 investigate/0 implement/0 qualify; Parts+Project 0 investigate/0 implement/0 qualify; Shared 0 investigate/0 implement/0 qualify)
- Functionally verified parents awaiting formal acceptance: 0
- Parents with unmet final joins: 0
  Layout: unassessed 0, missing 0, implemented 0, verified 27; ready investigate 0, implement 0, qualify 0
  PCB: unassessed 0, missing 0, implemented 0, verified 26; ready investigate 0, implement 0, qualify 0
  Keymap: unassessed 0, missing 0, implemented 0, verified 7; ready investigate 0, implement 0, qualify 0
  Keycaps: unassessed 0, missing 0, implemented 0, verified 13; ready investigate 0, implement 0, qualify 0
  Case: unassessed 0, missing 0, implemented 0, verified 39; ready investigate 0, implement 0, qualify 0
  Parts+Project: unassessed 0, missing 0, implemented 0, verified 39; ready investigate 0, implement 0, qualify 0
  Shared: unassessed 0, missing 0, implemented 0, verified 30; ready investigate 0, implement 0, qualify 0
Functional criteria verified; formal acceptance still outstanding:
  none
Exact unmet final joins:
  none

## Parents
- Accepted (58): INT.1, INT.2, BND.1, BND.2, F2.1, F2.2, F2.3, F2.4, F3.1, F3.2, F3.3, F3.4, F3.5, F3.6, F3.7, F3.8, F4.1, F4.2, F4.3, F4.4, F4.5, F4.6, F5.1, F5.2, F5.3, F5.4, F5.5, F5.6, F5.7, F5.8, F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5, F6.6, F7.1, F7.2, F7.3, F7.4, F7.5, F7.6, F7.7, F7.8, F8.1, F8.2, F8.3, F8.4, F8.5, F8.6, F9.1, F9.2, F9.4, F9.5
- Hold (0): none

## Criteria on hold or blocked
- F9.7-C01 [unassessed]: After the concrete F9.6 patch is reviewed, obtain explicit approval for actual cutover, then qualify launch/compatibility/critical journey.
- F9.7-C02 [unassessed]: Retain React reference and rollback until adoption approval; retire only inventory-covered entrypoints after approved cutover.

## Open RF findings (34)
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
- RF-034: Frontend artifact switches leave the prior service worker controlling the shell

## Qualification scope
- Scope: f86-integrated-project-export-20261005 - Paired six-workbench project-to-export journey
- Journeys: six-workbench-project-export
- State: qualifying

Last retro checkpoint: 2026-10-04T15:56:00Z

Latest milestone: 8fffe72c commits F9.5 performance acceptance, current 63-row inventory and reviewed RF reconciliation. Candidate remains immutable f956c0df at34830. All functional workflows are accepted; Undo/Redo qualification/repairs, mobile and accessibility remain excluded. Unrelated six staged entries in /tmp/case-layout-unrelated-index.json preserved.

F9.6-C02 now has a real same-origin release failure: run-f956-20261005-08 imported/saved/reloaded copied React project successfully, then artifact switch left old cache-first /sw.js controlling the React shell. Candidate /sw.js404; no /service-worker.js request. No candidate import or rollback attempted. RF-034 and pending_source_batch record the required repair. F9.6-C01/C03 verified; F9.7 remains separately approved adoption, not yet applied.

Source author migration_acceleration_review leases scripts/web/embed-worker-wasm.mjs, new service-worker-handoff.js, stage-rollback.mjs, test-service-worker-handoff.mjs, plus build-m1.py and affected test-build-m1-reuse.py manifest fixtures. Minimal candidate legacy-classic-worker handoff plus copied React rollback overlay; no store/cache clearing or original reference changes. Sole reviewer preset_batch_review confirmed minimum remedy and awaits settled diff. Root owns integration/package/publication.

case_local_export owns rehearsal folder/continuation only. Existing isolated browser session f96-cutover-164703-3448630 remains alive; old proxy34831 owner Python3448630 was SIGKILLed after identity check without browser cleanup, freeing34831. Author verified the original browser controller/store/cache/project remain intact. Continuation will bind the same origin to new package/rollback endpoints, then actual forward→rollback→forward after publication. No new browser transitions before repair. Source candidate34830 and read-only reference5175 stay available.
