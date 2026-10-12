# 143b: Review fixes for the part-offset constraint

Status: claimed
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
      source) after a selection change, an undo of a conversion and a reset; the initial signals
      and both resync paths take it from one pure function (`offset_drafts`). Confirmed first
      with a mounted-component repro (red: "part" stayed after selecting an unconstrained
      part), which is not committed; the committed tests are on the pure function.
- [ ] S2: converting with a mirror constraint is refused and leaves the document unchanged
      (done). A missing source or target cannot be tested through the edit API: Core refuses to
      open or accept a document with a constraint that has a missing endpoint, so those two
      branches are unreachable. A test pins the reason; the branches stay untested. Open
      question for the human: drop the defensive branches, or keep them untested.
- [x] S3: dragging a board-axes constrained companion, and moving it through a linked-layout
      edit, retunes the stored offset for both kinds, with the source turned 30 degrees; a
      back-solved linked component stores a non-zero turn for both kinds.
- [x] D1: a drag keeps the stored rotation's exact bits, including `-0.0`, for both kinds, and an
      exact-equality test pins it (red before the fix: the drag rewrote `-0.0` to `+0.0`).
- [x] S4: the inspector's drafts function is tested for a saved kind of either axes, a conversion
      and its undo, the default by source for a second part, and a mirror or off-board source.
      A reset calls the same function, so it has no separate case. That the component calls it
      on all three paths is covered only by the uncommitted mounted repro.
- [x] Refactor (separate commit, behavior unchanged): M1 `Constraint::offset_axes` and
      `offset_numbers`, M2 `retune_offset` reports a mirror instead of panicking and owns the
      source lookup, M3 `Constraint::is_finite`, M4 one `<select>` mapping, M5/M6 the named
      `OffsetDrafts`, M7 both items.
- [x] `python3 scripts/check.py repo` and `python3 scripts/check.py test` pass.

## Not in this ticket

- M8: the inspector's private `ConstraintKind::Offset` now means either offset kind while
  `web/crates/layout/src/matrix_relations.rs` uses it for board axes only. CONTEXT.md says to
  qualify "offset"; the rename is the user's decision.
- D2: the test oracle `relative` in `core/tests/part_offset_constraint.rs` hand-rolls the inverse
  rotation. An independent oracle is defensible.

## Outcome

Status stays `claimed`: the S2 box is not fully met, and that is a decision, not work left.

- Commits, in order: claim; S1 inspector resync; S2 mirror refusal; S3 board-axes retune and
  non-zero turn; D1 rotation bits; refactor M1-M7; clippy literal and native baseline.
- Checks: `check.py repo` passes. `check.py test` passes (1528 native tests; the 21 tests of
  `part_offset_constraint.rs` are now in the name baseline). Browser: the inspector, its
  `offset_axes_tests` and `layout_remainder_tests` modules pass; the full `browser` step was
  not run. Clippy on Core is clean; web clippy was not run workspace-wide.
- Mutations (all killed): board retune rotates the drag; mirror conversion silently succeeds;
  linked edit drops the turn; a drag adds `0.0` again (D1); part-offset retune skips the
  rotation into the key frame.
- Not done: committed coverage of the inspector wiring (the S1 repro stays a local patch),
  the missing source/target branches of `convert_offset` (unreachable), M5's
  `LayoutConstraintValues` duplicate fields, M8 (the user's rename).
- New finding: the `excessive_precision` lint failed on a literal in
  `converting_a_part_offset_to_board_axes_moves_no_part_and_keeps_the_id` at 482cf8906; fixed.
