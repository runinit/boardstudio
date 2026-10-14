# 21: Mirrored-half web adapter

Status: claimed
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
- [ ] Mounted browser tests pass; DESIGN.md updated only if a visible control changes.

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
