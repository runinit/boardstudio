# 34: Per-key Omit and Rejoin in the matrix surface

Status: claimed
Type: build
Blocked by: None (all dependencies resolved; the Local, Omitted and Rejoin Core lifecycle already exists)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Let the operator omit a single key from the matrix surface and rejoin it
later from the Inspector, so an omitted key leaves the live pose while
leaving the scan grid, and rejoining adopts the key back into its matrix
with the same safety as the established detach-confirm path.

## First RED / GREEN slice

Fail a mounted test: omitting a single key from the Inspector hides it from
the matrix surface while keeping its logical pose, and a single Undo
restores it with revision unchanged.

## Subsequent acceptance

- [ ] Omitting one key is a single atomic edit with one Undo entry; sibling
      keys keep their poses and no partial omission persists.
- [ ] Rejoining the omitted key adopts it back into its matrix with one Undo
      entry; anchor dependants stay valid or refuse atomically with a reason.
- [ ] Omit and Rejoin match the detach-confirm path: confirmation where the
      path requires it, an accessible blocked reason where refused, and no
      silent geometry loss.
- [ ] Relations explains the omitted state and the rejoin affordance without
      duplicating layout maths.

## Checks

Mounted browser tests for omit, rejoin, atomic refusal, and Undo round-trip;
affected quick/browser checks.

## Progress

- Investigated 2026-10-25: no per-key Omit/Rejoin UI exists in the new
  Inspector (`matrix_inspector*.rs`, `matrix_transform_*`). The transform
  Inspector has a per-key `KeyEnabled` toggle via `SetMatrix` (not an
  Omitted/Rejoin lifecycle), and the matrix Inspector has no omit affordance
  at all. Core offers two candidate contracts with different semantics:
  `SetMatrixCellsEnabled`/`SetMatrix` cell-enabled versus
  `SetHalfException{kind: Part, state: Omitted/Linked}` (+ Rejoin adoption
  in `mirrored_half/lifecycle`), and the ticket's detach-confirm parity
  (`DetachMatrixAnchorConfirmed`) is matrix-level, not key-level.
- The ticket's Blocked-by claims the Local/Omitted/Rejoin lifecycle already
  exists, but it does not name which Core variant is the per-key matrix-surface
  contract, nor which Inspector context (Key vs Matrix) owns the affordance,
  nor how omit interacts with the scan grid vs logical pose vs anchor
  dependants. Choosing the wrong variant would fork the exception lifecycle.
- Per task rules (no model.rs type changes unless required — if so, stop and
  report; and no new Core contract without an explicit ticket decision),
  stopping here for parent decision on the intended Core variant and owning
  surface. No code changed; ticket stays claimed for parent review.

## Implementation record (2026-10-25)

- Parent decision (from task brief): contract is
  `SetHalfException{pairId, kind: Part, mirrorId, state: Omitted}` to omit and
  `state: Linked` to rejoin, owned by the Matrix-inspector Key context
  (TreeContext::Key in the matrix transform inspector). No variant change was
  needed: Core keeps the omitted key's live pose while it leaves the scan grid
  (verified natively: scene cell member gone, doc part shed, pose retained in
  the scene/source; Relations already documents "live logical pose only").
- Scan-grid vs pose check: no different variant required. A Part-level Omitted
  overlay is exactly "leave the live pose, leave the scan grid"; cell-enabled
  toggles (KeyEnabled/SetMatrix) were NOT used (different semantics: scan-grid
  membership, not mirror exception).
- Web (`matrix_transform_operation.rs`, native): `KeyHalfException{pair_id,
  mirror_id, omitted}` + `key_half_exception()` projection helper +
  `key_half_exception_resolver(pair_id, mirror_id, omit)` (atomic, single Undo;
  Unchanged on same state, Retire on missing pair/correspondence) +
  `key_half_exception_at_resolver(matrix_id, row, column, omit)` (re-resolves
  member+pair at execution; Omitted names the canonical derived member when
  the scene cell is gone) + `MatrixTransformField::KeyOmit` + `build_operation`
  arm (validation gate).
- Web (controller/inspector, wasm): `Fields::Key.half_exception` computed from
  the scene member with Omitted fallback; KeyOmit dispatch; "Omitted from
  mirror" label; `Value::Bool` mapping; `OmitTransformField` checkbox with
  one-Undo consequence text + inline errors; shown only for pair-mirrored keys.
- Relations (`matrix_relations.rs`): Omitted-key item now names the rejoin
  affordance ("Clear the omission in the Key inspector to rejoin").
- RED: native `key_omit_tests` (failed to compile pre-resolver, then asserted
  the Submit/unchanged/retire contract); mounted
  `mirror_key_omit_rejoin_round_trip` + `mirror_key_omit_checkbox_toggles_half_exception`
  (failed pre-implementation: no Omit UI, resolver retired).
- New test names: `omit_submits_part_omitted_and_rejoin_submits_linked`,
  `rejoin_clears_an_omitted_overlay`,
  `same_state_is_unchanged_and_unknown_members_retire`,
  `rejoin_resolves_an_omitted_key_without_a_scene_cell`,
  `projection_helper_reports_pair_and_omitted_state`,
  `mirror_key_omit_rejoin_round_trip`,
  `mirror_key_omit_checkbox_toggles_half_exception`.
- Open-validation notes (for future pair fixtures): mirrored matrix members
  must use derived ids (`matrix/{id}/r{row}c{column}`); the pair needs
  `constructionSource`; matrix-level Part correspondences are required.
- Environment note: `~/.config/chromium` reports "Transport endpoint is not
  connected" (stale user FUSE mount, predates this session; left untouched).
  Browser tests were run with `XDG_CONFIG_HOME` redirected to a scratch dir
  to avoid the dead mount; renderer timeouts under shared-box CPU saturation
  (gortex ~270%) required retries, and one stray chromium was killed.

- Verify (so far): native `cargo test -p boardstudio-web-layout --lib`
  (6G, RUST_TEST_THREADS=4): green incl. 5 new native omit tests + updated
  relations text test. `cargo check --workspace --all-targets` clean; layout
  clippy `-D warnings` clean (one pre-existing `iter_next_slice` in untouched
  `mirror_existing_half.rs`); rustfmt clean on touched files.
- Mounted: `mirror_key_omit_rejoin_round_trip` +
  `mirror_key_omit_checkbox_toggles_half_exception` pass (direct runner):
  omit records Omitted, hides the key from the surface (scene member gone),
  keeps the logical pose, one Undo restores, rejoin clears. Neighboring
  mounted Key-section tests pass in isolation: mixed_enabled, deleting_selected,
  row/col delete, detach. Full layout browser suite in this environment:
  `setup_guide desktop viewport` fails (environmental: headless viewport
  detection; no viewport overlap with this ticket) and poisons later tests
  via the panic-hook cascade; the omit tests pass when run directly.
- Commits: `3b71398fd` feat(ergogen-34). Status: claimed (implementation complete; awaiting parent review/close).
