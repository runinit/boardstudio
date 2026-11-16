---
title: 'Refactor sweep'
type: refactor
ticket: 5
created: '2026-11-16'
status: done
baseline_revision: 'b015a9ba59475c7f4fdb30b0a2b7ff89aefb5ab9'
route: full
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

**Problem:** Six test-local column-layout setup blocks repeat grow/reset/zip mechanics across three modules, obscuring scenario inputs and making the repaired splay-origin case harder to maintain.

**Approach:** Replace repeated blocks with one private field-selecting helper in each repeated-use test module. Use the epic's approved direct-assignment exception for the single occurrence in the operation module. Preserve every scenario's resulting inputs, independent expectations and existing assertions. This is only ticket 1.5, epic R7 (parent R9).

## Boundaries & Constraints

**Always:** Helpers retain existing record count when values are shorter, extend when longer, default only the selected field across all records, and assign the supplied prefix in order. Preserve other record fields. Retain the fresh valid matrix and is_ok precondition before the NaN-origin case, and the oversized-list rejection before it. Keep all prior repaired behaviors and built plan statuses. Preserve existing worktree, branch and unpublished history; leave necessist.db unread, untouched and excluded from artifacts and commits.

**Never:** Production behavior changes, public/shared production helpers, assertion weakening, ignored tests, new fixture values, unrelated cleanup, harness-epic work, branch/checkout changes, stash, push or PR. No prior-story plan edits. No production fix if verification exposes a product bug: stop and report it separately.

## I/O & Edge-Case Matrix

| Setup | Expected preservation | Error handling |
|---|---|---|
| Values shorter than existing records | No truncation; selected-field tail defaults; other fields unchanged | Existing tests remain green |
| Values longer than existing records | Extend to values length; preserve other existing fields | Oversized projection still rejected |
| NaN splay origin on fresh valid matrix | Only intended origin invalid; original rejection assertion retained | Rejection still required |

</frozen-after-approval>

## Code Map

- `core/src/matrix/splay.rs::tests`: two blocks in `column_angle_moves_only_the_selected_column` set stagger [0;4] and splay [12,8,0,0]. Add one generic field-selecting helper beside existing test helpers; keep matrix(), key(), update(), origin_world() and assertions unchanged.
- `core/src/matrix.rs::projection_tests`: three blocks in `projection_rejects_invalid_draft_geometry` and `column_basis_applies_splay_mirror_and_matrix_rotation`. One helper handles Vec2 splay_origin and scalar splay. Preserve optional-value default conversion, oversized values count, fresh matrix boundary and NaN.
- `web/crates/layout/src/matrix_transform_operation.rs::tests::convert_offset_builds_the_one_shot_placement_and_follows_core_availability`: single block resets the three existing splay fields then sets the first to 12. Direct setup is sufficient; keep matrix() and all other fields unchanged.
- P1 research locations rechecked against current source. Since its revision only matrix.rs changed (the approved fresh-matrix repair); other two files unchanged. Prior plans 1–4 record no additional deferred cleanup. Do not broaden the sweep.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/matrix/splay.rs` — extract the two setup blocks into one test-private helper with explicit field selector; preserve both scenario arrays.
- [x] `core/src/matrix.rs` — extract three blocks into one projection-test helper; retain default mapping and both rejection causes in separate matrices.
- [x] `web/crates/layout/src/matrix_transform_operation.rs` — replace the single block with equivalent direct splay assignments.
- [x] The three files above — verify matrix cases with existing module tests; compare setup expansion against baseline for length, selected-field defaults, prefix values and untouched fields.
- [x] This plan — record module/full-check results, Git scope and review outcome; commit only the three test files and plan if the rendered workflow directs it.

**Acceptance Criteria:**
- Given the six baseline setup blocks, when refactored, then only the two module helpers contain grow/reset/zip mechanics and no scenario repeats the block.
- Given the existing tests, when the three module suites execute, then all pass without changing scenario values, existing assertions or ignored status.
- Given the completed sweep, when the full project check runs, then all steps pass and Git shows no production changes or edits to prior plans.

## Implementation Notes

- 2026-11-16: Implemented through native Gortex MCP in the exact current worktree. Frontmatter context is empty. Added one private `set_column_values<T: Default>` helper per repeated-use test module; the selector returns only the chosen mutable field. Replaced the operation test's single block with direct splay assignments. No existing function signatures changed.
- Baseline expansion audit against `b015a9ba59475c7f4fdb30b0a2b7ff89aefb5ab9`: both helpers retain `max(values.len(), records.len())`, default the selected field on every record, then assign the prefix in order. Thus shorter lists retain/default the tail and longer lists append default records; other existing fields are untouched.

| Baseline setup | Preserved expansion |
|---|---|
| Splay test stagger then splay | Records 3 → 4; stagger `[0,0,0,0]`, splay `[12,8,0,0]`; other fields unchanged/default on extension |
| Oversized origin list | Records 2 → 4 for 3 columns; all origins default; second record's stagger 3 and splay 15 preserved; original rejection retained |
| Fresh valid matrix then NaN origin | Records remain 2; origins `[(NaN,0),(0,0)]`; other fields preserved; fresh `is_ok()` precondition and rejection retained |
| Projection basis splay | Records remain 2; splay `[0,0]`, including defaulted tail; second stagger remains 3 |
| Operation splay | Records remain 3; splay `[12,0,0]`; offsets, staggers and all other fields unchanged |

- Coordinating-agent matrix audit: existing scenario tests did not directly assert every helper-preservation property. Added one test-private characterization test per helper, using the existing matrix fixtures and values, comparing the entire resulting Matrix against independently edited expectations for shorter and longer lists. Existing scenarios and assertions are unchanged. The approved shorter/longer rows are now covered by both `column_values_preserve_unselected_fields_and_record_counts` tests; the NaN row remains covered by `projection_rejects_invalid_draft_geometry`. This is verification of the approved contract, not an intent change.

## Plan Change Log

- No intent or scope changes; no deferred cleanup added.
- 2026-11-16 — Step-03 matrix audit added two helper-preservation tests to cover length/tail/untouched-field behavior explicitly; KEEP all original fixture values, assertions and fresh-matrix validation.

## Review Triage Log

- Implementation handoff: independent workflow review and commit remain for the coordinating agent; no review verdict claimed by the implementer.
- 2026-11-16 — Quick inspection review completed (GPT-6.1 Sol, high): no findings; high/medium/low/false/maybe-false counts all 0. No review patches, loopbacks, deferrals or unresolved review items. Other lenses skipped by the rendered quick-review route. The earlier Parts browser failure remains recorded with unknown cause; the unchanged retry passed.
- Final scope: only three test-private modules and this plan; original scenario inputs/assertions, production and prior built plans are unchanged. Git source blobs match the verified/reviewed versions. necessist.db remains unread, untouched, untracked and excluded; no push or PR.

## Verification

**Observed results (2026-11-16):**
- Before and after the final source edits, the specified module commands passed with identical counts: splay 4, projection 7, operation 38; zero failed or ignored. Logs: `.evidence/story-1-5-before-{splay,projection,operation}.log` and `.evidence/story-1-5-final-{splay,projection,operation}.log` at repository root.
- Full `python3 scripts/check.py`: exit 0; all six steps passed (`repo`, `tooling`, `build`, `lint`, `test`, `browser`), total 750.3s. Final browser batch executed 263 tests, failed 0. Log: `.evidence/story-1-5-full-check.log` at repository root.
- Initial standalone formatting check reported wrapping differences at `core/src/matrix/splay.rs:211` and `core/src/matrix.rs:908`; corrected only that wrapping through Gortex. Final `cargo fmt --all --check` passed; full-check lint also passed. Initial/final logs: `.evidence/story-1-5-after-fmt.log`, `.evidence/story-1-5-final-fmt.log`.
- Re-read the helper/scenario regions of all three modules through Gortex: grow/reset/zip now occurs only in the two private helpers, not any of the six original scenario blocks. Gortex detect/tests/guards/contract completed; no guards configured. Its test-coverage/blast heuristics warned on these test-only symbols, so observed Cargo runs and the inspected baseline diff are the verification evidence, not graph coverage claims.
- `git diff --check` passed. Baseline diff inspection and a test-boundary/assertion-line check confirmed every source change is below `#[cfg(test)]`, existing assertion lines are preserved, and no ignored status changed. Only the three planned source files differ from HEAD; no production code or prior plans changed. This plan was already untracked on entry and remains the only added plan.
- Resolved `target` before builds to this worktree's `target`; all builds/checks used the prescribed file-descriptor build lock. No concurrency/debug overrides, branch changes, stash, commit, push or PR. HEAD remains `b015a9ba59475c7f4fdb30b0a2b7ff89aefb5ab9`; `necessist.db` remains untracked, unread and untouched, excluded from review artifacts.

