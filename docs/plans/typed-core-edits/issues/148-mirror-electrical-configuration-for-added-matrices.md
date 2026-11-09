# 148: Extend the mirror electrical configuration for added matrices

Status: resolved
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

- [x] Red-first: add a base matrix then a thumb cluster; the mirror board's assignments and locks
      equal what `CreateMirroredHalf` derives for the same source.
- [x] Local/Omitted overlay cases; no change to unrelated assignments; missing-pin refusal and
      mutation checks.

## File ownership

`matrix_addition.rs` and tests; request ownership before touching electrical modules.

## Comments

2026-10-12 implementation candidate on `typed-core-edits/ticket-148`: new matrix member function
keys in assignments and locks are mapped through the planned member correspondences. Unrelated mirror
configuration remains unchanged. A Local electrical-board overlay preserves the complete mirror
configuration; an Omitted overlay leaves the mirror electrical configuration absent. An assignment
or lock is refused with its function identity if the mirror controller has no matching physical pin.

Evidence: red-first assignment test failed because the mapped mirror function was absent. Final
focused run: `cargo test -p boardstudio_core --lib mirrored_half::matrix_addition::tests:: -- --nocapture`
(26 passed). Mutation checks for function mapping, overlay handling and missing-pin refusal: 3 killed,
0 survived. `python3 scripts/check.py test --update-native-baseline` passed with 1695 executed native
tests and updated the baseline through the project command. Independent Standards and Spec reviews
found no violations or gaps (the Standards review noted one judgment-call duplicate loop). Merged
to `dev` as `b9db54166`. Post-merge `python3 scripts/check.py repo test lint` passed.
