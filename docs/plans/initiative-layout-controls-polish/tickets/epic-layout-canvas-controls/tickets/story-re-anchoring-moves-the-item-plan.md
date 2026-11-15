---
title: 'Re-anchoring moves the item'
type: feature
ticket: 4
created: '2026-11-15'
status: built
baseline_revision: 'db8abb44a6a563a9052dbccdb604caf56d41f4c1'
route: full
route_source: auto
risk: high
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/AGENTS.md'
  - '{project-root}/README.md'
  - '{project-root}/docs/adr/0040-part-anchoring-column-vocabulary.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Picking another part anchor must retain carried/local values and move the item. Committed code implements basic movement, but ticket acceptance is unproven.

**Approach:** Preserve existing implementation, strengthen public Core and mounted Inspector coverage, fix demonstrated gaps, and verify the complete behavior.

## Boundaries & Constraints

**Always:** Retain Spread, Stagger, Splay, Offset and Rotation; place freshly from the new key and requested Follow subset. Preserve mirror mapping, matrix stored frames, refusal atomicity and one Undo. Follow-subset edits, part/matrix Detach and drop auto-anchor remain pose-preserving. Existing feedback owns `<item> anchored to <new anchor> (was <old anchor>) · 1 Undo step`.

**Never:** Start 1.7/1.8, revive canceled plans, duplicate feedback, weaken test oracles, change initiative configuration, or push. BMad governs review/verification; historical mutation and fixed-review mandates are retired.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Part Pick | R3C6, Spread 4, other values nonzero; Pick R2C5 | Values retained; fresh ADR formula pose differs; one Undo restores anchor, values and pose | None |
| Mirror Pick | Retarget from either half | Counterpart key, mirrored values/movement and joint Undo | None |
| Matrix Pick | Re-anchor thumb | Stored frame retained; members move; Undo restores | None |
| Refusals | Locked, driven, member, cross-layout/board, missing target/key/counterpart, loop | Accepted document/history unchanged | Existing refusal |
| Non-Pick | Follow-subset, part/matrix Detach, drop | Existing pose preservation | Existing guards |

</frozen-after-approval>

## Code Map

- `core/src/matrix/part_anchors.rs`: `set_anchor` already retains all values. Reuse validation/mirror fan-out; do not change Follow/drop `solve_for_pose`.
- `core/tests/split_part_anchoring.rs`: public Open/Edit/Undo fixtures; basic Pick/mirror movement exists. Existing Follow/Detach tests must remain unchanged.
- `core/tests/matrix_anchors.rs`: existing movement/stored-origin, graph-refusal and detach regressions. Native modules register in `core/tests/integration.rs`.
- `web/src/presentation/layout_component_inspector_tests.rs`: reuse configured mounted probe, host, control/click and settling helpers against Runtime/Core; address controls by label.
- `web/src/presentation/inspector/layout_component_inspector.rs`: one-based row/column, immediate Pick commit; action docs already describe movement.
- `web/crates/layout/src/objects/layout_transform_toolbar.rs`, `web/crates/ui-model/src/transform_feedback.rs`: retain existing feedback owner/formatter/tests.
- `core/tests/boundary_parity.rs`: golden reproduction; fixture `core/tests/fixtures/boundary/transcript.json`.
- `docs/adr/0040-part-anchoring-column-vocabulary.md` already documents moving Pick. Check `GLOSSARY.md` Part anchor and rendered Inspector help for agreement.
- Source: [04](../../../../layout-controls-polish/issues/04-re-anchor-moves.md); epic supersedes legacy process requirements.

## Tasks & Acceptance

**Execution:**
- [x] `core/tests/split_part_anchoring.rs` — cover nonzero carried/local values and rotated key using an independent placement oracle, value retention and Undo/Redo; cover both-half mirror movement and missing refusal cases.
- [x] `core/tests/matrix_anchors.rs` — retain movement/frame, loop and Detach tests; strengthen full stored-frame assertion if needed. Test every matrix row without weakening existing oracles.
- [x] `core/src/matrix/part_anchors.rs` — fix only demonstrated failures; retain already-correct code. Record RED/GREEN for defects, not manufactured reversions of committed work.
- [x] `web/src/presentation/layout_component_inspector_tests.rs` — mount Inspector, Pick another key with nonzero values, assert retained displayed/accepted values and canvas/evaluated movement, then one Undo/Redo through production handlers.
- [x] `web/src/presentation/inspector/layout_component_inspector.rs`, `docs/adr/0040-part-anchoring-column-vocabulary.md`, `GLOSSARY.md` — correct/add Pick help where necessary; preserve accurate documentation.
- [x] `core/tests/fixtures/boundary/transcript.json`, `scripts/check-native-tests-baseline.json` — modify only for explained downstream Pick hash changes or deliberate native test renames, respectively.

**Acceptance Criteria:**
- Given an anchored part in mounted Inspector, when Pick commits another key, then the rendered/evaluated item moves with values retained and one Undo restores it.
- Given the completed changes, when scoped suites and the full gate run, then all matrix scenarios have executed passing coverage and actual outcomes are recorded here.

## Implementation Notes

- Preserved production Core Pick/Follow/Detach/drop and feedback implementations; no demonstrated product defect required a Core change.
- Added independent trigonometric placement checks with all six nonzero carried/local fields, rotated targets, both edited halves, every Follow subset, mapped counterpart values and joint Undo/Redo.
- Added atomic Pick refusal coverage for locked/driven/member owners, cross-layout/board targets, missing part/matrix/key, constraint-driven key and missing source counterpart of a valid mirror-only matrix. Invalid initial fixture attempts were corrected to reach the intended guards; these were test-fixture failures, not product RED evidence.
- Strengthened matrix Pick coverage to compare the complete stored matrix (nonzero origin/rotation), all three member poses and Undo/Redo.
- Added mounted Inspector Pick coverage using labelled controls and production action/history handlers, asserting retained displayed/accepted values and evaluated movement. Added explanatory Inspector help and glossary text; ADR already agrees.
- No native test renames or boundary golden changes.

