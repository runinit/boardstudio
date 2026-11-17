---
title: 'Cover nonzero linked rotation during key resize'
type: chore
ticket: 4
created: '2026-11-16'
status: built
baseline_revision: '5c88ca4abfa0f3dfde5bb202e9960d13283574c2'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
followup_review_recommended: false
context: []
warnings: []
deferred: []
---

<intent-contract>

## Intent

**Problem:** The linked resize coverage uses zero rotation, so copying the source angle can escape detection.

**Approach:** Add a named nonzero linked pair scenario alongside the legacy zero-angle test through the real planner and typed Core commit. Preserve A1's accepted independent axis, complete adoption-aware metadata (right split_gap Some(142), spacing None), ownership/correspondence, six identities, widths 18→56, zero-angle displacements (−19,0)/(0,0)/(+19,0), and independent 23-degree cell-rotation displacements ±(17.9594250584,7.6233236509) with the selected center fixed, document/scene agreement and complete paired Undo/Redo snapshots. Prove a temporary copied-source-rotation control fails the unchanged rotation oracle, then restore and pass. Test-only; no production changes or acceptance/closure, push or PR. Protected files remain unread and untouched.

</intent-contract>

## Implementation Notes

Oneshot: under 100 changed source lines in keycap_resize.rs. Share the existing assertion body between separately named zero-angle and nonzero-angle tests. Use cell rotation to preserve the established half origins. Cell centers remain 19 mm apart horizontally; projected separation is 19*cos(23°), old gap max(0,19*cos(23°)-18)=0, resized halfwidth sum 37, so outward movement is 37-19*cos(23°) along (cos(23°),sin(23°)). Establish expectations before the act without production helpers or post-edit output. Independent Sofia Ticket validation found no blockers; A1 is accepted/done and user explicitly authorizes continuation. Previous blocked state was mistaken and cleared. Gortex exact route returns view_building; source write retry after 30 seconds also failed, so use authorized exact native edits.

## Plan Change Log

- Independent Sofia Ticket validation confirmed the geometric values and zero-angle retention; fixed stale A1 wording and explicitly labeled the original blocked result historical.

- 2026-11-16 — User explicitly approved distinguishing zero-angle numeric reflow from the independent 23-degree CELL geometric oracle. Source inspection confirms separate matrix/cell angles, unchanged centers, reflected heading, projected-spacing gap clamping and selected-center preservation. Historical block below and named.log remain evidence; the criteria conflict is resolved and is not a product defect. Original baseline retained; A1 acceptance complete. User authorizes this plan alignment and resumption.

## Review Triage Log

### 2026-11-16 — Review pass
- verdicts: 1 findings — high 0, medium 0, low 0, false 1, maybe-false 0
- findings:
  - `[false]` `[reject]` Reviewer requested checking the accepted axis against fixture axis 0 during Open. The approved intent explicitly requires the independently captured Core-accepted axis before planning, not a fixture-to-Open axis invariant. Inspection confirms `accepted_metadata` and `axis_x` are captured before `plan_resize`, expected committed metadata is constructed before the act, and reflected committed poses use that captured axis. A hypothetical Open defect is outside the accepted A1 oracle and does not show an A2 requirement violation. No source change.

## Verification

Serialize all builds/checks with repository FD9 lock and BOARDSTUDIO_BUILD_LOCK_FD=9. Logs only in .evidence/a2-linked-rotation/.
- Named: cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::replace_document_reflows_nonzero_linked_rotation_through_history -- --exact: one pass, zero ignored.
- Temporary test-act control copies source rotation to partner in returned document and scene, unchanged assertions: named rotation oracle fails; remove control and named passes.
- Legacy named zero-angle test: one pass, zero ignored.
- Module: cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests:: : all pass, zero ignored.
- Full: python3 scripts/check.py : all six checks pass.
- Inspect final diff and quick review; no production changes or controls remain.

## Auto Run Result — historical halted run

Historical status: blocked (resolved by the approved Plan Change Log above). Sofia refinement and independent Ticket validation completed without findings. Refinement/clear-block commit: 5c88ca4abfa0f3dfde5bb202e9960d13283574c2. The source draft adds a separately named 23-degree cell-rotation scenario sharing the original assertions; accepted source/partner rotation checks pass before the preserved displacement assertion fails.

Named command in Verification ran under FD9 lock: compiled; 0 passed, 1 failed, 0 ignored, exit 101. Failure at keycap_resize.rs:1670: source column 0 must reflow by -19. Log: .evidence/a2-linked-rotation/named.log. Production unchanged. This is an observed test/criteria failure, not an established product defect; investigate rotated footprint reflow expectations separately before changing the fixture or numeric oracle.

