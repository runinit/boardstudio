# 21: Mirrored-half web adapter

Status: ready-for-agent
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
  so there is one path; topology and transport stay in the adapter. Pass physical setup's source and
  mirror relabelling in the optional `labels` on the creation request (ticket
  [32](32-mirrored-half-labels.md)), so creation and relabelling are one Undo step.
  Ticket [26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md) records the original differences;
  ticket 32 removes only rows 1 to 6 from the equivalence test when labels are supplied. Keep the
  remaining differences pinned until the adapter owns the complete topology/transport path.

## Acceptance

- [ ] No `ReplaceDocument` in the create path; typed ids from Core's changed-id report.
- [ ] Mounted browser tests pass; DESIGN.md updated only if a visible control changes.

## Verification

`typecheck`, `browser`, `lint`; GitNexus `impact` on the physical-setup entry points.

## Labels (from ticket [32](32-mirrored-half-labels.md))

The adapter passes the source and mirror labels (name, half, role) in the creation request instead of a separate relabelling edit.
