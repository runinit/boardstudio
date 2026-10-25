# 36: Committing test for aggregate anchors

Status: ready-for-agent
Type: build
Blocked by: None (all dependencies resolved; aggregate evaluation and the Inspector aggregate surface already exist)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Lock the aggregate anchor surface with a mounted test that commits a real
aggregate change, so the existing navigate-only coverage gains a regression
proof that method and parts edits write through and undo cleanly.

## First RED / GREEN slice

Fail a mounted test: committing an aggregate method change through the
mounted Inspector surface writes the change with one Undo entry, and Undo
restores the prior method with poses unchanged.

## Subsequent acceptance

- [ ] A mounted test commits an aggregate method switch end to end and proves
      the single Undo and Redo round-trip.
- [ ] A mounted test commits a parts change end to end (add or remove a part)
      and proves dependant poses update and Undo restores them.
- [ ] Refusals surface the published reason inline with nothing written; the
      existing navigate-only coverage keeps passing unchanged.
- [ ] No client-side geometry is asserted: expected poses come from
      Core-published values only.

## Checks

Mounted browser tests for the committing aggregate coverage; affected
quick/browser checks.
