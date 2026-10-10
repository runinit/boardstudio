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
  so there is one path; topology, transport and relabelling of the primary stay in the adapter.
  Use the differences recorded by ticket [26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md).
  Relabel the primary as left, central, "Left half" before calling the operation: Core's mirror label keeps any half
  other than left or right (a unibody source gets a "unibody", central "Keyboard mirror" mirror), see the F1 judgment
  in ticket 26's Verification. Afterwards update the pinned difference set in
  `physical_setup_and_reuse_board_agree_on_a_reversible_split_pair` (rows 1 to 6 should vanish).

## Acceptance

- [ ] No `ReplaceDocument` in the create path; typed ids from Core's changed-id report.
- [ ] Mounted browser tests pass; DESIGN.md updated only if a visible control changes.

## Verification

`typecheck`, `browser`, `lint`; GitNexus `impact` on the physical-setup entry points.
