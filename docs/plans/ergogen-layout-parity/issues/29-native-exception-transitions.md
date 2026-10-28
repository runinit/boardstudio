# 29: Native matrix-level exception transitions

Status: resolved
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

## Answer

Resolved on branch `docs/ergogen-layout-comparison` jointly with tickets 28
and 30–32. Slices B1–B5 landed as RED `c39c02af4` / `bfbddaf05` /
`5697114b0` then GREEN `260a954d5` (Local survives expansion), `28c8e17af`
(Omitted live-yet-absent), `f59c54b7c` (Make-Local detach/Rejoin),
`d13bf7d3c` (source removal), `51c07e8dd` (part-level Omitted,
shrink-after-growth), `52321e773` (anchored additions) and `c2424ec50`
(confirmed detach, rejoin adoption; style `0c1d069e4`). The standards review
and the spec re-review both PASSED, and the fix check PASSED, with no open
findings. Joint native baseline `6c0e4f30` (+53/−0). Joint gates green: quick
`check: all steps passed: repo, tooling, lint, typecheck, test, browser`
(468.3s) and full
`check: all steps passed: repo, tooling, lint, build, test, browser`
(419.5s).
