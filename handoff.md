# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ f4107630
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 60/60 accepted, 0 implementing, 0 planned
- Criteria: unassessed 0, missing 0, implemented 0, verified 221; 221 total
  - functional: verified 181
  - release: verified 32
  - visual: verified 8

## Served candidate
- Build: start-f4107630-20261005T181117Z-3535061
- Source commit: f41076301c592cf7145fbd2125bf7de00e4e7b72
- Root: http://127.0.0.1:4173/
- Subpath: http://127.0.0.1:4173/boardstudio/

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
- Accepted (60): INT.1, INT.2, BND.1, BND.2, F2.1, F2.2, F2.3, F2.4, F3.1, F3.2, F3.3, F3.4, F3.5, F3.6, F3.7, F3.8, F4.1, F4.2, F4.3, F4.4, F4.5, F4.6, F5.1, F5.2, F5.3, F5.4, F5.5, F5.6, F5.7, F5.8, F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.3, F6C.4, F6C.5, F6.6, F7.1, F7.2, F7.3, F7.4, F7.5, F7.6, F7.7, F7.8, F8.1, F8.2, F8.3, F8.4, F8.5, F8.6, F9.1, F9.2, F9.4, F9.5, F9.6, F9.7
- Hold (0): none

## Criteria on hold or blocked
- none

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
- Scope: f97-approved-default-adoption-20261005 - Approved default Dioxus launch and React entrypoint retirement
- Journeys: adopted-default-project-journey
- State: qualified

Last retro checkpoint: 2026-10-04T15:56:00Z

Migration complete within approved scope. User approved cutover/decommission on2026-10-05. Sourcef4107630 changes default start/dev/build/Pages/checks to Dioxus; explicit :react reference/rollback commands and shared providers retained. Actual pnpm start full package start-f4107630-20261005T181117Z-3535061 passed and remains served at4173 root/subpath (execsession40201). Package1412sources/191assets each route, zero mismatches/warnings. Adopted public copied-project import/rename/save/reload/sixpanels/export passed; all other fields/six asset hashes preserved. Sole independent review CLEAR.

No push or hosted Pages deployment was performed. Broader repo-check retains pre-existing unused generator export and archived-link failures, documented in f97 receipt; do not claim full pnpm check passed. Undo/Redo qualification/repairs, mobile/accessibility, unfinished VIK and visual polish remain excluded/deferred per user scope. RF structural follow-ups remain separate. Reference5175 and copied rollback34835 preserved; actual F97 browser profile remains open. Six unrelated staged entries preserved per /tmp/case-layout-unrelated-index.json.
