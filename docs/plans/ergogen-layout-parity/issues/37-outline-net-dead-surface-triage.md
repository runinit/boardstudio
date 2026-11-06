# 37: Outline and Net dead-surface triage

Status: resolved
Type: build
Blocked by: None (all dependencies resolved; the live outline and net surfaces are established, so each dead variant can be judged against them)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Triage every Core edit variant with no live surface caller, and for each one
either wire it to a real Inspector or canvas surface or delete it, so no
dead outline or net variant remains in the Core contract.

## First RED / GREEN slice

Fail a mounted test: the triage table in this ticket names every dead
variant in the SelectOutline family, SetNet, and the superseded mechanical
and outline operations, with a wire-or-delete verdict for each, and follow-up
work proves the table is empty by the ticket's close.

## Subsequent acceptance

- [ ] Each SelectOutline-family variant is either reachable from a real
      surface with acceptance proof or deleted with dependents updated.
- [ ] SetNet is either reachable from a real surface with acceptance proof or
      deleted with dependents updated.
- [ ] Each superseded mechanical and outline operation is either reachable
      from a real surface with acceptance proof or deleted with dependents
      updated.
- [ ] No dead Core variant remains: enumeration of the Core edit contract
      shows every surviving variant has a live caller or an explicit test.

## Checks

Triage table plus mounted browser tests for each wired surface (or deletion
proof with dependents updated); affected quick/browser checks.

## Triage (2026-10-25, worktree `t3-18fae8db`, base `d772c70a2`)

Wiring-audit pointer was unavailable in-session (no audit doc found under
`docs/`; `matrix-layout-dependents-inventory` and the ergogen comparison do
not name these variants), so the table below is enumerated directly from the
Core contract (`core/src/model.rs::EditOperation`) against live callers in
`web/crates/` + `web/src/` (`grep -rln`). `core/src` rows are Core-internal
mirroring/lifecycle/test dependents, not live surfaces.

| Variant | Live surface caller? | Core-internal dependents | Verdict |
| --- | --- | --- | --- |
| `SelectOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — live outline surface uses `ApplyOutlineEdit` (`layout/src/outline/adapter.rs`, `hook.rs`, `inspector.rs`) |
| `RenameOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — version rename lives in `OutlineSettingEdit::RenameVersion` via `ApplyOutlineEdit` |
| `RemoveOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — no live remove-version surface found |
| `CopyOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — no live copy surface found |
| `SetNet` | None in `web/` (`pcb_wiring` does not submit `SetNet`) | `core/src/lib.rs`, `core/src/mirrored_half/inverse/hosted.rs` | Delete (with dependents) — no live net Inspector/canvas path found |
| `SetMechanical` | None in `web/crates/`; stale `web/src` refs only (`layout_component_inspector.rs`, `presentation.rs`) | `core/src/lib.rs`, `mirrored_half/reconcile/prepare.rs`, `mirrored_half/lifecycle.rs` | Delete (with dependents) — superseded by `ApplyMechanicalEdit` (`case/src/mechanical_settings_controller.rs`) |
| `SetOutline` | None in `web/crates/`; stale `web/src` refs only (same two files) | `core/src/lib.rs` | Delete (with dependents) — superseded by `ApplyOutlineEdit` (`layout/src/outline/`) |

Live counterparts confirmed: `ApplyOutlineEdit` (adapter/hook/inspector +
`outline_lifecycle_tests.rs`) and `ApplyMechanicalEdit`
(`mechanical_settings_controller.rs`, `mechanical_pair.rs`, session tests).
`SelectOutline`-family + `SetNet` + superseded `SetMechanical`/`SetOutline`
are the only dead families in this ticket's scope; the full 50-variant
contract was not re-audited here.

## Progress

- Implemented 2026-10-25 (worktree `t3-18fae8db`, base `76c2bb1c5`, parent approval
  recorded in the task brief): deleted the 7 dead variants
  (`SelectOutline`, `RenameOutline`, `RemoveOutline`, `CopyOutline`, `SetNet`,
  `SetMechanical`, `SetOutline`) plus dependents.

