---
title: 'Framed solve tests and splay-origin validation fail under their counter-examples'
type: chore
ticket: 3
created: '2026-11-16'
status: done
baseline_revision: '100a9b90095e1e9ce6af55712bbf6fac7b4c19a0'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The persisted framed-solve and local-mirror endpoint tests begin at the poses they assert, so skipping their solve calls passes. The non-finite splay-origin case inherits an oversized column list and rejects for the wrong reason (epic R2/R3).

**Approach:** Perturb the two framed tests' targets locally in both position and heading while retaining their independent expected poses and existing assertions. Give the non-finite-origin case a fresh valid matrix before assigning NaN; retain the repeated resize/reset/zip block for the later Refactor sweep. Demonstrate normal passing runs, failing counter-examples and immediate restoration/reruns, then neighboring tests and the full project check. No delivered production or shared fixture changes. If current production fails a repaired test, stop and report a separate production bug under this epic; do not fix production or commit ignored tests. Preserve stories 1.1/1.2 and required history, leave necessist.db unread/untouched and excluded from reviews/commits, and do not branch, stash, push or open a PR.

**Decision (2026-11-16):** User selected the test-act control: temporarily replace only the NaN splay origin with a finite value and require the unchanged rejection assertion to fail. This proves dependency on invalid origin, not detection of a single input-guard removal; downstream finite-result protection makes the latter mutation survive. Do not disable unrelated production guards.

</frozen-after-approval>

## Implementation Notes

- Oneshot selected: fewer than 100 changed code lines in two test sections; low business risk because delivered production and shared fixtures stay unchanged. User approved proceeding with the only pre-existing working-tree item, untracked necessist.db.
- Code map: core/src/constraints/framed_tests.rs, a_local_mirror_is_an_ordinary_part_endpoint (460) and persisted_framed_constraints_solve_through_the_stored_document_path (687). Reuse set_at/set_heading locally; do not modify document(), with_exception(), offset_5_neg2() or assert_pose(). Start beta/T off-position/off-heading in the local case; start alpha/T authority and beta/T visible target off-position/off-heading in the persisted case. Keep the existing expected local pose (8,2,55) and persisted authority pose (4.2,14.4,161.86989764584402).
- Independent oracle: local/P at (3,4), heading 40, plus world offset (5,-2) and heading 15 gives beta/T (8,2,55). With L(x,y)=(-0.8x-0.6y,-0.6x+0.8y) and t=(20,-7), the pair map is F(p)=L(p)+t and its inverse is F⁻¹(q)=L(q-t): remove translation before applying the linear reflection. Thus F(8,2)=(12.4,-10.2), whereas F⁻¹(8,2)=(4.2,14.4). The reflected heading is 180+36.86989764584402-55. Expectations must not call production geometry helpers.
- Code map: core/src/matrix.rs, projection_tests::projection_rejects_invalid_draft_geometry (1160). Insert a fresh matrix before the final NaN-origin setup, leaving prior invalid-dimension/pitch/oversized-list cases and repeated fixture block unchanged. Assert fresh geometry validates before assigning NaN, isolating origin as the invalid input.
- Research F2/F3/F5 rechecked against current code. Git diff from research commit 14c276248bb8064c8f5d27760b5c33ac7ce63b53 is empty for framed_tests.rs, matrix.rs and kernel.rs. Prior story plans loaded for continuity: local fixtures, independent oracles, mutation/restoration evidence, module tests, full check and production invariance. Neither prior plan is modified.
- Verification limitation requiring a decision: valid_projection_matrix runs the entire kernel evaluator. Removing only kernel.rs validate's finite_vec(c.splay_origin) input check cannot make this fresh NaN case succeed: intermediate/output finite checks still reject. They share the same generic error, so an error-text assertion cannot identify the rejecting guard. Independent read-only investigation confirmed this; no execution or mutation has occurred. Do not disable unrelated finite-result guards or represent a test-act control as proof of killing the single input-check removal.
- Exact Gortex worktree routing confirmed. Test impact reports no dependents with a budget-limited risk attachment; use actual runs and Git to verify scope. target resolves inside this worktree; recheck before builds, use the prescribed lock if shared, preserve Cargo settings/runners and do not overlap manual builds. .claude/settings.json is absent; follow environment sandbox safeguards.

