# 143b: Review fixes for the part-offset constraint

Status: resolved
Type: build
Blocked by: none
Related: [143](143-part-offset-constraint.md)
Decision: [ADR-0011](../../../adr/0011-part-offset-is-its-own-constraint-kind.md)

## Request

A review of the part-offset series (`d75d50d3e..482cf8906`) found one suspected defect, three
acceptance gaps in the tests and one bit-level behavior change. This ticket closes them without
changing the meaning of `offset` or `mirror`, `CURRENT_VERSION`, or the golden transcript, and
without touching `core/src/constraints/relation.rs` or `endpoints.rs` (139's files).

## Findings

- **S1, stale axes in the inspector.** `offset_axes` is seeded once at mount and not resynced
  when the selection changes, a conversion is undone or the drafts are reset, although the
  Offset X / Y / rotation drafts are. Reproduce with a test before any fix; if the seam cannot
  reproduce it, record that here and leave the code alone.
- **S2, missing-endpoint refusal.** Ticket 143 slice 2 says a missing endpoint changes nothing;
  only an unknown constraint id is tested. A missing source, a missing target and a mirror
  constraint are not.
- **S3, retune pinned for `part-offset` only.** The board-axes `offset` retune through
  `MoveParts` and through a linked-layout edit has no assertion, and no test uses a non-zero
  turn.
- **S4, inspector wiring.** The default by source and the sync of the axes choice are covered
  only through helpers that the component does not use for its resync paths.
- **D1, operation order.** `MoveParts` on a board-axes constraint now runs `*rotation += 0.0`,
  which rewrites a stored `-0.0` rotation to `+0.0`. The old code did not touch the rotation.

## Acceptance

- [x] S1: the axes choice follows the saved constraint (or the default for the selected part's
      source) after a selection change, an undo of a conversion and a reset; the first render
      and both resync paths take source, axes and numbers from one pure function
      (`offset_drafts`). Confirmed first with a mounted-component repro (red: "part" stayed
      after selecting an unconstrained part).
- [x] S2: converting a mirror constraint is refused with the document unchanged (edit API
      test). A missing source and a missing target are refused with the document unchanged
      (in-crate unit test in `core/src/constraints.rs`; the edit API cannot reach them because
      an accepted document never holds such a constraint, so the guards are defensive).
- [x] S3: dragging a board-axes constrained companion, and moving it through a linked-layout
      edit, retunes the stored offset for both kinds, with the source turned 30 degrees; a
      back-solved linked component stores a non-zero turn for both kinds.
- [x] D1: a drag keeps the stored rotation's exact bits, including `-0.0`, for both kinds, and an
      exact-equality test pins it (red before the fix: the drag rewrote `-0.0` to `+0.0`).
- [x] S4: one mounted-inspector test drives the first render, a selection change to an
      unconstrained part, a conversion and its undo, and the Properties reset, and reads the
      axes, source and Offset X the form shows; the pure `offset_drafts` cases cover the rest.
- [x] Refactor (separate commits, behavior unchanged): M1 `Constraint::offset_axes`,
      `offset_numbers` and `is_mirror` at every kind switch, M2 `retune_offset` reports a mirror
      instead of panicking and owns the source lookup, M3 `Constraint::is_finite`, M4 one
      `<select>` mapping, M5/M6 the named `OffsetDrafts` struct for the drafts, M7 both items.
- [x] `python3 scripts/check.py repo` and `python3 scripts/check.py test` pass.

## Not in this ticket

- M5's other clumps, left as judgement calls for the user: `offset_numbers` returning a
  `(Vec2, f64)` pair, and `LayoutConstraintValues::{Offset, PartOffset}` carrying the same
  fields (changing the enum rewrites the existing remainder tests that construct it).
- A `Constraint::offset(axes, ..)` constructor, reusing the `OffsetAxes` serde names for the
  select, removing `default_offset_axes`, aligning the test helpers, and a different error
  channel for `Retune`.
- M8: the inspector's private `ConstraintKind::Offset` now means either offset kind while
  `web/crates/layout/src/matrix_relations.rs` uses it for board axes only. CONTEXT.md says to
  qualify "offset"; the rename is the user's decision.
- D2: the test oracle `relative` in `core/tests/part_offset_constraint.rs` hand-rolls the inverse
  rotation. An independent oracle is defensible.

## Outcome

- Commits, in order: claim; S1 inspector resync; S2 mirror refusal; S3 board-axes retune and
  non-zero turn; D1 rotation bits; refactor M1-M7; clippy literal and native baseline; then the
  follow-up round: the in-crate missing-endpoint test (the Open-only test it replaces is
  dropped), the mounted inspector test, the last kind switch and `is_mirror`, Core tests for
  `offset_axes`, `offset_numbers` and `is_finite`, the doc comment naming the D1 test, and this
  ticket.
- The effect that resyncs the form captures `board_parts` and `part_id` from the first render.
  That is **not** a defect: the mounted test selects another part with a different first other
  part and the form follows it, and a mutant that pins the first part id is killed by the same
  test. No change was made there.
- Mutations (all killed): board retune rotates the drag; mirror conversion silently succeeds;
  linked edit drops the turn; a drag adds `0.0` again (D1); part-offset retune skips the
  rotation into the key frame; conversion without its target or source guard; the resync or the
  reset forgetting the axes; the resync using the first part id; `is_finite` ignoring the turn
  or a mirror coordinate; `offset_axes` swapped; `offset_numbers` skipping part-offset.
- Found along the way: the `excessive_precision` lint failed on a literal in
  `converting_a_part_offset_to_board_axes_moves_no_part_and_keeps_the_id` at 482cf8906; fixed.
- Checks: `check.py repo` and `check.py test` pass; the native name baseline records the new
  Core tests. The inspector, `offset_axes_tests` and `layout_remainder_tests` browser modules
  pass; the full `browser` step was not run.
