# 40: Core operation to edit the companion placement mode

Status: resolved
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

- [x] All selector kinds affect eligible companions only and report exact changed ids; one Undo step; refusals are atomic.
- [x] Independent controllers/connectors keep the general mirror rule; stabiliser poses retain 39's invariant.

## Verification

`repo lint test typecheck`; boundary parity with an additive transcript entry; browser step only at the batch boundary.

## Pair-default tweak representation: reviewed 2026-10-14

The human scope decision above is settled. Root reviewed this contract against the stored model:
the pair owns only its default mode, while companion tweaks and overrides belong to individual
correspondences (ticket 31). No pair-wide tweak field or independent-part frame rule is added.

- A `pair` selector changes only the default mode. A request containing a pair selector rejects
  any tweak payload; pair and item selectors may be combined when the tweak is absent.
- Item, row, column and matrix selectors set the same mode and optional tweak on their eligible
  switch-companion correspondences. A tweak is valid only with
  `literal-mirror-with-tweak`; omitting it clears the selected items' previous tweak. Resolve
  selectors against the accepted document and deduplicate by `(kind, mirrorId)` before mutation.
- A pair-default change preserves explicit item overrides. Before changing the default away from
  `literal-mirror-with-tweak`, materialize that mode on any inherited correspondence that has a
  stored tweak. This preserves its effective placement and keeps the record valid; new
  correspondences inherit the new pair default.
- Correspondence settings remain editable under Local or Omitted exception overlays so Rejoin
  retains the latest settings. Re-derive geometry only for linked companions. Only actual switch
  companions are eligible; controllers, connectors, switches and unrelated parts are excluded.
- A missing pair, stale item identity or selector with no eligible companions retires without
  mutation. Invalid modes, non-finite tweaks, or a tweak with another mode refuse atomically.

Tests must cover pair-default inheritance and preservation, mode transitions with inherited
tweaks, mixed pair/item selectors without tweaks, rejection of pair-plus-tweak, item identity
from either half, overlapping selectors, Local/Omitted persistence through Rejoin, linked-only
re-derivation, exact changed ids, Undo and atomic refusal.

## Outcome (2026-10-14)

- Core and the physical-setup adapter changes were committed to `dev` as `fd2f217e`.
- Core resolves item selectors from either half, expands row/column/matrix selectors only to
  switch companions, deduplicates overlap, preserves explicit overrides on a pair-default change,
  and keeps local/omitted records editable for Rejoin. The reviewed pair contract above remains
  enforced; no pair-wide tweak was added.
- `cargo test -p boardstudio_core --locked --test integration mirrored_half::`: **114 passed**,
  including exact selector expansion, independent-part exclusion, stabiliser invariants, changed
  IDs, atomic refusal and Undo. The pair-default test now covers preserving an explicit
  `literal-mirror-with-tweak` override and its exact tweak while changing the pair default.
- `scripts/mutation-check.py` killed both the selector-expansion mutant (two tests failed) and a
  pair-default mutant that overwrote every correspondence (the override test failed); files were
  restored byte-for-byte. Evidence is in the ignored `.evidence/ticket40-mutation-results/`.
- The full `python3 scripts/check.py` after the operation was implemented passed repo, tooling,
  lint, build, test and browser. Focused Core tests and `cargo fmt --all --check` passed again after
  strengthening the override regression. Boundary parity passed in that full check.

The mounted Inspector controls are tracked separately by [41](41-inspector-companion-mode.md).
