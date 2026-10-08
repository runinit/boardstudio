# 03: Core publishes companion placement modes

Status: resolved
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

## Outcome

Implemented on `typed-core-edits/matrix-core`:

- `89051b376` claim; `d48c1d976` implementation; `70c12ea38` review response.
- `MatrixScene.companions` carries `MatrixCompanionPlacement { part_id, key_id,
  mode }`. `MatrixCompanionMode` is `Follows | Authored | Constrained { constraint_id,
  source_part_id }`, serde `follows`/`authored`/`{"constrained": {"constraintId",
  "sourcePartId"}}`, skipped when empty. Only enabled, non-deleted cells whose
  companion part exists are listed.
- `matrix::layout::companion_mode` is the one classification source: an explicit
  constraint wins, then an authored pose, otherwise the companion follows its key.
  `assembly_placements` stores the mode, `constraints::pose` branches on it instead
  of re-checking precedence, and `matrix::scene_companions` calls the same function.
  Extracted `constraint_pose`/`follow_pose` and the shared `matrix::companion_id`.
- The boundary fixture now builds a diode/led/sensor cell, adds a constraint and
  authors the LED, and asserts all three modes (including the structured constrained
  shape) across native and WASM; the golden transcript was regenerated.

Impact (GitNexus `impact`, before factoring): `assembly_placements` HIGH (7 impacted,
1 direct) and `constraints.rs:pose` HIGH (7 impacted, 1 direct). Mitigation: the
mode-driven refactor is behaviour-preserving and every `key_move_assemblies.rs` test
passes unchanged. `detect-changes --scope compare --base-ref 89051b376` reported 11
files, 39 symbols, 0 affected processes, low risk, with no truncation.

Checks run: `matrix_companions.rs` 4 tests; `key_move_assemblies.rs` 23 unchanged;
`cargo test -p boardstudio_core` 174 unit + 349 integration; `cargo test -p
boardstudio-application` 30; `cargo clippy --workspace --locked --all-targets
--no-deps -- -D warnings`; `cargo fmt --all --check`; WASM `cargo check -p
boardstudio-web --target wasm32-unknown-unknown --features page`; native boundary
parity. Standards and Spec re-reviews are clean after the review response (one
test-only helper-duplication observation remains).

Follow-ups: the WASM browser half of the boundary test was not run here (`typecheck`
covers compilation). Ticket 08 can consume `companions` and should rely on Core's
`mode` rather than re-deriving precedence. `MatrixScene.companions` is populated for
actual projections only; draft matrix projections list none.

## Files

`core/src/matrix/layout.rs` (classification), `core/src/constraints.rs`,
`core/src/matrix.rs` (scene), `core/src/model.rs` (scene types), the transcript
fixture.
