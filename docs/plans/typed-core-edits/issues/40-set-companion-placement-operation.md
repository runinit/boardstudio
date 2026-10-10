# 40: Core operation to edit the companion placement mode

Status: ready-for-agent
Type: build
Blocked by: [19](19-reconcile-mirrored-halves.md), [39](39-stabilisers-and-companion-fallback.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-stabilisers-fallbacks-and-editing-the-mode-2026-10-10)

## Request

Ticket [31](31-companion-placement-modes.md) set the mode at creation only. The mode (and tweak) must be editable afterwards for
one companion, a row or a column, a whole matrix, one part, or several parts at once.

## What to do

- `EditOperation::SetCompanionPlacement {pairId, targets, mode, tweak}` where `targets` is a list of selectors: `item` (a
  correspondence id: one companion or part), `row`/`column` (matrix id and index), `matrix` (matrix id) and `pair` (the pair default). Core
  expands selectors to correspondences; unknown or already-removed targets retire per ADR-0005 (resolve against the accepted document at execution).
- Applying a mode re-derives only the affected companions (reconciliation from ticket 19), keeps other items' modes and tweaks, reports the changed
  ids exactly, and is one Undo step. Validation as in ticket 31 (mode names, finite tweak, tweak only with mode 3). Stabilisers stay unrotated (39).
- Typed intent through `ResolveEdit` (ADR-0005): no `ReplaceDocument`.
- Tests: each selector kind, a multi-target mix, overlapping selectors (last writer wins deterministically), a pair-default change that leaves explicit
  overrides alone, a rotated map, Undo, refusals leave the document byte-identical; mutation-check expansion and the override rule.

## Acceptance

- [ ] All selector kinds work and report exact changed ids; one Undo step; refusals are atomic.

## Verification

`repo lint test typecheck`; boundary parity with an additive transcript entry; browser step only at the batch boundary.