- Implementation (2026-11-16): added only local set_at/set_heading calls in the two named framed tests; inserted a fresh matrix plus an is_ok() precondition before the final NaN-origin setup. No shared helper or production edits. Existing rejection and pose assertions remain unchanged. Gortex writes succeeded without fallback. Detect found the three named tests plus the containing projection_tests module; tests returned no graph targets, guards none configured, contract warned about incomplete graph reach/co-change. Those warnings do not authorize a refactor or endpoint change.
- STOP: first normal command, `cargo test -p boardstudio_core --lib constraints::framed_tests::persisted_framed_constraints_solve_through_the_stored_document_path -- --exact --nocapture`, exited 101: `0 passed; 1 failed; 0 ignored; 766 filtered out`. Log: .evidence/bmad-build-1-3/persisted-normal.log. At core/src/constraints/framed_tests.rs:76:5: `alpha/T is Pose2 { at: Vec2 { x: 4.200000000000002, y: 14.399999999999999 }, rotation: -198.13010235415598 }, expected (4.2, 14.4) heading 161.86989764584402`.
- Separate bug investigation report for epic-repair-weak-tests: perturbing the persisted framed target's starting position/heading reveals a heading-representation mismatch against the existing literal oracle. Reproduce with this uncommitted repaired test and the command above. Position agrees within tolerance; headings differ by exactly 360 degrees and represent the same physical orientation. A production defect is not established; determine the intended heading-representation contract separately before deciding on a production fix or test-fixture refinement. No ticket file or epic spec was modified by Build.
- Stopped immediately after that current-production failure, as requested: the other two normal cases, all temporary counter-examples/restored runs, neighboring suites, full check and quick review have NOT run. No temporary mutation has been applied; none needs restoring. Do not commit this failing repair or mark it built. No commit, push or PR.

- Final Git confirmation: baseline diff contains only eight added lines inside the three named tests; all production code, shared fixtures and prior story plans are unchanged. git diff --check passes. Branch and HEAD remain audit/layout-engine-pcb-layout and 100a9b90095e1e9ce6af55712bbf6fac7b4c19a0. necessist.db remains the pre-existing untracked item, unread and untouched; no review artifact or commit was created.

### Resumed verification (2026-11-16)

- With the approved initial authoritative heading 0, all three exact normal commands passed: 1 passed, 0 failed, 0 ignored each. Logs: .evidence/bmad-build-1-3/persisted-normal-resumed.log, local-normal.log and origin-normal.log. The earlier heading-branch failure remains intact in persisted-normal.log; it is not a production-defect claim.
- Persisted counter-example: skipped only super::resolve in the named test. Exit 101, 0 passed, 1 failed, 0 ignored; `alpha/T is Pose2 { at: Vec2 { x: -12.0, y: 9.0 }, rotation: 0.0 }, expected (4.2, 14.4) heading 161.86989764584402` at framed_tests.rs:76:5. Immediately restored, inspected the Git diff, and reran: 1 passed, 0 ignored. Logs: persisted-counter-example.log and persisted-restored.log in the same evidence folder.
- Local-mirror counter-example: skipped only resolve_prepared in the named test. Exit 101, 0 passed, 1 failed, 0 ignored; `beta/T is Pose2 { at: Vec2 { x: -12.0, y: 9.0 }, rotation: -20.0 }, expected (8, 2) heading 55` at framed_tests.rs:76:5. The unused-record warning was caused by the temporary skipped call. Immediately restored, inspected the Git diff, and reran: 1 passed, 0 ignored. Logs: local-counter-example.log and local-restored.log.
- Splay-origin test-act control: changed only its NaN x coordinate to 0.0 on the fresh matrix, leaving the final assertion unchanged. Exit 101, 0 passed, 1 failed, 0 ignored; `assertion failed: valid_projection_matrix(&m).is_err()` at matrix.rs:1202:9. Immediately restored NaN, inspected the Git diff, and reran: 1 passed, 0 ignored. Logs: origin-counter-example.log and origin-restored.log. This proves dependency on the invalid origin, not detection of a single input-guard removal; no production guard was changed.
- Neighboring suites passed: constraints::framed_tests 37/37 and matrix::projection_tests 7/7, both 0 ignored (framed-tests.log and projection-tests.log). Git confirms all temporary edits restored and only eight fixture/precondition lines added inside the three named tests; production and shared fixtures remain unchanged.
- Gortex's first resumed plan write was refused because its checkout view held the prior HEAD; the prescribed delayed retry succeeded, with no native-edit fallback. Later detection sometimes reported graph publication pending; actual Git diffs and test runs are the scope/evidence authority.
- Concurrent user planning edits appeared during verification, concerning other repair/harness entries. They leave entry 1.3 and R2/R3 unchanged and are preserved, excluded from this story's commit and review scope. The original approved planning replan commit remains history, not a working-tree deliverable of this build.
- target still resolves inside this worktree. No concurrency/debug overrides or overlapping manual builds. The full project check is running under the prescribed lock, with output only in .evidence/bmad-build-1-3/full-check.log; result and quick review pending.

