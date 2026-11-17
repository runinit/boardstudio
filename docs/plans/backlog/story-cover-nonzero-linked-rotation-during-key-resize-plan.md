---
title: 'Cover nonzero linked rotation during key resize'
type: chore
ticket: 4
created: '2026-11-16'
status: ready-for-dev
baseline_revision: '5c88ca4abfa0f3dfde5bb202e9960d13283574c2'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: []
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
