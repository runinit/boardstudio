# Handoff (generated from records)

- Branch: codex/rust-v1-ui-parity-20261001 @ 3a412978
- Worktree: /home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001
- Parents: 22/61 accepted, 28 implementing, 11 planned
- Criteria: unassessed 27, missing 1, implemented 80, verified 114; 222 total
  - functional: implemented 67, missing 1, unassessed 3, verified 110
  - release: implemented 10, unassessed 19, verified 4
  - visual: implemented 3, unassessed 5

## Served candidate
- Build: frontend-parts-library-repairs-20261004
- Source commit: 3a412978cee729a24d37e7e55e7c3664f1187fb2
- Root: http://127.0.0.1:34822/
- Subpath: http://127.0.0.1:34822/boardstudio/

## Parents
- Accepted (22): INT.1, INT.2, BND.1, BND.2, F2.1, F2.3, F3.1, F3.2, F3.3, F3.4, F4.2, F5.2, F5.3, F6K.1, F6K.2, F6K.3, F6K.4, F6C.1, F6C.2, F6C.4, F7.2, F8.2
- Hold (0): none

## Criteria on hold or blocked
- F5.6-C05 [implemented]: Valid unflipped Case viewer proof retained. Changed Left/Right flips accepted but current candidate generation hit auto-closure envelope errors and correctly displayed Previous geometry; this does ...
- F5.7-C04 [implemented]: The shared-viewer projection now forwards the accepted enabled BoardReference pose/elevation; a nonidentity packet regression was RED before and GREEN after. On one imported routed-board archive, p...
- F5.8-C05 [unassessed]: Record both named viewer integration joins after their current provider capabilities are qualified.
- F8.3-C03 [implemented]: Qualify failed-package/protection and stale owner behavior, plus exact PCB/outline blocker navigation.
- F8.5-C02 [implemented]: Run paired public generated mechanical package output when F7 readiness/generation joins are satisfied.
- F8.5-C03 [implemented]: Pair authored canonical and generated selected-instance paths, including switch during pending delivery; retain Case-local geometry entrypoint.
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
- Scope: frontend-parts-library-repairs-20261004 - Scoped assembly editing, shared-viewer consumption and durable project continuity
- Journeys: F7.3-parts-individual-model-layers, F2.2-library-mounted-recovery, saved-recipe-layout, layout-keymap-keycaps, pcb-model-viewer, case-local-step-scope
- State: repairing

Last retro checkpoint: 2026-10-04T15:56:00Z

## Prior session notes (historical)
- **Primary checkout `dev` is fast-forwarded to `df2abad6`** (local only, no push). Later commits (33390d6c, tooling) exist only on `codex/rust-v1-ui-parity-20261001`; re-run the fetch + `merge --ff-only` from `/home/chris/01_Projects/ts-boardstudio2` when you want dev current (4 untracked files may collide; compare, remove identical ones first).
- **Latest candidate:** `frontend-review-fixes-20261004` served on 34820 (`python3 scripts/serve-candidate.py <build> <port>`, detached). It has the mirrored-pair fixes and Layout fixes; later test-conversion commits are test-only except `renderer_host*.rs` (no behavior change).
- **In flight:** one Sonnet agent was wiring `scripts/run-wasm-tests.py` + native/wasm test gates into `migration-deliver.py commit` (files: scripts/run-wasm-tests.py, test-run-wasm-tests.py, wasm-known-failures.json, migration-deliver.py, test-migration-deliver.py). Check `git status`; verify with `python3 scripts/test-migration-deliver.py` and commit the exact files if tests pass.
- **Next work:** (1) F3.5-C05 mirrored-pair check in the browser: on a linked pair made via Add object -> Mirror existing half, the Dioxus mirror-target key already shows "Use mirrored components" on a fresh pair, so `assembliesLocal` may be true from creation; compare with the TypeScript reference (:5175) before changing anything. (2) Grouped F3.5/F3.6 Layout journey (owner tests now cover stale finding, locked/driven X/Y, zoom limits, selected-board 3D, renderer lifecycle; browser still needed for Inspector contexts, keyboard/compact drawers, 3D pick/orbit, repeated 2D/3D switching). (3) More Opus diff reviews per batch before acceptance; parent acceptance via `progress.py set-status ... --review`.
- **Known failing/ignored:** RF-033: 4 wasm tests are known failures (3 catalogue tests need BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL; `mounted_component_drafts_survive_unrelated_acceptance_and_blur_uses_latest_owner` fails).
- **Traps:** run `build-m1.py` outside the sandbox as a tracked background task (wasm-pack needs a writable cache); publish a candidate BEFORE agents edit source (publish checks current source hashes); Agent `isolation: worktree` fails (no origin/main); presentation code is wasm-only so run the wasm `page` check and the headless-Chrome tests; Chrome test runs need the sandbox off; `pkill -f` matches its own shell.
- **Token rules (see CONSTRAINTS.md):** restart sessions at ~150k context with this file; agent reports <=150 words with detail in a file; no screenshots or `innerText` dumps unless a visual check needs them; pipe build/test output through tail/grep.


