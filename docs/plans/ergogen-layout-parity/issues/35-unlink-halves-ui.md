# 35: Unlink halves in the UI

Status: ready-for-agent
Type: build
Blocked by: None (all dependencies resolved; linked-half reflection and the exception lifecycle already exist)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Let the operator unlink a linked pair of halves from the UI with a
confirmation step, so unlinking ends the correspondence cleanly, leaves both
halves independently valid, and undoes as one entry.

## First RED / GREEN slice

Fail a mounted test: unlinking a linked pair through the confirmed UI action
leaves two valid independent halves with no live correspondence, and a
single Undo restores the linked pair with poses unchanged.

## Subsequent acceptance

- [ ] The unlink action asks for confirmation naming the consequence, then
      writes one edit with one Undo entry.
- [ ] After unlink, no orphaned correspondences remain: neither half follows
      the other and neither half references the former pair.
- [ ] Both halves stay valid after unlink: dimensions, layout membership and
      anchors hold, or the action refuses atomically with a reason.
- [ ] Relations and the canvas stop showing the pair relationship once
      unlinked and restore it on Undo.

## Checks

Mounted browser tests for confirmed unlink, no-orphan proof, and Undo
round-trip; affected quick/browser checks.
