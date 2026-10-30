# 08: Slice the build

Status: resolved
Type: task
Blocked by: [07](07-prototype-split-editing-view.md)
Map: [map.md](../map.md)

## Question

Slice the approved 04/05/06 decisions and the 07 prototype into TDD build tickets.

## Outcome

Dependent build tickets with acceptance criteria, ready for implementation work.

## Answer

Sliced on 2026-10-25 into build tickets 09–18; dependency order shown, frontier first.

- [09](09-split-half-frame-contract.md) Split half frame and stored gap (contract prefactor; ADR). Blocked by: none.
- [14](14-dim-unfocused-half.md) Dim the unfocused half; click to focus. Blocked by: none.
- [15](15-placement-breadcrumb-retire-relations.md) Placement breadcrumb and Relationships; retire the Relations tab. Blocked by: none.
- [16](16-field-hover-moves-n-keys.md) Restore the per-field amber "Moves N keys" preview. Blocked by: none.
- [10](10-adaptive-spacing-on-commit.md) Spacing adapts on every committed edit. Blocked by: 09.
- [17](17-canvas-anchor-badge.md) Canvas anchor badge for the selection. Blocked by: 15.
- [11](11-adaptive-gap-at-creation.md) Adaptive gap at Mirrored pair creation. Blocked by: 10.
- [12](12-half-frame-parts-and-mirrored-controller.md) Parts in the half frame, split clamp, mirrored controller by default. Blocked by: 10.
- [18](18-spacing-change-ghost.md) Ghost of the previous spacing. Blocked by: 10.
- [13](13-part-anchoring-column-vocabulary.md) Part anchoring with the column vocabulary. Blocked by: 12, 17.

Primary design source: the prototype on local branch `prototype/split-board-editing`
(ef1c8a008), which is kept for comparison and refinement after implementation. The carried 07
points are acceptance items in 10, 11, 12 and 13.
