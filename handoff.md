# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ f956c0df
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 58/60 accepted, 1 implementing, 1 planned
- Criteria: unassessed 3, missing 0, implemented 0, verified 218; 221 total
  - functional: verified 181
  - release: unassessed 3, verified 29
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
- Scope: f86-integrated-project-export-20261005 - Paired six-workbench project-to-export journey
- Journeys: six-workbench-project-export
- State: qualifying

Last retro checkpoint: 2026-10-04T15:56:00Z

Milestone: source f956c0df, immutable frontend-layout-pointer-cache-20261005 at34830. Independent review accepted F9.5 after actual five paired sessions: all30scenarios/100samples each meet33/50/100ms caps; median candidate p95 28.055/43.435/93.305ms. Source gates native248+1ignored, page, reachability, strict3WASM pass; exact receipts reused at guarded commit. No active performance/build job. Unrelated six staged entries preserved exactly.

58/60 parents accepted; F9.6 implementing/F9.7 planned. C01 inventory reviewed and verified; authoritative tsx-inventory.json now has current63-row scoped responsibility mappings with stale prior fields preserved historically. No new product failure is inferred from old unassessed notes. Source/test deletion and excluded UndoRedo/mobile/a11y/visual checks are not claimed.

case_local_export is executing required copied-data same-origin cutover/rollback rehearsal under evidence/f96-adoption-20261005/rehearsal only. React uses boardstudio-v2 store/active key and sw.js; Dioxus uses scoped boardstudio-m1 stores and service-worker.js. Archive transfer is the known compatibility boundary, no automatic IndexedDB migration. Preserve isolated profile/store/worker caches across controlled proxy switches to expose actual stale-shell problems; never preemptively unregister or clear. Source/publications unchanged; no external deploy. If natural worker transition fails, capture one realfailure before any repair. Sole reviewer preset_batch_review cleared performance/C01/entrypoint proposal; C03 final register independently CLEAR and criterion verified; C02 actual rehearsal remains. Default package/Pages patch is still unapplied; final explicit cutover approval remains separate. F95 acceptance/current inventory/RF notes and final measurement artifacts are uncommitted for next milestone. Rehearsal proxy HTTP framing and reference activation are corrected; preserve failed setup attempts as harness-only. Author now uses explicit imported-project readiness and must keep the session alive for selector recovery. No valid upstream switch has occurred yet.
