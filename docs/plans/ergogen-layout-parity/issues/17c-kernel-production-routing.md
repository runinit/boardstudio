# 17c: Kernel production routing

Status: claimed
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md)
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Make the staged kernel the production layout engine, with replacement coverage active before the old oracle goes.

## Acceptance

- Route draft/actual projection and materialized parts through the staged kernel.
- Retire the old TypeScript oracle only with replacement coverage active.
- Cover explicit enabled/deleted/omitted production states (ticket 16 Answer).

## Checks

Source anchors: `core/src/geometry.rs`, `matrix/layout.rs`. Use the committed parity poses through actual scenes, draft projections and materialized parts, plus `check.py quick --files`; require native/contract and WASM compilation.

## Progress

- [ ] 
