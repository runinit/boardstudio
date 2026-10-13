# 150: Prepare selective exceptions for mirrored pairs without Matrix correspondences

Status: resolved
Type: build
Blocked by: [149](149-targeted-mirrored-set-mechanical.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare Local, Omit and Rejoin for linked independent items on a native mirrored pair that has
no Matrix correspondence. Parent 19's selective exceptions apply to those items as well.
Keep this work private; ticket 111 owns the operation vocabulary and production activation.

## Source-verified gap (2026-10-13)

`mirrored_half::lifecycle::prepare_edit_transition` returns an empty result when the pair has no
Matrix correspondence. A pair may still contain physical matrices without a Matrix correspondence.
Its `lifecycle_context` requires source and mirror matrix IDs. In contrast,
`reconcile::prepare::prepare_open` and `projection_for_matrixless_pair` already handle pairs with
no Matrix correspondence, with a regression for linked independent Parts. Open support does not
provide a selective exception path. Selecting an arbitrary or fabricated matrix would hide this
gap.

The matrix-backed lifecycle is already verified: private tests compose
`transition_exception_context` with `prepare_lifecycle_transition`. Preserve that implementation
and its ownership, durable association, copper/pin cleanup and child-exception rules.

## Implementation scope

- Resolve the exact pair/kind/mirror identity and requested state inside the existing lifecycle
  module. Keep the intent-to-draft helper private there; 111 can compose it from an operation
  branch in the existing `prepare_edit_transition` adapter without a new callable seam.
- Represent the absence of matrix ownership explicitly where context is required. Do not invent
  sentinel IDs, create dummy matrices, pick the first unrelated matrix, or duplicate reflection
  or exception rules. Keep matrix-specific membership work conditional on actual matrix ownership.
- Reuse the independent/hosted projection and lifecycle kernels. Local retains visible geometry,
  Omit removes only the selected owned projection, and Rejoin derives from the current source.
  Unrelated objects, other pairs and explicit child exceptions remain intact.
- Build the complete result on a disposable draft and validate pair records before returning.
  Ambiguous or stale identities refuse without redirecting to another object or publishing data.
- Own `core/src/mirrored_half/lifecycle.rs`, its private tests, and only necessary private context/
  projection changes in `mirrored_half/reconcile.rs` and `reconcile/prepare.rs`. Inspect overlapping
  work first. No visibility widening, new public operations, storage changes or production hooks.
  If another path or visibility change is necessary, assign the path or obtain the required owner
  approval before making it.
- Keep these shared private paths serial: 150 owns its scoped preparation through resolution; after
  that handoff, 111 owns the remaining production dispatch and activation wiring.

## Acceptance

- [x] Direct private tests first expose the missing matrixless exception behavior at runtime.
- [x] On a pair with no Matrix correspondence (physical matrices may still exist), an independent
      Part supports Local/Omit/Rejoin under a rotated map, including edits to the current source
      between transitions and exact identity retention.
- [x] EmbeddedCircuit owner exceptions preserve existing owned-reference and explicit-child rules
      without introducing matrix ownership. Local/Omitted Part exceptions for module host connectors
      preserve or clean Module references according to the selected state; Module is not treated as
      an owner of those Parts.
- [x] Local and Omitted states survive serde and direct reconciliation; Rejoin uses current source.
- [x] Ambiguous/stale selection and late validation refuse atomically; unrelated data is preserved.
- [x] Existing matrix-backed lifecycle regressions remain green, with no synthetic-matrix workaround.
- [x] Independent Standards and Spec review pass; mutation checks detect skipped state transitions,
      lost durable association and unintended cleanup. Production activation remains owned by 111.

## Related

- [109](109-mirrored-half-exceptions-unlink.md) owns the verified lifecycle policy and kernel.
- [111](111-integrate-mirrored-half-reconciliation.md) consumes this preparation before activation.
- [116](116-prepare-mirrored-open-repair.md) demonstrates matrixless Open support.

## Comments

2026-10-13: Private matrixless exception preparation is implemented and frozen for integration.
The checked context carries either the exact Matrix correspondence or `None`; matrix-only
membership work refuses missing context with a recoverable error, while generic lifecycle
preparation keeps matrixless independent and hosted ownership on the existing transition kernels.
The private intent helper covers independent Parts and applicable EmbeddedCircuit owners without
changing production visibility or operation vocabulary. Physical matrices remain untouched when
the pair has no Matrix correspondence. The tests cover Local/Omit/Rejoin, current-source edits,
serde and repeat reconciliation, module connector cleanup, stale/ambiguous selection, matrix-owned
Part refusal, and late atomic refusal.

The runtime red checkpoint failed because Local prepared zero drafts instead of one. Final checks:
`cargo check -p boardstudio_core --tests`; `cargo test -p boardstudio_core --lib matrixless_`
(4 passed); `cargo test -p boardstudio_core --lib mirrored_half::reconcile::` (209 passed);
`cargo fmt --all -- --check`; and `git diff --check`. Evidence and SHA-256 digests are in
`/home/chris/01_Projects/ts-boardstudio2/.evidence/ticket150/`. Independent review, mutations,
and full repository checks remain with integration owner; Status remains `claimed` pending those gates.

## Outcome

Private preparation is complete; production activation remains in 111. Matrixless Local/Omit/
Rejoin reuses the existing lifecycle and forward projector with an optional exact Matrix pair.
The explicit Local circuit-child and independent connector-reference checks passed independent
changed-scope Standards and Spec review. Final `repo test lint` passed with all 1,821 baselined
native tests; four intentional test additions were recorded through `check.py`. The final
test-only connector-reference assertion passed the focused four-test suite and reviews.

Seven distinct compiled mutations were killed by the relevant focused or library tests, with
exact restoration. A shared deletion-cleanup probe survived the focused matrixless suite because
the forward Module projector independently clears an Omitted connector; the full library probe
killed it via the membership-compaction test. Both scoped outcomes are preserved. The active
over-cleanup probe was killed by the new Local-reference assertion. The initial runtime red is
recorded as a transcript; an independent empty-route mutation automatically reproduced the
expected zero-versus-one Local-draft failure. No raw original-red command log is claimed.
Evidence, accepted hashes and combined mutation results are in `.evidence/ticket150/` in the main
checkout. Source commit `5a2ca4d4`; local merge and post-merge checks are next. No push.
