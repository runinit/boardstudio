# 03: Core publishes companion placement modes

Status: ready-for-agent
Type: build
Blocked by: 02
Spec: [spec.md](../spec.md#core-owns-the-parameter-rules) · Map: [map.md](../map.md)

## What to build

`MatrixScene.companions: Vec<MatrixCompanionPlacement>` (serde `companions`, skipped
when empty). Each entry is `{ part_id, key_id, mode }`, with
`mode: Follows | Authored | Constrained { constraint_id, source_part_id }`.

Derive it from the same inputs Core uses to place companions. Do not write a second
rule. Factor the classification out of `matrix::layout::assembly_placements`
(the `layoutOverride` test) and `constraints::pose` (explicit target precedence),
and have both the solver and the scene call it. Only enabled, non-deleted cells
whose companion part exists are listed.

This is blocked by 02 only because both regenerate the boundary transcript fixture
and edit the scene types in `model.rs`.

## Impact

Rerun `impact` on `assembly_placements` and `constraints::pose` before factoring
them. Both sit on the key-move path from `f65ed01a6`. Every test in
`core/tests/key_move_assemblies.rs` must pass unchanged.

## Tests (tdd; native)

1. A cell with a following diode, an authored LED and a constrained companion: the
   scene lists the three modes. The constrained entry names its constraint and
   source.
2. Consistency with placement: move the key with `MoveParts`. The poses each mode
   predicts match the solver's poses.
3. A disabled or deleted cell lists no companions. Removing the constraint changes
   that entry to `Follows` or `Authored`.
4. Linked halves: the target half's companions are listed with its own part IDs.

## Acceptance

Tests pass, `key_move_assemblies.rs` passes unchanged, and `lint`, `test` and
`typecheck` pass. The transcript fixture is regenerated.

## Files

`core/src/matrix/layout.rs` (classification), `core/src/constraints.rs`,
`core/src/matrix.rs` (scene), `core/src/model.rs` (scene types), the transcript
fixture.
