# 20: Recipes build one half

Status: ready-for-agent
Type: task
Blocked by: [19](19-reconcile-mirrored-halves.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Delete the hand-written mirror arithmetic in `core/examples/demo_projects/recipes.rs`
(`x → max_x − x`, negated rotation, reversed columns, separate bay placement) and the
matching Sofle closures and `mechanical.rs` frame checks. Generator-parameters 06 made the
recipe geometry consistent as a stopgap.

## What to do

- Build only the measured half (Lily58 and KLOR build `right`), call `create-mirrored-half`
  and the mechanical edits.
- Re-record `core/tests/fixtures/demo-projects-baseline.json` deliberately: for each of
  the ten keyboards list every renamed id (source numbering) and every moved part; keys
  must match within 1e-9 by correspondence.
- Recorded exceptions: none expected for the shipped pairs; instance role, half and name
  differ by nature.

## Acceptance

- [ ] No mirror arithmetic remains (grep `max_x -`, `-k.rotation`); all 20 fixtures regenerate.
- [ ] Per-keyboard record in the Outcome; KLOR's rotated thumb cluster reflects correctly.

## Verification

`build` (sandbox lifted for wasm-pack), native `test` in a non-nested checkout, `lint`, `repo`.
