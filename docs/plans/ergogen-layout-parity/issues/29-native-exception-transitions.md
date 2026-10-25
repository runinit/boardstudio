# 29: Native matrix-level exception transitions

Status: claimed
Type: build
Blocked by: none (decisions locked in [deferred-items-plan.md](../deferred-items-plan.md))
Parent: [deferred plan](../deferred-items-plan.md) · Research: [deferred-items-research.md](../deferred-items-research.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Full ticket-21 mirror on the native (mirrored-half) pair, slices
B1–B5. Same-board behavior stays owned by [21](21-exception-lifecycle.md);
confirmation UI stays owned by [23](23-inspector-toolbar-and-handles.md)
(reused verbatim, not forked).

## Outcome

Native matrix-level Local/Omitted + membership-change transitions work through
the reconciliation kernel instead of refusing at `validate_matrix_association`:
Local/Omitted survive source resize, Make-Local detach and Rejoin, confirmed
detach, and source removal — mirroring the same-board contract in spec §5
(Local keeps world pose via independent offset/rotation compensation only;
carried Shift/Bend stay shared; Omitted keeps a live logical pose; Rejoin
adopts source and validates atomically).

## First RED / GREEN slice

Fail a public-route test: native pair, matrix-level Local on the mirror
matrix, then source `set-matrix` expansion; the mirror keeps its dimensions
and pose with valid correspondences (mirrors ticket 21's first slice natively).
Seams: `reconcile.rs` gate + `mirrored_half/lifecycle.rs` only; no projection
change yet.

## Subsequent acceptance

- Native Omitted mirror matrix stays live-yet-absent through source
  expansion/shrink.
- Native Make-Local detach keeps pose in one Undo entry; Rejoin adopts source
  dimensions/layout/anchor keeping explicit child exceptions, refusing
  atomically on missing anchors.
- Native detach needing a new Local exception refuses with proposal plus a
  confirmed variant in one entry, reusing ticket 23's confirmation UI
  verbatim; source removal retains a complete Local matrix, retires only
  defunct links, preserves unrelated links.
- Hardening: identity remaps, child exceptions, preview/cancel, exact
  one-Undo Redo, frame/Follow restrictions through both halves' edits — all
  via public `Open`/`Edit` routes, never private-helper tests.

## Checks

Core integration tests via public routes (spec §11 seam rule); affected
quick/browser checks; full-gate evidence for the batch. Normal tests do not
invoke Ergogen/Node.
