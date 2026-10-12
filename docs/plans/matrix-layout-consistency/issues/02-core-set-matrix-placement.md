# 02: Core `SetMatrixPlacement` and the stagger axis

Status: resolved
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

## Outcome

Implemented on `typed-core-edits/matrix-core` from `ef45d94d7`:

- `50939cfff` claim; `59b0cbc49` Core implementation; `91ffa779c` boundary coverage.
- `EditOperation::SetMatrixPlacement { matrix_id, change }` with
  `MatrixPlacementChange::{ColumnStagger, ColumnOffset, RowOffset}` (serde
  kebab-case, `matrixId`), applied by `core/src/matrix/placement.rs`. `update`
  validates index and finiteness, pads vectors with zeros, treats the current value
  as a no-op, and changes only the named entry. `apply` routes through
  `matrix::set_matrix` + `layout::sync`, carries authored companions by their key's
  world delta, and reports only members whose projection actually moved.
- `MatrixColumnBasis.stagger_axis` (`staggerAxis`) is computed by differencing
  `location` with the column stagger raised by one, keeping `location` the single
  formula. The spec's derived axis formula matched Core for plain, rotated, mirrored,
  default-origin and custom-origin cases, so the spec was not changed.
- Added the variant to the preview admission and outline-affecting sets, matching
  `SetMatrixSplay`. Web keycap fixtures gained the new scene field.
- Boundary requests now exercise all three variants (preview + commit) through both
  native and WASM transports; the golden transcript changed only by `staggerAxis`
  values and the six new replies.

Impact (GitNexus `impact`, before edits): `apply` MEDIUM (20 impacted, 7 direct);
`column_basis` CRITICAL (22 impacted, 3 processes, 10 modules); `EditOperation`
UNKNOWN (no callers resolved, so exhaustive matches were found by compiling).
`detect-changes --scope compare --base-ref ef45d94d7` and `--scope all` reported a
low-risk change set with no affected processes and no truncation.

Checks run: `core/tests/matrix_placement.rs` 9 tests; `cargo test -p
boardstudio_core` 345 integration plus unit; `cargo test -p
boardstudio-application` 30; `cargo clippy --workspace --locked --all-targets
--no-deps -- -D warnings`; `cargo fmt --all --check`; WASM `cargo check -p
boardstudio-web --target wasm32-unknown-unknown --features page`; native boundary
parity. Standards review found no violations (one optional dedup observation);
spec review's only High finding — the boundary fixture did not exercise the new
operation — is fixed in `91ffa779c`. The WASM browser half of the boundary test was
not run here; `typecheck` covers compilation only.

Behaviour note: `SetMatrixPlacement` carries authored companions (spec point 7)
while `SetMatrix`/`SetMatrixSplay` keep their existing behaviour; that asymmetry is
deliberate and scoped to the new operation. Ticket 03 is now unblocked.

## Files

Owner of `core/src/model.rs` (`EditOperation` and scene types), `core/src/lib.rs`
(apply/preview/outline sets), `core/src/matrix.rs` and the new
`core/src/matrix/placement.rs`, and the transcript fixture, until 03 starts.