- Triage table above is the RED artifact: it names every dead variant in the
  ticket's scope with a delete verdict and the live counterpart for each.
  No wire-to-surface candidate was found — every family is superseded.
- Deletion (model.rs variant removal + `lib.rs` arms + `mirrored_half`
  inverse/reconcile/lifecycle + `outline_versions.rs` dependents + baseline
  test-name updates) required model.rs type changes and proceeded under parent
  approval. Deletion landed in `c0f6b7e06`
  (see Implementation record below).

### Implementation record (2026-10-25)

- Parent approval line (from task brief): APPROVED deletion of the 7 dead variants
  (SelectOutline, RenameOutline, RemoveOutline, CopyOutline, SetNet,
  SetMechanical-superseded, SetOutline-superseded) + dependents in
  lib.rs/mirrored_half/outline_versions.rs. Live counterparts are
  ApplyOutlineEdit/ApplyMechanicalEdit (verified each deletion site has one before
  removing). Pre-delete `grep -rn EditOperation::<Variant> web/` showed zero live
  callers for all 7 (the two `web/src` `SetOutline` hits are a local UI action enum,
  not the Core op; `SetMirroredHalfState` there already submits live SetHalfException).
- RED: `contracts::removed_outline_net_and_raw_mechanical_kinds_refuse_deserialization`
  (new) failed pre-deletion (`select-outline` deserialized), passes post-deletion.
- Removed: 7 variants from `core/src/model.rs`; `outline_versions::apply` (helpers kept:
  `outline_edit.rs` still uses `snapshot`/`name`/`version_identity_problem`, and
  `refresh_recovery` + `geometry.rs` use the rest); `lib.rs` apply arms +
  `affects_outline`/`uses_prepared_transaction`/prepared-contour guards;
  `inverse.rs` dead arms + `SET_MECHANICAL_*` consts; `inverse/outline.rs` 5 dead
  translate arms + now-unused `feature_context` (kept `Context::feature`, still used
  by ApplyOutlineEdit); `inverse/hosted.rs` `set_net` + `net_target` + `Op::SetNet`
  arm + hosted-exhaustiveness arms; `prepare.rs` 3 SetMechanical sites (pre/post
  `prepare_lifecycle_pairs` unified to the post-resolution call);
  `lifecycle.rs` instance-SetMechanical unlink arm (superseded by
  `UnlinkMirrorPair`/`UnlinkHalves`); orphaned `mechanical/edit/replacement.rs`
  whole-config instance-replacement module + `edit.rs` wrapper (live instance
  writes go through `edit.rs` patch path, proven by repurposed tests).
- Kept intentionally: `OutlineContourEdit` (used by wasm test imports),
  `MechanicalTarget`/`MechanicalConfiguration` (used by MechanicalEdit),
  `MirroredItemKind::Net/Outline/OutlineVersion` (used by live mirror records).
- Repurposed (same names, live counterparts): lib.rs 3 mechanical tests (Disable/
  Enable on Board/Instance targets); contracts `boxed_public_payloads...`
  (ApplyMechanicalEdit JSON shape); case `case_preparation...` (SetCase history
  setup); core `undo_redo_increments_revision` (SetCase); inverse
  `set_outline_on_a_linked_mirror...` (set-feature); settings
  `reconciling_twice...` (legend-only); mirrored_half.rs production instance test
  (Enable+mounting-hole then SetDimension); edit_family outline rename/feature
  tests (set-feature/rename); noop test (set-feature); gasket helper (JSON setup);
  mechanical_workflow establish test (Enable); 6 outline_versions + 6 ownership +
  persistence/linked-resolution suites (Copy/Activate/Rename/Delete/SetFeature/
  EditGeneratedPerimeter/AddFeature, incl. JSON-setup for authored generated state).
