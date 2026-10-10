# 40: Core operation to edit the companion placement mode

Status: ready-for-agent
Type: build
Blocked by: [19](19-reconcile-mirrored-halves.md), [39](39-stabilisers-and-companion-fallback.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-stabilisers-fallbacks-and-editing-the-mode-2026-10-10)

## Request

Ticket [31](31-companion-placement-modes.md) set the mode at creation only. The mode (and tweak) must be editable afterwards for
one switch companion, the companions in a row or column, all companions in a matrix,
or several selected companions at once.

Human clarification, 2026-10-10: **companions only**. Independent parts, including
controllers and connectors, keep the general mirror rule and are excluded from these selectors.

## What to do

- `EditOperation::SetCompanionPlacement {pairId, targets, mode, tweak}` where `targets` is a list of selectors:
  `item` identifies a switch-companion correspondence by `(kind, mirrorId)`; normalize a selected source or mirror
  identity to that tuple. `row`/`column` (matrix id and index) and `matrix` (matrix id) expand to switch-companion
  entries only, never independent controllers/connectors or switches themselves. `pair` changes the companion
  default. Resolve against the accepted document at execution; unknown or already-removed targets retire per ADR-0005.
- One operation supplies one mode/tweak for all selectors: deterministically deduplicate overlapping companion
  targets before applying it. There is no selector-specific last-writer rule. Pair-default changes preserve explicit
  item overrides. Mixed pair/item selector behavior must follow the reviewed default/tweak contract below.
- Applying a mode re-derives only the affected companions (reconciliation from ticket 19), keeps other items' modes and tweaks, reports the changed
  ids exactly, and is one Undo step. Validation as in ticket 31 (mode names, finite tweak, tweak only with mode 3). Stabilisers retain
  39's mirrored-X position and unchanged heading in every mode; a tweak does not change that pose.
- Typed intent through `ResolveEdit` (ADR-0005): no `ReplaceDocument`.
- Tests: each selector kind, a multi-target mix, overlapping selectors (deterministic deduplication), a pair-default change that leaves explicit
  overrides alone, a rotated map, Undo, refusals leave the document byte-identical; mutation-check expansion and the override rule.

## Acceptance

- [ ] All selector kinds affect eligible companions only and report exact changed ids; one Undo step; refusals are atomic.
- [ ] Independent controllers/connectors keep the general mirror rule; stabiliser poses retain 39's invariant.

## Verification

`repo lint test typecheck`; boundary parity with an additive transcript entry; browser step only at the batch boundary.

## Pair-default tweak representation: pending technical review

The human scope decision above is settled. Root must review this technical contract before
implementation; this draft does not authorize code or an additional pair-wide tweak field.
The stored pair has a default mode only; tweaks belong to item associations (31).

Proposed minimum representation: a `pair` selector changes only the stored default mode and
accepts no tweak payload. Default mode 3 with no item tweak means zero adjustment, including
future companions. A nonzero tweak is expressed through companion item/row/column/matrix
selectors and stored per item. Explicit overrides remain untouched; no independent-part frame
or heading rules are introduced. This is a **proposal**, not an approved validation rule.

Root also needs to specify compatible transitions when inheriting items already retain tweaks
and the pair changes away from mode 3, and the behavior of a mixed pair/item target list with
one shared tweak payload. Do not silently erase retained tweaks, invent a pair tweak, or resolve
these cases with arbitrary selector ordering. Add atomic-refusal and inheritance tests once
that technical contract is reviewed.
