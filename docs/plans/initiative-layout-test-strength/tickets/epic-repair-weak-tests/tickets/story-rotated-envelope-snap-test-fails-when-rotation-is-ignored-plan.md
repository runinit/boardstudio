---
title: 'Rotated-envelope snap test fails when rotation is ignored'
type: chore
ticket: 2
created: '2026-11-16'
status: done
baseline_revision: '7c2640ff1e48e12beef3cbeaf10e215ff241ddd4'
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

**Problem:** The rotated-envelope gap snap test uses an 18x18 square, making its 0-degree and 90-degree cases indistinguishable when the target rotation is ignored (epic R5).

**Approach:** Change only the named test's fixture to an 18x20 envelope and independently calculate the expected gap and aligned-edge positions for each rotation. Retain coordinate and label assertion intent. Prove the delivered test passes, temporarily remove its target rotation assignment and demonstrate failure, then immediately restore and rerun. No delivered production changes, shared fixture changes, ignored tests, or changes to story 1.1. If current production fails the repaired test, stop and report a separate bug. Preserve the branch and required commits; leave necessist.db unread, untouched and excluded from review and commits; no stash, push or PR.

</frozen-after-approval>

## Implementation Notes

- Oneshot: a low-risk change under 100 lines, confined to gap_snap_preserves_edge_alignment_for_rotated_envelopes in application/tests/drag_snap.rs. User approved proceeding with only untracked necessist.db present.
- Current-source investigation confirms the shared fixture's 18x18 keycap and identical expectations for both rotations. Set document.definitions[0].keycap locally, not fixture(). snap_part_in_board delegates to snap_part_candidates in application/src/interactions.rs; the latter calculates envelope gaps then aligns the nearest edge within tolerance. These production functions remain unchanged.
- Independent geometry: moving envelope half-width 9, half-height 10; right target center (30,0). At 0 degrees target half-extents (9,10), so x=30-9-1-9=11 and aligned bottom edges give y=-10+10=0. At 90 degrees target half-extents (10,9), so x=30-10-1-9=10 and y=-9+10=1. Start each moving pose 0.1 mm left of expected x at y=0.3; tolerance 2 mm permits both gap and edge correction. Use fixed arithmetic, never production geometry helpers for expectations.
- Gortex reads route to this exact worktree; impact reports LOW with no dependents. Validate actual changes against Git. Follow story 1.1's normal/counter-example/restored/module/full-check evidence pattern without modifying that story.

- Implemented only the named test: local 18x20 keycap override, separate moving pose per rotation, fixed independent expected positions (11,0) and (10,1), and retained both coordinate checks and the exact edges-aligned gap label. Gortex writes succeeded without fallback; detect identifies only this test. Tests/guards/contract found no graph test targets, no configured guards, and allow/low risk; actual test commands supply verification.
- Counter-example (2026-11-16): exact targeted command passed 1 test, 0 ignored (.evidence/bmad-build-1-2/production-test.log). Temporarily removed only `document.parts[1].pose.rotation = rotation;`. The same command exited 101 at drag_snap.rs:72, with `rotation 90: SnapGuide { at: Vec2 { x: 11.000000000000002, y: -7.216449660063518e-16 }` instead of expected (10,1); result `0 passed; 1 failed; 0 ignored` (.evidence/bmad-build-1-2/counter-example.log). Immediately restored the assignment before any further work; Git diff shows only the repaired test, no production changes. Restored exact test passed 1/1 (.evidence/bmad-build-1-2/restored-test.log); module passed 8/8, 0 ignored (.evidence/bmad-build-1-2/drag-snap-tests.log).
- target resolves to this worktree's own target directory, not a shared target; no build flag overrides or overlapping manual Cargo builds. Full project check started after targeted runs; result pending (.evidence/bmad-build-1-2/full-check.log).

- First full check exited 1: repo, tooling, build, lint and test passed; browser failed (.evidence/bmad-build-1-2/full-check.log). Exact runner summary: `check: first failure [Rust panic/assertion]: parts::generator_settings::tests::mounted_generator_upload_landing_preserves_apply_draft_012 at web/crates/parts/src/parts/generator_settings.rs:3485:24 — no entry found for key; follow-on symptom: 33 later test(s) hit 'cannot modify the panic hook from a panicking thread' in shared WASM state (re-run after fixing first failure); later harness timeout was a follow-on symptom`. Cause not established; no claim that this change or a pre-existing fault caused it. One unchanged full-check retry planned; no unrelated code will be changed.

- Unchanged full-check retry passed all six steps: repo 7.0s, tooling 36.5s, build 65.8s, lint 70.0s, test 52.7s, browser 268.8s, total 382.3s (.evidence/bmad-build-1-2/full-check-retry.log). Final page batch executed 263 tests, 0 failures. This passing retry does not establish the cause of the first run's failure. No production or unrelated test changes were made between runs.

## Plan Change Log

## Review Triage Log

- Quick inspection review (GPT-6.1 Sol, high) completed: no findings; nothing patched or deferred. Other lenses skipped on the oneshot route.
- Final Git inspection: only the named test and this plan are deliverables; production, the shared fixture and story 1.1 remain unchanged. Whitespace check passed. The temporary mutation is restored. necessist.db remains unread, untouched and excluded from the review diff and commit. Required history is preserved; nothing pushed and no PR opened.

## Verification

- `cargo test -p boardstudio-application --test drag_snap gap_snap_preserves_edge_alignment_for_rotated_envelopes -- --exact --nocapture` — one test passes; stop on a current-production failure.
- Temporarily remove only `document.parts[1].pose.rotation = rotation;`; rerun the exact test and record the assertion failure. Immediately restore the assignment, inspect Git diff for production invariance and rerun successfully. Never commit the mutation.
- `cargo test -p boardstudio-application --test drag_snap` — all neighboring tests pass with none ignored.
- `python3 scripts/check.py` — all six full-project steps pass. Long logs in .evidence/bmad-build-1-2/; quote failures without unsupported cause attribution.
- Recheck target resolution before builds; use prescribed build lock if shared. Do not override Cargo concurrency/debug settings or overlap manual builds.
- `git diff --check` and baseline comparison — only the named test plus this plan delivered; production and shared fixture identical to baseline. Review excludes necessist.db. Record review outcome and canonical commit identifier if workflow commits.
