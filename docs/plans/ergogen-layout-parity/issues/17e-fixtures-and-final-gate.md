# 17e: Fixtures and final gate

Status: claimed
Type: build
Blocked by: [17b](17b-validation-and-ownership.md), [17c](17c-kernel-production-routing.md), [17d](17d-entry-path-gates.md)
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns fixture regeneration and the changed-file and full gates for the cutover; the example/starter rebuilds are left to [26](26-rebuild-examples-and-fixtures.md) and the end-to-end checklist and release gate to [27](27-end-to-end-acceptance.md).

## Outcome

Close ticket 17: regenerate the minimum affected fixtures, run the gates and review.

## Acceptance

- Regenerate the minimum affected baseline schema/scene fixtures deliberately; do not weaken tests.
- Changed-file quick gate and the full check gate pass.
- Standards and Spec review before closure.
- Keep current storage transaction, identity, companion, scan/keymap and history contracts intact. Remaining feature tickets must complete before this lane ships.

## Checks

Use `check.py quick --files` for all changed callers and the full check gate; require native/contract and WASM compilation, not just serde tests.

## Progress

### Implementation claim — 2026-10-23

- Verified branch `docs/ergogen-layout-comparison` at pinned
  `42abce93c3854f3f76de43f0a0b26cc6e71f3e81`; 17a–17d headers are resolved.
  Only preexisting untracked `web/crates/layout/prototype/` is present; it is excluded.
- Plan: fix the review base before grouped-field replacement using Git history and
  17a's Answer; inventory the entire coordinated range, audit schema/scene/boundary
  fixtures and reuse deliberate 17a rebuilds where sufficient. Change only fixtures
  with a spec justification; leave examples/starters to 26 and release/E2E to 27.
- Then run fresh all-caller changed-file quick followed, only if green, by the
  supported full pre-PR scheduler. Every heavy command uses the shared flock,
  22G memory/no-swap scope, 3600-second timeout, absolute TMPDIR and two test threads.
  Keep native/WASM baselines unchanged; stop on first failure or uncertain drift.
- Record exact inventory, commands, receipts and observed/unreached scopes here.
  Leave this ticket and parent 17 claimed for independent Standards+Spec reviews;
  no source fixes, later claims, shipping or user-data writer approval.
