# 10: Integrated mechanical verification and reviews

Status: resolved
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

## Outcome

Resolved 2026-10-09 at integrated `2a8d5e076`. The complete `python3 scripts/check.py`
passed on the integrated source — repo, tooling, lint, build (including the podman CAD
wasm build), test (workspace, footprints, native CAD 51 passed/4 ignored), typecheck and
browser — run on the parent checkout and the non-nested validation worktree at the same
commit (each owning the steps its environment supports: the podman build cannot follow
the validation worktree's cache symlink, and the nested worktree breaks the native CAD
cargo workspace discovery). The boundary transcript reproduces byte-for-byte (native
golden and WASM host parity), so no rebless was needed. Real Session/Core queue ordering
(matrix before mechanical and reverse), one Undo per accepted action, save/reopen and
browser IndexedDB round-trips (pairs, states, reservations through a paired
disable/enable) all pass. Independent Standards and Spec reviews ran over the phase
range `d543201a8...3a00845c` (reports: `/tmp/tce-reviews/`); all blockers were fixed and
re-verified (`a67c266a2` new-instance Local stamping, `acf3d332b` linked changed-id
completeness, `d6ef1715b` + `2a8d5e076` Core-owned case-admission predicate), and the
ratified wire naming and accepted deviations are recorded in ticket 02. The full check
repassed at `2a8d5e076` after the fixes.