- Full project check completed successfully: `python3 scripts/check.py`, all six steps passed: repo 6.8s, tooling 36.4s, build 138.0s, lint 146.3s, test 67.4s, browser 572.3s, total 761.8s (762.0s including wrapper). Final browser batch executed 263 tests, 0 failures. Log: .evidence/bmad-build-1-3/full-check.log. No full-check failure occurred in this run; earlier historical failures are not assigned a new cause.
- Concurrent planning edits were committed independently as cf4bd6e94122faa62111e8bb20723d8cb929cebe, above the approved replan 827a7ba3b0b7648e873f3a5f90c7a56392a0bd3b. Preserve both commits and all earlier required history. Before review, the only remaining tracked modifications are the two test files; the story plan and necessist.db are untracked. Review scope includes this story's source/plan and its approved replan, not unrelated concurrent planning changes or necessist.db.
- Following the concurrent HEAD change, Gortex refused the plan update as stale and its delayed retry returned view_building. Used the AGENTS.md native-edit fallback for this plan only; source edits and all temporary mutations/restorations used Gortex. Validate delivered scope against Git.

## Plan Change Log

- 2026-11-16 — User approved resuming Build after the planning-only replan committed as 827a7ba3b0b7648e873f3a5f90c7a56392a0bd3b. The earlier heading mismatch is explained by documented nearest-previous-heading selection, not an established production defect. Set only the persisted test's initial authoritative heading to 0 degrees, retaining displaced positions, both independent expected poses, exact assertions, and the three-test scope. Keep the fresh-matrix splay repair and approved finite-origin test-act control. Clear the resolved block and preserve the original baseline revision and all historical failure evidence; no production, shared fixture, prior-story or planning-commit changes.

## Review Triage Log

- Quick review (GPT-6.1 Sol, high): one finding, classified low and patched; 0 high/medium/false/maybe-false findings, nothing deferred. Other review lenses skipped on the oneshot route.
- Patch — the plan incorrectly called the translated map its own inverse and assigned F(8,2) the inverse result. Confirmed against mechanical/pair.rs::inverse, which subtracts translation before the linear reflection, and independent arithmetic: F(8,2)=(12.4,-10.2), F⁻¹(8,2)=(4.2,14.4). Corrected only the documented derivation; retained the existing test oracle, assertions and all source code. No second review agent was needed for this small correction.
- Final scope check: the delivered source diff remains exactly eight added test-local fixture/precondition lines in the three named tests. Production, shared fixtures, the existing exact assertions and prior story plans remain unchanged; every temporary counter-example is restored. Preserve both planning commits and all required history. Commit only the two test files and this plan, leaving necessist.db unread, untouched, untracked and excluded. No push or PR.

## Verification

- Run each repaired test normally and stop on a current-production failure:
  - cargo test -p boardstudio_core --lib constraints::framed_tests::persisted_framed_constraints_solve_through_the_stored_document_path -- --exact --nocapture
  - cargo test -p boardstudio_core --lib constraints::framed_tests::a_local_mirror_is_an_ordinary_part_endpoint -- --exact --nocapture
  - cargo test -p boardstudio_core --lib matrix::projection_tests::projection_rejects_invalid_draft_geometry -- --exact --nocapture
- Temporarily remove each framed test's own solve call in separate runs, expect its existing pose assertion to fail; restore immediately and rerun. For the splay-origin test-act control, temporarily replace only its NaN x coordinate with 0.0; expect the unchanged final is_err() assertion to fail, restore immediately and rerun. Record executed counts, exact failures and restoration evidence here; never commit mutations.
- Neighboring tests: cargo test -p boardstudio_core --lib constraints::framed_tests; cargo test -p boardstudio_core --lib matrix::projection_tests. Expect all tests passing, none ignored.
- Full check: python3 scripts/check.py. Long output goes only to .evidence/bmad-build-1-3/ logs. Quote failures without unsupported cause attribution. Story 1.2's first check failed with generator_settings.rs:3485:24 `no entry found for key`; an unchanged retry passed, cause unknown.
- Inspect diff before tests. After restoration, git diff --check and baseline comparison must confirm only the named tests plus this plan changed, including byte-identical production, shared fixtures and prior stories. Exclude necessist.db from all review artifacts and commits. Record review outcome and canonical commit hash if the workflow commits.
