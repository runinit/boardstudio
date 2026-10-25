# 34: Per-key Omit and Rejoin in the matrix surface

Status: ready-for-agent
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