- Deleted dead-transport tests (no live op drives them; list for baseline update):
  prepare/tests.rs 11 `targeted_set_mechanical_*`; replacement.rs 20 tests;
  hosted_tests.rs 6 (`a_mirror_net_edit_is_a_pin_delta_on_the_source_net`,
  `an_unchanged_mirror_net_translates_to_the_unchanged_source_net`,
  `a_renamed_mirror_net_renames_the_source_net_and_pad_ids_are_kept`,
  `a_net_edit_that_changes_the_mirrors_own_pins_is_refused`,
  `source_local_omitted_and_new_nets_follow_the_established_rules`,
  `a_pin_on_a_member_of_a_local_circuit_is_the_mirrors_own_pin`) + `net_pins`;
  settings_tests.rs 5 (`a_source_net_pin_edit...`, `a_mirror_side_net_edit...`,
  `a_local_part_keeps_its_own_pins...`, `a_source_pin_removed...`,
  `a_local_net_keeps_its_own_pins...`); inverse/tests.rs
  `targeted_raw_set_mechanical_is_delegated_without_inverse_mapping`,
  `outline_local_exception_runs_as_written_and_omission_is_refused`,
  `legacy_outline_version_commands_map_linked_versions_and_keep_their_other_inputs`;
  edit_family_acceptance.rs `parent19_plain_net_membership_commits_from_either_half`.
- New test names: `removed_outline_net_and_raw_mechanical_kinds_refuse_deserialization`.
  Renamed: none (repurposed bodies kept names; `first_point_edit...` still accurate).
- Verify: `cargo test -p boardstudio_core` 772 lib + 864 integration, 0 failed;
  `cargo check --workspace --all-targets` clean; `cargo clippy -p boardstudio_core
  --all-targets -- -D warnings` clean; rustfmt applied to touched files only
  (pre-existing `web/crates` drift untouched).
- Notes for parent: typed-core-edits tickets 147/149 referenced SetNet/SetMechanical;
  both are resolved and superseded per this deletion. Baseline JSONs were NOT edited;
  the deleted test names above need baseline update.

- Commits: `c0f6b7e06` test(ergogen-37) deletion + coverage; `309304a9d`
  docs(ergogen-37) reservation-link repoint. Status: resolved.

## Answer

Resolved on `docs/ergogen-layout-comparison` via merge `21d16e2db`
(`feat/ergogen-tickets-33-37`). Implementation `c0f6b7e06`, repoint `309304a9d`.

- Parent-approved model.rs deletion for this ticket only: removed the 7 dead
  variants (`SelectOutline`, `RenameOutline`, `RemoveOutline`, `CopyOutline`,
  `SetNet`, superseded `SetMechanical`/`SetOutline`) + dependents (`lib.rs`
  arms, `mirrored_half` inverse/reconcile/lifecycle, `outline_versions.rs`).
  Live counterparts verified per deletion site (`ApplyOutlineEdit` /
  `ApplyMechanicalEdit`); `grep` showed zero live `web/` callers for all 7.
- 1 new test (`contracts::removed_outline_net_and_raw_mechanical_kinds_refuse_deserialization`);
  dead-transport tests deleted (list in Triage/Implementation record);
  survivors repurposed onto live counterparts under kept names.
- In-ticket verification: core lib 772 + integration 864, workspace check +
  core clippy clean, rustfmt clean on touched files.
- Baseline `73fd27ea` (+24/-46: -46 ticket-37 deletions incl. the 45 core +
  1 parent19 names above, +10 ticket additions, +14 absorbed stale main-side
  names each green in the 2280 gate and tracing to a reviewed commit);
  native gate 2280 passed / 0 failed
  (receipt `.evidence/ergogen-33-37-integration/native-baseline.json.md`).
- Closing verification: fresh `quick` + `full` gates green; logs under
  `.evidence/ergogen-33-37-integration/`, last lines in the closing commit.
- Verify: targeted `cargo test -p boardstudio_core` (6G, RUST_TEST_THREADS=4): lib
  772 passed/0 failed, integration 864 passed/0 failed. `cargo check --workspace
  --all-targets` clean. `cargo clippy -p boardstudio_core --all-targets
  -- -D warnings` clean. rustfmt clean on all touched files. `check.py quick`
  repo step: my 2 broken doc links fixed; remaining failures are pre-existing
  (deferred-items links in issues 28-32, web/crates fmt drift, and a
  `clippy::iter_next_slice` in untouched `mirror_existing_half.rs`).
