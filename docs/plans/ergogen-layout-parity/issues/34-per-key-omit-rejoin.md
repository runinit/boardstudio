# 34: Per-key Omit and Rejoin in the matrix surface

Status: claimed
Type: build
Blocked by: None (all dependencies resolved; the Local, Omitted and Rejoin Core lifecycle already exists)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Let the operator omit a single key from the matrix surface and rejoin it
later from the Inspector, so an omitted key leaves the live pose while
leaving the scan grid, and rejoining adopts the key back into its matrix
with the same safety as the established detach-confirm path.

## First RED / GREEN slice

Fail a mounted test: omitting a single key from the Inspector hides it from
the matrix surface while keeping its logical pose, and a single Undo
restores it with revision unchanged.

## Subsequent acceptance

- [ ] Omitting one key is a single atomic edit with one Undo entry; sibling
      keys keep their poses and no partial omission persists.
- [ ] Rejoining the omitted key adopts it back into its matrix with one Undo
      entry; anchor dependants stay valid or refuse atomically with a reason.
- [ ] Omit and Rejoin match the detach-confirm path: confirmation where the
      path requires it, an accessible blocked reason where refused, and no
      silent geometry loss.
- [ ] Relations explains the omitted state and the rejoin affordance without
      duplicating layout maths.

## Checks

Mounted browser tests for omit, rejoin, atomic refusal, and Undo round-trip;
affected quick/browser checks.

## Progress

- Investigated 2026-10-25: no per-key Omit/Rejoin UI exists in the new
  Inspector (`matrix_inspector*.rs`, `matrix_transform_*`). The transform
  Inspector has a per-key `KeyEnabled` toggle via `SetMatrix` (not an
  Omitted/Rejoin lifecycle), and the matrix Inspector has no omit affordance
  at all. Core offers two candidate contracts with different semantics:
  `SetMatrixCellsEnabled`/`SetMatrix` cell-enabled versus
  `SetHalfException{kind: Part, state: Omitted/Linked}` (+ Rejoin adoption
  in `mirrored_half/lifecycle`), and the ticket's detach-confirm parity
  (`DetachMatrixAnchorConfirmed`) is matrix-level, not key-level.
- The ticket's Blocked-by claims the Local/Omitted/Rejoin lifecycle already
  exists, but it does not name which Core variant is the per-key matrix-surface
  contract, nor which Inspector context (Key vs Matrix) owns the affordance,
  nor how omit interacts with the scan grid vs logical pose vs anchor
  dependants. Choosing the wrong variant would fork the exception lifecycle.
- Per task rules (no model.rs type changes unless required — if so, stop and
  report; and no new Core contract without an explicit ticket decision),
  stopping here for parent decision on the intended Core variant and owning
  surface. No code changed; ticket stays claimed for parent review.
