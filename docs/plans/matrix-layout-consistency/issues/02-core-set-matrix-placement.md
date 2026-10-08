# 02: Core `SetMatrixPlacement` and the stagger axis

Status: claimed
Type: build
Blocked by: 01
Spec: [spec.md](../spec.md#core-owns-the-parameter-rules) · Map: [map.md](../map.md) · Decision: [ADR-0007](../../../adr/0007-matrix-gestures-edit-named-parameters.md)

## What to build

1. `EditOperation::SetMatrixPlacement { matrix_id, change: MatrixPlacementChange }`
   in `core/src/model.rs`, next to `SetMatrixSplay`, with
   `MatrixPlacementChange::{ColumnStagger { column, value }, ColumnOffset { column, value }, RowOffset { row, value }}`.
   Use serde names in the style of `SetMatrixSplay`. Ticket 07 adds
   `ColumnOffsetToStagger`; leave room for it, but do not add it here.
2. Apply it in `core/src/lib.rs::apply`, as the `SetMatrixSplay` arm does. Find
   the matrix, build the next matrix in a new `matrix::placement::update(current,
   &change)` (index in range, value finite, pad vectors with zeros, change only the
   named entry), then `matrix::set_matrix` and `layout::sync`. Add the variant to
   the preview admission set (`lib.rs` ~line 503) and to the outline-affecting set
   (~line 1020), matching `SetMatrixSplay`.
3. `MatrixColumnBasis.stagger_axis: Vec2` (`staggerAxis`), computed in
   `column_basis` as `location(stagger_c + 1) − location(stagger_c)` for the column's
   row-0 cell. This is exact because `location` is affine in stagger. It keeps
   `location` as the only formula.
4. Regenerate `core/tests/fixtures/boundary/transcript.json` with the repo's
   documented fixture route. Check that only `staggerAxis` and the new operation's
   entries change.

## Impact

Rerun `impact` upstream on `apply`, `column_basis` and `EditOperation` before
editing. Expect wide reach for `apply` (every edit) and `EditOperation`
(serialization across application and web). Report the risk level in the Outcome.
Exhaustive matches on `EditOperation` in `application/` and `web/` must gain the
variant. Find them by compiling, not by grep alone.

## Tests (tdd; native `core/tests/matrix_placement.rs`, new)

RED first:

1. `stagger_axis_matches_location`: covers plain, rotated 30°, mirror X, mirror Y,
   splay with a default origin, a custom origin on the column itself, and a custom
   origin on an earlier and on a later column. Raising stagger `k` by 1 moves column
   `k`'s keys by exactly `staggerAxis` (1e-9). It also confirms the spec's derived
   formula. If they disagree, Core wins: correct the spec.
2. `column_stagger_moves_this_and_following_columns`: staggers `[0, 2, −1]`, set
   column 1 to 5. Columns 1 and 2 move +3 local; column 0 does not. The stored value
   is `[0, 5, −1]`.
3. `column_offset_moves_only_its_column` and `row_offset_moves_only_its_row`.
4. `companions_follow_placement_changes`: an attached diode follows. An authored
   companion keeps its relative pose. A constrained companion keeps its constraint
   (reuse `core/tests/key_move_assemblies.rs` fixtures).
5. `linked_halves_sync`: on the `linked_pair` fixture (copy it from
   `core/tests/core.rs`), editing either half updates both. Keys stay reflected.
6. `invalid_changes_error`: an out-of-range index or a non-finite value errors and
   leaves the document unchanged. Setting the current value is a no-op.
7. `preview_and_undo`: a preview restores the document; a commit is one Undo step.
8. `changed_ids_are_precise`: the matrix ID plus the moved members and companions,
   with no unrelated parts.

## Acceptance

Tests 1–8 pass. `lint`, `test` and `typecheck` pass. Run `detect-changes --scope all`
before committing.

## Files

Owner of `core/src/model.rs` (`EditOperation` and scene types), `core/src/lib.rs`
(apply/preview/outline sets), `core/src/matrix.rs` and the new
`core/src/matrix/placement.rs`, and the transcript fixture, until 03 starts.
