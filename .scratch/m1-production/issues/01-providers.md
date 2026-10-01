# 01: Reconcile the exact newer provider reference

**What to build:** Open and round-trip copied projects using the newer encoder/VIK/module providers without losing accepted migration contracts.

**Blocked by:** None (can start immediately)

**Status:** complete — exact provider tree and affected checks verified; independent review has no provider findings

**Category:** behavior-preserving migration plus explicitly approved transparent Rust enum boxing (RUN approval, 2026-10-01); ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [x] Record exact dev and migration source revisions and preserve original checkout/stashes.
- [x] Resolve overlapping edits without reverting enum boxing, lossless core JSON or lifecycle fixes.
- [x] Pass affected provider tests, contracts, formatting and strict lint or record actual blockers.

Acceptance: provider tree (`core`, `contracts`, `renderer`, `cad`, `app/src`)
is unchanged from verified provider candidate `3a43cb3d` to integration
`680972a4`. Retained [provider command evidence](../evidence/providers/SUMMARY.md)
covers native/WASM build, fmt/strict lint, contracts, provider/package tests and
actual provider browser tests. The reference production build and fresh contract
check pass in [integrated evidence](../evidence/integration/). Independent
Standards/Spec reviews found no provider issues; the session export finding is a
downstream ticket 02 repair. Read-only preservation check on 2026-10-01 confirms
original `dev@5a472a9426e6e38993361da402cd4ec730feb369`, unrelated `.scratch/`, and
stashes `b255f75ab8ffc2a0a63967ca6d0ea57a0d9050ea` and
`26aaa63bf14a3840e8725657d39226d4aaec47bf` remain intact. Imported upstream
whitespace is retained; broad diff whitespace diagnostics are inherited rather
than an authored-source formatter failure. This closes provider reconciliation
only; it does not close the complete M1 browser/performance acceptance gates.
