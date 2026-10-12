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

- [ ] S1: a test shows the axes choice follows the saved constraint (or the default for the
      selected part's source) after a selection change, an undo of a conversion and a reset; the
      initial signals and both resync paths take it from one pure function.
- [ ] S2: converting with a missing source, a missing target or a mirror constraint is refused
      and leaves the document unchanged (constraints and every part pose).
- [ ] S3: dragging a board-axes constrained companion, and moving it through a linked-layout
      edit, retunes the stored offset for both kinds, with a non-zero turn of the source.
- [ ] D1: a board-axes drag keeps the stored rotation's exact bits, including `-0.0`, and an
      exact-equality test pins it.
- [ ] S4: the inspector's drafts function is tested for a selection change, an undo of a
      conversion, a reset and the default by source for a second part.
- [ ] Refactor (separate commit, behavior unchanged): M1 one `Constraint::offset_axes`, M2 one
      guarded retune, M3 one `Constraint::is_finite`, M4 one `<select>` mapping, M5 and M6 one
      named drafts struct, M7 if trivial.
- [ ] `python3 scripts/check.py repo` and `python3 scripts/check.py test` pass.

## Not in this ticket

- M8: the inspector's private `ConstraintKind::Offset` now means either offset kind while
  `web/crates/layout/src/matrix_relations.rs` uses it for board axes only. CONTEXT.md says to
  qualify "offset"; the rename is the user's decision.
- D2: the test oracle `relative` in `core/tests/part_offset_constraint.rs` hand-rolls the inverse
  rotation. An independent oracle is defensible.