**Coordinating-agent verification (after the two matrix-audit tests):**
- Formatting passed; module suites passed splay 5, projection 8, operation 38 (51 total, 0 failed/ignored), preserving all 49 original tests and adding two helper checks. Logs: `.evidence/story-1-5-coordinator-{fmt,splay,projection,operation}.log`. Both helper checks and the NaN-origin rejection test executed and passed.
- First coordinator full check exited 1 in browser. Exact runner summary: `parts::generator_settings::tests::upload_preview_refresh_keeps_a_later_failed_apply_draft at web/crates/parts/src/parts/generator_settings.rs:2461:24 — no entry found for key; follow-on symptom: 33 later test(s) hit 'cannot modify the panic hook from a panicking thread' in shared WASM state; later harness timeout was a follow-on symptom`. Log: `.evidence/story-1-5-coordinator-full-check.log`. Cause remains unknown; no claim that this change or a pre-existing fault caused it. One unchanged full-check retry planned; no unrelated source changes or skipped failures.
- Unchanged coordinator retry exited 0: repo, tooling, build, lint, test, browser all passed, total 960.0s; final browser batch executed 263 tests, failed 0 (`.evidence/story-1-5-coordinator-full-check-retry.log`). No source changed between these coordinator full checks; the passing retry does not establish the cause of the first failure. Matrix audit confirmed both helper checks and the fresh-matrix NaN rejection executed and passed, including in the successful full check. Original 49 module tests remain green; two added checks bring the total to 51.

**Commands:**
- Before and after edits: `cargo test -p boardstudio_core --lib matrix::splay::tests`; `cargo test -p boardstudio_core --lib matrix::projection_tests`; `cargo test -p boardstudio-web-layout --lib matrix_transform_operation::tests` — same existing tests pass, none newly ignored.
- `python3 scripts/check.py` — all six full-project steps pass.
- `git diff --check` and baseline diff inspection — only test-private setup/helper changes plus this plan; scenario inputs, assertions, production and earlier plans unchanged.

Inspect all three modules through Gortex to confirm repeated blocks are gone from scenarios. Recheck target resolution before builds; use the prescribed build lock if shared and existing concurrency/debug settings. Put long logs only under .evidence/. Quote failures without unsupported cause attribution. Record results here, not in duplicate status documents. No new behavior mutation is required for this fixture-only refactor; earlier counter-example evidence remains preserved in built plans.
