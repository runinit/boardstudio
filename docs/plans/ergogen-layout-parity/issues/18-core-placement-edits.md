# 18: Implement named placement, inverse moves and exact origin compensation

Status: claimed
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md), [17c](17c-kernel-production-routing.md) (parent: [17](17-cut-over-format-and-model.md))
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Extend existing typed placement/splay seams for Spread, Padding, row/key Shift/Bend,
Column rotation, independent offsets and origin hold-still/swing. Core owns inverses,
exactness and changed IDs; callers do not rebuild matrices with their own geometry.

## First RED / GREEN slice

Fail a public prepared edit that moves a key in a turned/curved column by its requested
world delta without moving its later keys. Implement the offset-frame inverse; then
add one carried Shift/Bend case and its preview/cancel/Undo behavior.

## Subsequent acceptance

- Field, gesture and nudge inputs resolve against accepted state. Preserve companion
  carry/constraint precedence and independent key identity, changed IDs and no-op rules.
- Column rotation affects only its column; switching Splay scope changes no value.
- Origin hold-still uses this/next step compensation, with matrix Origin for column 0;
  cover Column rotation, previous Splay, Bend, mirrors and nonzero custom origins.
- Inverse-solve a dragged/Picked final origin after compensation, rather than merely
  transforming its initial pointer delta. Fields/nudges hold; Ctrl swing writes origin
  only and is locked at pointer-down; Reset holds and Ctrl base snap swings.
- Implement reusable Core rigid-column translation for Align, with the correct next
  step compensation under Splay, not column/per-key offset scattering under Bend.
- Refuse nonrepresentable exact requests without changing state. Keep Local ownership
  promises; anchor/linked paths in 19–21 extend the same implementation, not another
  inverse. Preserve explicit Spread/pitch behavior and one operation/Undo per edit.
- Keep Y-only Convert exact eligibility: no Splay, Column rotation or carried Bend;
  transfer to this Stagger, subtract next, clear offset. Core owns the blocked reason.

## Checks

Extend `core/src/matrix/{placement,splay,layout}.rs` and existing placement, companion,
splay reconciliation and part-offset tests through public Core/Session routes. Run
focused `boardstudio_core` tests, then quick checks. Private numeric tests supplement,
never replace, prepared-route preview/commit/refusal/Undo evidence.

## Progress

### Parallel implementation claim — 2026-10-23

Claimed for an isolated implementation lane. Acceptance, TDD, review and gates are
unchanged; integration owns shared baselines, test aggregation, map and closure.
