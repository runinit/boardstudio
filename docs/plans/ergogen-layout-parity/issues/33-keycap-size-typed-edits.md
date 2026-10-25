# 33: Keycap size through typed edits

Status: claimed
Type: build
Blocked by: None (all dependencies resolved; the Inspector keycap surface and the typed-edit Core contract already exist)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Make the Inspector keycap size control submit the operator's typed value as a
single named typed keycap edit, so a keycap resize commits exactly what was
typed, validates before writing, and undoes and redoes as one entry.

## First RED / GREEN slice

Fail a mounted test: typing a valid keycap size in the Inspector and
committing resizes the selected key's keycap, and a single Undo restores the
prior keycap size with nothing else changed.

## Subsequent acceptance

- [ ] Committing a valid typed keycap size writes one typed edit with one
      Undo entry; the keycap and plate opening follow the tied size rule.
- [ ] Committing an invalid keycap size is refused with an inline message and
      writes nothing; history and the saved document are untouched.
- [ ] One Undo restores the prior keycap size exactly; Redo restores the
      committed size exactly.
- [ ] The control never submits a whole-document replacement for a keycap
      resize; history shows the single named typed entry.

## Checks

Mounted browser tests for the typed keycap commit, refusal, and Undo/Redo
round-trip; affected quick/browser checks.

## Progress

- Investigated 2026-10-25: the Layout key-size surface (`keycap_size_controller.rs`
  `resize_resolver` + `keycap_resize.rs::plan_resize`) submits
  `EditOperation::ReplaceDocument` for every keycap resize. No Core typed
  variant writes `Part.keycap` size: `SetKeycapBoard/Matrix/Key` cover keycap
  *settings* (`KeycapKeyChange::Units` etc. in `core/src/keycaps/edits.rs`),
  not the physical `Part.keycap: Option<Vec2>` size tied to pitch minus edge
  gap. `grep` for `part.keycap =` shows only `ReplaceDocument`, matrix
  creation, and mirrored-half lifecycle writers; no typed size edit exists.
- Ticket requires a new named typed keycap-size Core edit (model.rs type
  change) to replace the whole-document path. Per task rules (no model.rs
  type changes unless a ticket requires it — if so, stop and report), stopping
  implementation here and reporting to parent. No code changed; ticket stays
  claimed for parent review.

## Implementation record (2026-10-25)

- Parent approval line (from task brief): APPROVED model.rs change for this
  ticket only — added typed `SetKeycapSize` writing `Part.keycap` + wired
  `keycap_size_controller` `plan_resize` through it instead of `ReplaceDocument`.
  Earlier no-model.rs rule lifted FOR THIS TICKET by parent decision.
- Core (`core/src/model.rs`, `lib.rs`, `mirrored_half/inverse.rs`,
  `mirrored_half/inverse/hosted.rs`): new `SetKeycapSize{sizes,moves,offsets}` +
  `KeycapCellOffset`; apply arm validates everything (known part/matrix, finite
  positive sizes, finite poses/offsets) before writing, one Undo entry; Preview
  routes through `preview_edit` like MoveParts; `affects_outline` includes it
  (plate openings follow size); mirror pass-through as Delegated (planner
  pre-computes both halves; hosted bucket extended for exhaustiveness).
- Web (`keycap_resize.rs`): plan exposes `sizes`/`moves`/`offsets` derived from
  the same computation (absolute poses, changed cell offsets); controller
  submits `SetKeycapSize` with `plan.target_ids`. `plan.document` kept for
  existing planner assertions. Existing `rapid_*`/queued/selector mounted tests
  pass through the typed path unchanged.
- RED: `keycaps::typed_keycap_size_writes_sizes_moves_and_offsets_in_one_undo_step`
  + `typed_keycap_size_refuses_invalid_writes_atomically` (failed compile
  pre-variant); `resize_plan_exposes_typed_keycap_size_sizes_moves_and_offsets`
  (native planner); `resize_resolver_submits_a_typed_keycap_size_edit` (mounted:
  would fail pre-wire with ReplaceDocument).
- Repurposed (same names): `accepted_rotated_and_mirrored_resize...` and linked
  mirror test now commit the typed edit (assemblies/poses/history proof kept).
- New test names: `typed_keycap_size_writes_sizes_moves_and_offsets_in_one_undo_step`,
  `typed_keycap_size_refuses_invalid_writes_atomically`,
  `resize_plan_exposes_typed_keycap_size_sizes_moves_and_offsets`,
  `resize_resolver_submits_a_typed_keycap_size_edit`.
- Verify: `cargo test -p boardstudio_core` (6G, RUST_TEST_THREADS=4): lib 772 +
  integration 866 (2 new), 0 failed. `cargo test -p boardstudio-web-layout --lib`:
  107 passed. Mounted `wasm-pack test ... layout -- keycap`: 13 passed including
  new resolver test + selector undo/redo + queued retire. `cargo check --workspace
  --all-targets` clean; core clippy `-D warnings` clean; rustfmt on touched files.
- Commits: `d489f5798` feat(ergogen-33). Status: done (claimed through close).
