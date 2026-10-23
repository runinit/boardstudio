# 17e: Fixtures and final gate

Status: blocked
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

- [ ] 
