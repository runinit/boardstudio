# 10: Integrated mechanical verification and reviews

Status: ready-for-agent
Type: build
Blocked by: 09
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [02](02-mechanical-settings-patch.md)

## Question / goal

Verify the integrated mechanical work at the Session/Core seam and complete the
ticket's acceptance: matrix-then-mechanical and mechanical-then-matrix ordering with
Undo/Redo, unrelated edits during preparation, deleted/reordered features/vertices,
stale reviewed inputs, fit conflicts, exact export gates and archive reopen.
Regenerate the combined native/WASM boundary transcript once, update the resolver
inventory, remove superseded web mutation/default tests, and run the full
`python3 scripts/check.py` plus Standards and Spec reviews.

## Seams under test

- Session/Core queueing with mixed matrix and mechanical edits.
- The combined boundary transcript (native and WASM).
- Export readiness gates and resolver inventory.

## TDD cases

1. Matrix placement queued behind a mechanical edit and the reverse order both
   settle with correct geometry and one Undo step each.
2. A fit conflict blocks only the affected assembly/board export while saving and
   further editing remain available.
3. Save/reopen preserves pair/feature identity and exceptions.
4. The boundary transcript records the mechanical operation and reproduces through
   the WASM exports.

## Verification

`python3 scripts/check.py repo lint typecheck test browser`, `wasm-pack test
--headless --chrome web/crates/case --locked --lib`, GitNexus `detect-changes`, and
both required reviews.
