# 145: Mirror new matrices on a mirrored board

Status: ready-for-agent
Type: build
Blocked by: [110](110-inverse-map-mirrored-half-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [115](115-prepare-mirrored-edit-transaction.md), [111](111-integrate-mirrored-half-reconciliation.md), [108](108-reconcile-mirrored-outlines-assemblies.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

A user adds a base matrix on a mirrored board and then a thumb cluster. The new matrix must be
mirrored and linked too: the other half gets the reflected matrix, its keys and companions, the
modules and wiring that follow its members, and a stored `Matrix` correspondence so later edits
from either half reconcile. Today nothing mints a matrix correspondence after creation.

## What 110 supplies

`mirrored_half::inverse::canonicalize` (private, inert) answers a `SetMatrix` whose matrix id is
new and sits on a board a pair relates with:

- the source-equivalent `SetMatrix` (a mirror-board addition is inverted through the pair map, its
  companions under the pair's default placement mode; a source-board addition is returned as
  written);
- `matrix_additions`: one planned `MatrixAddition { source_id, mirror_id, source_board_id,
  mirror_board_id, members }`, with every key and companion as a `(source id, mirror id)` pair.
  Identities are allocated free of existing matrices and correspondences.

## What this ticket owes

The forward side, which does not exist on dev: 105's derivation kernel only derives an existing
mirror, and 106–108's membership transitions add members to a matrix that already corresponds
(`PlannedAddition`, `membership_transition_expands_with_planned_pair_identities_and_references`);
108 records that minting new mirror correspondences belongs to the membership slices.

- A transition that, given a `MatrixAddition`, creates the mirror matrix on the mirror board
  (`layout::reflected` or the established derivation), its members with the planned identities,
  and records the `Matrix` correspondence plus one `Part` correspondence per member pair.
- The same for what follows a matrix: layouts, nets the member pins use, modules and circuits
  hosted on its members, outline and mechanical references already covered by 107/108.
- Undo/redo, Open repair and preview treat the pair as one transaction (with 115–117).
- Refusals: a planned identity that has become taken, a board pair in a Local/Omitted state
  that cannot receive it, and a matrix also linked by a legacy layout mirror link.

Acceptance: a red-first test adds a base matrix and then a thumb cluster on each half and sees
both mirrored and linked, with correspondences, exactly-once members and unchanged unrelated
geometry; editing the new matrix from either half reaches the other.

## File ownership

Private mirrored-half transition modules; established mirrored-half tests. No production call
site before 111 activates the prepared path.