## Current continuation notes — grouped Layout completion
- Published full candidate **frontend-layout-grouped-repairs-20261004**, source **f320e6b8**, root **http://127.0.0.1:34821/** and `/boardstudio/`. Detached serve PID 2032771; old 34820 and reference 5175 remain. Root/subpath package proof passes with no source mismatches.
- Requested gate tooling/AGENTS batch committed in 97926b0e; preserve external 8b9f8335; root-path repair faa10f38; settled Layout and file-selected isolation repair f320e6b8. Python suites: 20 runner and 26 delivery tests pass. Enforced native/page/reachability and 267 listed wasm outcomes across 64 isolated modules completed. **Configured 13 failure exclusions are not passes; 9 external additions remain unreviewed.** RF-033 reachability baseline is now 0; assertion failures remain open.
- Fresh mirror creation is already `assembliesLocal=true` in both frontends. Do not “fix” that shared behavior. Grouped paired receipt covers Inspector/compact keyboard, numeric drafts, actual 3D orbit/pick and repeated switching. Reproduced mirrored-key disable→Undo selection loss received a real Core/Session RED-first repair; mirrored and ordinary keys now restore selection/Fit/relation. Target Choc/diode history and reload pass without changing canonical components.
- Independent native reviews are clear for source and scoped replay. See `.scratch/dioxus-frontend-v1/evidence/layout-grouped-20261004/RECEIPT.md` and `layout-grouped-review-20261004.md`. **Still 20/61 accepted**: F3.5/F3.6 remain open for recorded context/lifecycle/fault-observation limits and F7.3 join. Unassessed F8/F9 criteria were untouched.
- Canonical tool is **`.scratch/dioxus-frontend-v1/progress.py`**; `scripts/progress.py` does not exist. No `.codegraph` exists in this checkout, despite stale AGENTS wording. Do not reset unrelated staged/untracked evidence. No push or branch switch was performed.


## Current continuation notes — consolidated34822 milestone
- Source reductions and Parts model repair committed4812d82b; successful enforcing gate recorded3a412978. Native/page/reachability/changed-headless modules passed with five library exclusions removed, eight remain. Astra xhigh review frozen; F6K.4 andF5.3 closures bring22/61accepted.
- Published frontend-parts-library-repairs-20261004 on34822 (source3a412978), root/subpath proof verified, detachedPID2135651. Reuse this single candidate for remaining journeys. Consolidated receipt: .scratch/dioxus-frontend-v1/evidence/consolidated-34822-20261004/RECEIPT.md.
- F2.2-C04 andF7.3-C03 nowverified from bounded attributed evidence. Paired recipe default models/UndoRedo/board150,10placement/re-editX4independence/reload pass. MatrixApply/import/parameter/identity branches remain. Selector-originated3DKeymap A andKeycapsDSA/legendQ edits pass; actual3Dmesh-pick remains unqualified.
- Candidate library keeps Grouped journey Sofle 20261004 with recipes and a separate fresh Sofle for validCase. FreshSoflePlate+mechanicalstack reaches exactgeometry; wall2.2andphysicalscopechange observedGenerating/Previous/currentrecovery. Configured Case lacks local Export geometry, reference mounts it. Authored-only context also lacks the reference local action, so do not widen the discrepancy.
- Native subagentcase_local_export is READ-ONLY pending user clarification: native tests cannot observe this missing wasm-onlyDOMbutton. Asyncquestion requests mountedChromeREDfirst exception; no production edit authorized under that exception until reply. Do not manufacture native failures. Proposed ownedsourcecad_presentation.rs; existing Runtime::export_mechanical must be reused with exactcurrentinstance/snapshotguards.
- No running shellheavyjob; no push/switch. Six preexistingstaged evidence files remain unrelated. Preserve them. Native browserbinding exists; fresh tabs werecandidate8/reference7 this turn, but listtabs before relying on them.
