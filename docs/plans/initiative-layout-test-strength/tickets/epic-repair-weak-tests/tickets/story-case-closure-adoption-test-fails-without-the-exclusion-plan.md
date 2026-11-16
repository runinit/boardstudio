---
title: 'Case-closure adoption test fails without the exclusion'
type: chore
ticket: 1
created: '2026-11-15'
status: built
baseline_revision: 'cdbd69ca6b0d2db21576d28f82e079b48e465c94'
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

**Problem:** The case-clearance adoption test has no generated key part, so adoption returns before checking the case-closure exclusion. Its empty-list assertion cannot detect a regression.

**Approach:** Repair only this test's fixture to include a matrix key and an ordinary unassigned component. Keep the key, ordinary component and clearance hole left of the mirror axis, assert that the ordinary component is adopted and the hole is excluded, and prove sensitivity by temporarily removing the production exclusion. No shared fixture or delivered production changes; if the repaired test fails against current production, stop and report a separate bug. Build ticket 1.1 only, preserve the current branch and planning commits, leave necessist.db untouched and uncommitted, and do not stash, push or open a PR.

</frozen-after-approval>

## Implementation Notes

- Planning: oneshot selected for a test-local change under 100 lines in core/src/matrix/layout.rs. Risk low: no delivered production behavior or shared helper changes.
- Investigation: adoption gathers key positions through matrix::cell_members. Add key matrix/matrix/r0c0 left of axis x=100, retain the hole at x=10, and add an ordinary part at x=20. Exact adopted IDs demonstrate exclusion and actual adoption.
- Audit F1 was rechecked against current source; git diff --stat from research commit 14c276248bb8064c8f5d27760b5c33ac7ce63b53 is empty for layout.rs and matrix.rs.
- Gortex route is exact; impact reports low risk and no dependents for this test. Two plan writes were refused as stale, so native writes use the AGENTS.md fallback. target resolves inside this worktree, not the main checkout. User approved proceeding with untracked necessist.db present.
- Implementation: changed only the named test in core/src/matrix/layout.rs. Added the generated matrix key at x=0 and an unassigned controller at x=20; retained the hole at x=10 and axis x=100. Explicit hole-exclusion and exact controller-adoption assertions now exercise the filter. Two Gortex edit admissions for layout.rs were rejected as stale; native patch fallback was used.
- Gortex detect included the test module and the adjacent function because of stale line ranges; Git's diff confines changes to the named test. Follow-up tests/guards/contract reported no graph test targets, no configured guards, and allow with a medium lower-bound risk. These graph limitations do not replace the actual test runs.
- Counter-example evidence (2026-11-15): the exact test command below first passed (1 passed, 0 ignored; .evidence/bmad-build-1-1/production-test.log). Temporarily changed only `.filter(|id| !assigned.contains(id) && !id.starts_with("case-closure/"))` to `.filter(|id| !assigned.contains(id))`. The same command exited 101, executing one test: `assertion failed: !doc.layouts[0].part_ids.contains(&"case-closure/board/10/10".to_owned())` at layout.rs:4808; result `0 passed; 1 failed; 0 ignored` (.evidence/bmad-build-1-1/counter-example.log). Immediately restored the predicate, inspected the Git diff (only the test remained changed), and reran: 1 passed, 0 ignored (.evidence/bmad-build-1-1/restored-test.log). Counter-example is not delivered.
- Neighboring module run passed all 4 tests, 0 ignored (.evidence/bmad-build-1-1/case-hardware-tests.log).
- Full project check passed: `python3 scripts/check.py`, all steps repo, tooling, build, lint, test and browser, total 1477.8s (.evidence/bmad-build-1-1/full-check.log). The final page browser batch executed 263 tests with 0 failures. No check failures to quote.
- Git baseline comparison verified that all content outside the named test is byte-identical; the exclusion is restored. Only layout.rs and this plan are deliverables. Untracked necessist.db remains untouched and is excluded from the review artifact and commit.

## Plan Change Log

## Review Triage Log

- Quick inspection review (GPT-6.1 Sol, high): no findings; nothing patched or deferred. The oneshot route skipped all other review lenses.

## Verification

- `cargo test -p boardstudio_core --lib matrix::layout::case_hardware_tests::case_clearance_holes_are_not_adopted_by_linked_layouts -- --exact --nocapture` — exactly one passing test; stop if current production fails.
- Temporarily remove only `!id.starts_with("case-closure/")` from the adoption filter; run the same test and expect failure because the hole is adopted. Restore immediately, rerun and expect success. Record failure output and log paths here. Never commit the counter-example.
- `cargo test -p boardstudio_core --lib matrix::layout::case_hardware_tests` — neighboring tests pass.
- `python3 scripts/check.py` — full project check passes; long logs under .evidence/. Quote any failure without unsupported attribution, including rename-preview or model-upload failures if they recur.
- `git diff --check` and `git diff -- core/src/matrix/layout.rs` — clean whitespace, only the named test changed, production identical to baseline. Commit only this test and its BMad plan if directed by the workflow.
