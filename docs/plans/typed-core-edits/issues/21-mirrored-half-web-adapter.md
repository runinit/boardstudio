# 21: Mirrored-half web adapter

Status: resolved
Type: build
Blocked by: [19](19-reconcile-mirrored-halves.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Let a user create and manage a mirrored half in the app through the typed operations.

## What to do

- "Create mirrored half" in physical setup and Layout, submitting `CreateMirroredHalf`
  through the ResolveEdit path (ADR-0005), never `ReplaceDocument`.
- Inspector shows linked, local or omitted per item; Make local, Omit and Rejoin as
  one-shot actions. No history entry from hover or focus.
- Mounted tests for create, an edit from either half, a local exception and Undo.

- Move physical setup (`propose_with_mirror_pair` and `into_submission`) onto `CreateMirroredHalf`
  so there is one path and one atomic Undo step. Extend the operation to accept the physical setup's
  topology and transport values and the resolved `sharedConstruction`; Core applies and validates
  them together with half creation. The adapter chooses topology and transport and resolves
  `sharedConstruction` with existing physical-setup precedence: keep the existing hardware value,
  otherwise use the selected primary's mechanical settings (including the existing board-settings
  and initial-settings fallbacks). Pass that value explicitly so Core preserves it exactly. The
  request's `labels` explicitly carries both the source and mirror label inputs (name, half, role;
  ticket [32](32-mirrored-half-labels.md)). Do not stage topology, transport, shared construction or
  relabelling in a separate edit or submit `ReplaceDocument`.
- Preserve physical setup behavior for topology, transport, `sharedConstruction`, instance IDs,
  labels, and mirror-pair creation. Validate the topology/transport combination and that
  `sharedConstruction` belongs to the configured physical board; reject inconsistent requests
  atomically without changing the document. Return all affected IDs through Core's changed-id report.
- Update ticket [26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md)'s parity
  comparison for the new operation inputs and ownership. Its adapter-owned topology/transport
  differences are historical findings, not the target contract.

## Acceptance

- [x] Eligible physical-setup creation submits `CreateMirroredHalf` with typed target ids; the create path no longer submits `ReplaceDocument`.
- [x] Core tests cover topology and transport application, valid and invalid combinations, labels,
  exact `sharedConstruction` preservation, unchanged state on refusal, and one atomic undoable
  creation (`physical_setup_and_pair_creation_are_one_atomic_core_edit`).
- [x] Mounted browser tests pass; DESIGN.md updated only if a visible control changes.

## Verification

`typecheck`, `browser`, `lint`; confirm callers with `rg` and source inspection.

## Labels (from ticket [32](32-mirrored-half-labels.md))

The adapter passes the source and mirror labels (name, half, role) in the creation request instead of a separate relabelling edit.

## Progress (2026-10-14)

The atomic Core operation and physical-setup adapter submission were committed to `dev` as
`fd2f217e` (`feat(core): make mirrored setup and placement edits atomic`). The code now submits
`CreateMirroredHalf` with physical setup values and labels in one ResolveEdit operation; mounted
physical-setup create/edit/local/undo coverage is still missing.

Core's 114 mirrored-half integration tests and the catalogue's 24 physical-setup tests pass; `lint`,
`repo` and WASM `typecheck` pass. With refreshed filesystem access, the pinned OCCT build completed
in a disk-backed validation copy, its 15 CAD smoke checks passed, and the browser gate passed 89
mounted tests. The focused host storage suite also passed all 11 tests. The mounted tests do not
cover physical-setup mirrored-half create/edit/local/undo as required above, so that acceptance
item remains open.

## Progress (2026-10-14, Layout exception Inspector)

Commit `1ebcd35c` adds linked/local/omitted projection for a mirrored Part and Make local, Omit
and Rejoin actions submitted through `ResolveEdit` as `SetHalfException`. The selected source or
mirror part resolves to the same mirror identity. No `ReplaceDocument` is used for exception edits.

`python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs`
passed all 33 tests, including a mounted disclosure/action test and typed resolver assertions.
`python3 scripts/check.py typecheck`, `cargo fmt --all --check` and `git diff --check` passed.
The visible control follows the native disclosure pattern already specified in `DESIGN.md`; no
design change was needed. Full physical-setup create/edit/local/undo and accepted Core action
coverage remain outstanding, so acceptance stays open.

## Progress (2026-10-14, physical-setup browser coverage)

The repository already contains `a_reversible_split_establishes_its_mirror_pair_and_undoes_in_one_step`
in `web/crates/pcb/src/pcb_physical_setup/tests.rs`. It exercises the mounted physical-setup
controller against Runtime/Core and proves split creation establishes the pair in one Undo step.
I ran the crate-level WASM suite directly (`cargo test -p boardstudio-web-pcb --target
wasm32-unknown-unknown pcb_physical_setup::tests::`): 8 passed. This suite is not selected by
`scripts/run-wasm-tests.py`, which runs the web binary's presentation modules.

This closes the physical-setup create/Undo evidence gap. End-to-end Layout exception editing from
both halves, exception Undo, and accepted Core action coverage remain open; acceptance stays open.

## Progress (2026-10-14, mounted Layout exception round trip)

The mounted Layout test `mounted_mirrored_half_actions_work_from_either_board_and_undo` now drives
Omit and Undo from the source board, then Make local and Undo from the mirror board. Each assertion
waits for a newer accepted revision as well as the expected projected state; after navigation it
waits until the selection adapter owns the mirror-board component before dispatching the next edit.

`python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs`:
37 passed. The physical-setup WASM suite also passed 8/8, including atomic split creation and Undo.
The mounted browser acceptance item is now checked. This does not complete ticket 22's reused-PCB,
save/reopen, full-check, boundary-transcript, and review requirements.

## Progress (2026-10-14, Rejoin)

Added `mounted_rejoin_clears_a_local_exception_and_undo_restores_it`: the mounted Inspector sends
Rejoin through the edit path, Core removes the Local exception, and one Undo restores it. The full
Inspector module passed (`python3 scripts/run-wasm-tests.py --files
web/src/presentation/layout_component_inspector_tests.rs`, 40 tests); `cargo fmt --all --check` and
`git diff --check` passed. Full Editor selection-owner integration remains open under ticket 41.


## Answer

The typed mirrored-half adapter is complete. Physical setup submits atomic `CreateMirroredHalf` values and labels; mounted Runtime/Core coverage proves split creation and one-step Undo, while mounted Layout coverage proves Omit, Make local, Rejoin, and Undo from either half. All three acceptance items above are checked. The current full `python3 scripts/check.py` gate passed on 2026-10-14 in 202.4 s; the separate reused-PCB end-to-end workflow and review requirements belong to dependent ticket 22, and full Editor companion-placement selection ownership remains in ticket 41.
