# 20: Recipes build one half

Status: ready-for-agent
Type: task
Blocked by: [19](19-reconcile-mirrored-halves.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Current contract (2026-10-11)

This remains optional shared recipe work after the required mirrored-half/link capability in 19/22. It may improve common examples, but it is not a prerequisite for the ordinary saved projects in ticket 25. Do not create per-board recipe branches, builders, behavior suites, or helper extraction. Preserve this ticket's existing Outcome and history below.

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
- [ ] Per-keyboard record in the Outcome. KLOR's second outline equals the first's reflection: its keys, bay,
      reset and TRRS already reflect exactly, but the automatic gap repair fills a pocket on one outline only,
      so the halves' outlines differ by 6.4 mm today (generator-parameters 06 Outcome). Remove the 6.5 mm
      KLOR allowance in `the_split_halves_outlines_are_mirror_images_through_the_pair_map`.

## Verification

`build` (sandbox lifted for wasm-pack), native `test` in the assigned lane worktree, `lint`, `repo`.