No temporary controls, restored named run, legacy/module/full checks or Build quick review ran after this failure. No implementation commit, push, PR, acceptance or epic closure. Source and plan drafts remain for inspection. Protected files remained unread/untouched/excluded; A1 was then built/unaccepted (subsequently accepted separately). Gortex source write failed twice with view_building separated by 30 seconds; authorized exact native fallback used.


## Auto Run Result

Status: blocked — new full-check browser failure, not the historical displacement criteria conflict.

- Approved Sofia replan committed separately as `6fa4566d75fc518688c8119133010cfaadc20da2`; independent Ticket check identified stale state wording, corrected before Build. Original baseline and `.evidence/a2-linked-rotation/named.log` preserved.
- Native Gortex tools absent in this resumed turn; used default tools under user's explicit failover authorization. No daemon/index changes performed here.
- Test implementation shares the existing both-halves history assertions between separately named 0° and 23° CELL scenarios. Independent expected displacement constants are set before planning; no production reflow/extent helper or post-edit result supplies them. Zero-angle ±19 X/zero Y and complete A1 metadata expectations retained.
- All Cargo/check commands used `bash -c 'exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9; <command>'`, sequentially; configured flags unchanged. Logs under `.evidence/a2-linked-rotation/`.
- `cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::replace_document_reflows_nonzero_linked_rotation_through_history -- --exact`: 1 passed, 0 failed, 0 ignored (`geometric-named.log`).
- Same command with temporary reply-boundary copied-source-angle control: compiled and executed 0 passed, 1 failed, 0 ignored, exit 101 (`copied-angle-control.log`), failing `linked partner reflected rotation at column 0`. Both partner document and scene angles were changed together; document/scene consistency passed before the rotation oracle. Exact control removed; same command restored to 1 passed, 0 failed, 0 ignored (`restored-named.log`).
- `cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::replace_document_reflows_source_and_core_syncs_linked_partner_through_history -- --exact`: 1 passed, 0 failed, 0 ignored (`zero-angle.log`).
- `cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::`: 11 passed, 0 failed, 0 ignored (`module.log`).
- `python3 scripts/check.py`: exit 1 (`full-check.log`). Required browser step failed in `parts::generator_settings::tests::upload_preview_refresh_keeps_a_later_failed_apply_draft`, `web/crates/parts/src/parts/generator_settings.rs:2649:33`: `generator upload incomplete after 150 polls; binding=None; assets=[]; queued effects=[]`. Scheduler identifies 33 later panic-hook failures and later timeout as follow-on symptoms. No retry or out-of-scope parts fix performed. This is newly observed verification failure; causality is not established.
- `git diff --check` passes. Production portion before `#[cfg(test)]` byte-identical to original baseline. Review diff inspected at `/home/chris/.cache/tmp/a2-review-current.diff`, limited to authorized A2 ticket/plan/source. No controls remain. Quick review and implementation commit did not run because Step 3 verification failed; resume at verification after the parts browser failure is resolved. No A2 acceptance, production edits, push, PR or closure. Protected untracked files preserved unread/untouched/excluded; authorized source and plan drafts remain uncommitted.


### Authorized bounded verification retry — 2026-11-16

- User authorized exactly one unchanged full-check retry. Ran `bash -c 'exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9; python3 scripts/check.py > .evidence/a2-linked-rotation/full-check-retry.log 2>&1'` without a process timeout or overlapping builds.
- Retry exit 0: all six steps passed — repo, tooling, build, lint, test, browser; total 652.4s. Application browser execution: 263 executed, 0 failed.
- Original `full-check.log` and its failure evidence above are preserved unchanged. Passing retry does not prove flakiness or baseline attribution; causation remains unproven. No parts-test or production edits and no further retries. Focused named/control/restored/zero-angle/module checks were not repeated because source was unchanged.
- Verification block cleared on the passing authorized retry; proceeding to required quick inspection review. This supersedes the preceding halted run's pending-review status only.


### Final result — A2 built, pending human acceptance

- Quick inspection review completed: one finding rejected as false against the approved accepted-axis contract; zero patches, zero deferred findings. No further review recommended. No source changes after the recorded focused verification or unchanged full-check retry.
- Implementation changes only `web/crates/layout/src/objects/keycap_resize.rs` and this plan in the final commit. Ticket replan commit remains separate. Production section is byte-identical to original baseline; temporary controls absent. Full both-halves history, identities, sizes and complete adoption-aware metadata retained.
- Verification: named nonzero 1/0 ignored; copied-angle control compiled and failed at rotation oracle; restored named 1/0 ignored; zero-angle 1/0 ignored; module 11/0 ignored. Original full check failed as recorded; exactly one unchanged authorized retry passed all six steps. `git diff --check` passes.
- Built pending human acceptance; no A2 done transition, pilot acceptance, epic closure, push or PR. Only protected pre-existing untracked files remain outside this work and are unread/untouched/excluded.