## Plan Change Log

## Review Triage Log

- Quick review completed by a fresh-context reviewer: **no findings**. Verdict counts: high 0, medium 0, low 0, false 0, maybe-false 0. No review patches or deferrals. Coordinator inspected the complete diff and audited every matrix row against executed coverage; full verification passed on the unchanged product tree.

## Verification

Wrap each build/check with `bash -c 'exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9; <command>'`. Run sequentially; long logs only in `.evidence/`. Preserve repository Cargo flags.

- `cargo test -p boardstudio_core --test integration split_part_anchoring` — all part scenarios pass.
- `cargo test -p boardstudio_core --test integration matrix_anchors` — frame, movement, graph and Detach pass.
- `cargo test -p boardstudio_core --test integration boundary_parity` — golden matches; bless only explained downstream Pick changes.
- `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs` — mounted suite runs/passes; layout-only browser checks do not replace it.
- `python3 scripts/check.py` — full repo/tooling/lint/build/test/browser gate; failures block completion.

Inspect mounted DOM/scene with accepted state, and retain exact feedback tests. Only for deliberate native renames run `python3 scripts/check.py test --update-native-baseline`, reviewing name changes. Record executed covering tests for every matrix row and any unmet criterion.

### Executed outcomes (2026-11-15)

- Scoped native suites: `split_part_anchoring` **14 passed**, `matrix_anchors` **42 passed**, `boundary_parity` **7 passed**. Logs: `.evidence/reanchor-parts.log`, `reanchor-matrices.log`, `reanchor-boundary.log`.
- Pick and mirror rows: `pick_retains_all_values_from_either_half_with_rotated_keys_and_every_follow_subset` executes 16 edited-half/Follow combinations, with retained values, independent placement, mapped values and joint Undo/Redo.
- Matrix row: `reanchoring_a_matrix_moves_it_and_keeps_its_stored_frame` checks the complete nonzero stored frame and every thumb member through Undo/Redo.
- Refusal row: `pick_refusals_leave_the_document_and_history_unchanged` executes ten owner/target/counterpart refusals with whole accepted-document equality and history sentinels; `anchor_graph_refusals_leave_state_unchanged` covers self/cycle and other matrix graph refusals.
- Non-Pick row: unchanged `drop_anchors_to_the_nearest_key_and_keeps_the_drop_point_upright`, `follow_subset_change_keeps_the_pose`, `detach_keeps_the_pose_from_the_half_origin`, `detach_keeps_world_frame_and_clears_the_anchor` and `linked_detach_clears_both_anchors_and_keeps_both_poses` all passed.
- First mounted invocation was interrupted by the shell's 120-second timeout. The next invocation reached compilation and failed because generated `web/assets/tailwind.css` was absent. The documented `dx build --web --no-default-features --features page --cargo-args=--locked` prerequisite then succeeded. Mounted suite retry passed: **50 executed, 0 failed**, including `mounted_pick_moves_with_all_values_retained_and_one_undo_redo`. The test inspects labelled mounted fields and accepted evaluated pose through Pick/Undo/Redo. Log: `.evidence/reanchor-inspector.log`.
- Full `python3 scripts/check.py` gate **failed in browser**, so completion/acceptance remains blocked. Repo/tooling checks, checked build, native tests, native test-name guard, formatting and native/WASM Clippy passed. The full native run re-executed the final additions: **903 Core integration tests passed**. Existing `transform_feedback::tests::text_note_commit_reports_the_anchor_toast` passed unchanged. Layout browser launch passed **281 tests**.
- Full-gate blockers outside this diff (baseline causality not established): `library::mounted_tests::browser_selection_previews_without_opening_and_focuses_the_transfer`, `web/crates/library/src/library.rs:1993`, timed out after 5000 ms (`outcome=None`, accepted revision 0); `parts::generator_settings::tests::upload_preview_refresh_keeps_a_later_failed_apply_draft`, `web/crates/parts/src/parts/generator_settings.rs:2461`, panicked `no entry found for key`. Later panic-hook failures/timeouts were cascade symptoms reported by the harness. No out-of-scope fixes or test weakening attempted. Remaining downstream full-gate steps are not claimed as passed. Log: `.evidence/reanchor-full-check.log`.
- Gortex post-edit detect/tests/guards/contract ran. Its graph did not identify test coverage for these test symbols and reported no configured guards; direct executed suites above are the verification evidence. No production signatures changed.
- Follow-up investigation: without product/test changes, `python3 scripts/check.py browser --only library parts` passed **Parts 106/106** and **Library 22/22**, including both previously failing tests. Log: `.evidence/reanchor-blockers-rerun.log`. The Parts helper polls for upload completion for a bounded interval before indexing a parameter; the Library helper waits on an observed rename outcome after blur. These identify the failure sites, not a proven root cause. No baseline execution or pre-existing-failure claim was made.
- Unchanged-tree full rerun: `python3 scripts/check.py` **passed all steps: repo, tooling, build, lint, test, browser** (1225.7 seconds), including the page runner's **263 executed, 0 failed**. Log: `.evidence/reanchor-full-check-rerun.log`. This supersedes the earlier verification blocker, without erasing the failed attempt. The two browser failures did not reproduce in either subsequent run; root cause remains unproven. No unrelated files, harness settings, timeouts, exclusions or test expectations were changed.
