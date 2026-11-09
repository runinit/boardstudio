# 03: Post-import invariant suite

Status: resolved
Type: build

## What to build

Add a small, deterministic post-import invariant suite that runs after every
vendor STEP import. These are cheap checks that catch incomplete or corrupt
imports without per-route accounting.

## Acceptance criteria

- [ ] Face/edge/vertex counts in imported shell equal source census counts.
- [ ] Mesh is nonempty (at least one triangle).
- [ ] No NaN or Inf vertices in mesh.
- [ ] Bounds are finite and within a sane multiple of declared precision.
- [ ] Original bytes hash matches (already exists; verify it's in the suite).
- [ ] Every source face has positive f64 mesh coverage (already exists; verify).
- [ ] All invariants run as a single function returning pass/fail + reasons.
- [ ] Add tests: one passing import, one failing (corrupt) import.

## Notes

- These checks are O(mesh) or O(entities), not O(geometry) per route.
- They complement the aggregate reports (01) and GVP (02), not replace them.
- Invariants that are already implemented should be reused, not duplicated.

## CURRENT

(none)
