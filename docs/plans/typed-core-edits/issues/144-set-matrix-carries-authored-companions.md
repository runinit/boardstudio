# 144: A whole-matrix edit carries authored companions with their keys

Status: resolved
Type: build
Blocked by: none
Related: [143](143-part-offset-constraint.md)

## Request

A user drags a diode or LED away from its generated place, then moves, rotates or mirrors the
whole matrix. The keys move and the authored companions stay where they were. Carry them with
their keys, the way a column splay and a placement edit already do.

## Evidence this ticket starts from

- `SetMatrixSplay` and `SetMatrixPlacement` run through
  `matrix::placement::carrying_authored_companions` (commit `30903fe4b`), which moves an
  authored companion rigidly with its key: the offset turns about the key and the turn is added.
- The `SetMatrix` arm in `core/src/lib.rs` is `set_matrix` plus `layout::sync` and carries
  nothing. Generated companions follow through `set_matrix`; a part with `layoutOverride == 1`
  keeps its pose.
- The web matrix transform maps these fields to `SetMatrix`
  (`web/crates/layout/src/matrix_transform_operation.rs`): matrix origin X and Y, matrix
  rotation and matrix mirror. Key-level fields use the same operation too.
- Read from the code, not run: no test yet moves an authored companion through `SetMatrix`.

## Behavior

- After `SetMatrix` changes a matrix's origin, rotation, mirror or any other value that moves a
  key, each authored companion of that key keeps its offset and turn relative to the key.
- A mirror flips handedness. The carry keeps the companion at the mirrored position of its
  key-relative offset; `Pose2` carries no handedness, so decide and test what "keeps its pose"
  means for a mirrored matrix before building.
- Companions with an explicit constraint keep that constraint, as today.
- `SetMatrix` also creates matrices and replaces definitions. A key that did not exist before
  the edit has nothing to carry, so a new matrix and a regenerated key are unaffected.
- Report the same parts as `SetMatrix` reports today; the golden transcript and the Matrix
  setup tests pin that.

## Acceptance

- [x] A test authors a companion, then changes the matrix origin, rotation and mirror through
      `SetMatrix`; the companion keeps its pose in the key's frame each time.
- [x] Translation-only edits move the companion by exactly the key's delta, bit for bit.
- [x] Linked halves, disabled and deleted cells and locked companions have an explicit test or
      a written reason they are out of scope.
- [x] Existing `SetMatrix` tests and the golden transcript pass unchanged.

## Not in this ticket

- A constrained companion's offset axes, which is [143](143-part-offset-constraint.md).

## Outcome

Implemented `SetMatrix` companion carrying in `3d20a7202` and merged to `dev` at `fb801a92`.
The focused companion, placement, splay, and golden transcript suites passed before merge;
the merged `python3 scripts/check.py repo test lint` passed after correcting the ticket 111 link.
The native test baseline includes the five added tests. Independent Standards and Spec reviews
cleared the change; the reviewed mirror rule reflects the key-relative X offset and heading.

## Comments

- 2026-10-12: Coverage carries an authored source companion through `SetMatrix` and checks that
  the linked mirror's companion remains materialized by the existing layout sync. Full linked
  edit transaction behavior and activation remain with [111](111-integrate-mirrored-half-reconciliation.md).
