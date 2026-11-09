# 145: Mirror new matrices on a mirrored board

Status: resolved
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

## Outcome

Resolved as a private, inert transition: `mirror_added_matrix` in
`core/src/mirrored_half/matrix_addition.rs` (no production call site; 111 activates it). It takes
the document after 110's source-equivalent `SetMatrix` was applied and the planned
`MatrixAddition`, and returns a draft plus the changed ids; the input is never edited and a
refusal returns no draft. Tests: `matrix_addition/tests.rs` (24) and
`matrix_addition/reconcile_tests.rs` (1, a child of `reconcile` so it can build a `Projection`).

What it does: reflects the source matrix through the pair map under the pair's default companion
mode (`layout::reflected` and `derive::companion_assembly`, as `CreateMirroredHalf` does),
places it on the mirror board with the planned identities through `set_matrix`, copies the
member attributes creation copies, and records one `Matrix` and one `Part` correspondence per
member pair. It then carries what follows the matrix: layouts of the source matrix, nets a new
member pin joins (an existing linked net gains the mapped pins, any other becomes a mirror net
with a `Net` correspondence and a board registration), modules connected to a new member
(`mirror_module`, with a `Module` correspondence) and the keycap settings of the matrix and its
keys. The record is checked with `layout::validate` and `halves::validate_document`.

Refusals, each named by identity: no pair relates the boards, or the pair reuses one board; the
electrical-board correspondence is Local or Omitted (the board pair cannot receive it); a
legacy layout mirror link on either matrix; a planned mirror matrix or member identity that has
become taken or is already named by a correspondence; a source matrix or member that is not on
the source board; a net of an embedded circuit that a member pin joins (112); a layout part or
module connector with no linked mirror; a tweak pair with no one stored tweak to give.

Acceptance, evidenced: a base matrix, then a thumb cluster added on the source half and one on
the mirror half, are each mirrored and linked, with exactly-once members and unrelated parts and
matrices byte-identical. Editing from the mirror half names the source through the stored
correspondence (110's seam). Editing from the source half reaches the mirror through the
existing reconciliation (`reconcile` with a `Projection`), and the result equals the mirror the
creation path derives for the edited source. The strongest oracle: for a translated, a slanted
and a rotated map, in all three companion modes with a stabiliser, the added mirror matrix, all
its member parts (except the generated reference) and its correspondences equal what
`CreateMirroredHalf` produces when the same matrix exists before creation.

Evidence: red first (stub returning an error, 14 tests failing at the transition), then green.
Mutation run, 23 mutants over the transition (`.evidence/ticket145/`): 22 killed, 1 survives.
Four first-round survivors (member identity message, source board, final validation, module
instance) led to stronger tests and were then killed. The survivor, `planned != placed` in
`check_generated_members`, is a defensive duplicate of the plan validation that runs before it,
and could not be made reachable.

Judgement calls:

- **Where it lives.** A sibling module of `inverse` (not under `reconcile`), which reuses the
  creation path's private helpers. The only shared-file edits are two `mod` lines
  (`mirrored_half.rs`, `reconcile.rs` for the one reconcile-side test) and the `pub(super)` test
  fixtures.
- **Companions under the pair default.** A new companion takes the pair's default mode and no
  per-item override. In a tweak pair it takes the one tweak the existing companions share; with
  none, or several, the transition refuses rather than guesses. (110 refuses the mirror-side
  addition in such a pair already, since its inverse needs a stored tweak.)
- **Source-local companions.** A cell whose companions are local to the source has none on the
  mirror (as `project_matrix` treats it), so those companions get no correspondence.
- **Matrix name.** The mirror's name is the source name with the board token substituted, which
  reverses the substitution 110 applies to a mirror-side addition.
- **Electrical-board overlay as "board pair state".** The ticket's Local/Omitted board pair is
  read as an overlay on the pair's electrical-board correspondence, the only board-level
  correspondence kind the model has.
- **Outline and mechanical references need no transition code.** Placing a matrix lists its
  members in that board's own envelope outline features, so each half registers its own
  members; 107/108's reconciliation maps anything beyond that.

Not done:

- Transaction behaviour (undo, redo, Open repair, preview treating source request and mirror
  draft as one step) belongs to 115-117; this slice is a pure draft-to-draft function.
- The mirror board's electrical configuration (pin locks, assignments) and the controller are not
  extended for the new matrix: assignments that name a new member are wiring edits that follow.
- Embedded circuits and their private nets on a new matrix are refused, not translated (112).
- Sparse source cells (fewer cells than `rows x columns`) pass through as stored; the existing
  reconciliation requires a fully stored cell list, so 111 must normalise a matrix before it
  reconciles. The forward transition itself does not depend on it.
- Not run against the production apply path: no call site exists before 111.
