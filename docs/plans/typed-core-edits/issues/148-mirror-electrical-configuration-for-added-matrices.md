# 148: Extend the mirror electrical configuration for added matrices

Status: ready-for-agent
Type: build
Blocked by: [145](145-mirror-new-matrices-on-a-mirrored-board.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

145 mirrors a new matrix (members, correspondences, layouts, nets, modules, keycap settings) but does
not extend the mirror board's electrical configuration: pin locks and controller pin assignments for
the new matrix are missing on the other half. A thumb cluster added on a mirrored board must arrive
fully wired on both halves.

## What this ticket owes

- In the 145 transition (`core/src/mirrored_half/matrix_addition.rs`), extend the mirror board's
  electrical configuration for the added matrix: mirrored pin assignments and locks, honouring the
  `ElectricalBoard` correspondence overlay (Local keeps its own, Omitted gets none).
- Refuse, naming the identity, when the mirror controller has no pin for an assignment.

## Acceptance

- [ ] Red-first: add a base matrix then a thumb cluster; the mirror board's assignments and locks
      equal what `CreateMirroredHalf` derives for the same source.
- [ ] Local/Omitted overlay cases; no change to unrelated assignments; mutation check.

## File ownership

`matrix_addition.rs` and tests; request ownership before touching electrical modules.

