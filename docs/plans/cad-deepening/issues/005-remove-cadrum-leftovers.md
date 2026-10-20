# 005: Remove leftover Cadrum references

Status: ready-for-agent
Type: build
Blocked by: None
Parent: [CAD deepening](../map.md)
Related: [012: Final cutover, remove Cadrum](../../cad-backend-replacement/issues/012-final-cutover-remove-cadrum.md)

## What to build

The kernel is Monstertruck-only, but the CAD measurement script keeps an old-versus-new
Cadrum benchmark path, the CAD build script still cleans stale Cadrum WASM output, and the
CAD README points at a CAD worker location that has moved. Make the scripts and README
describe only the current kernel.

## Acceptance criteria

- [ ] CAD scripts have no Cadrum benchmark or cleanup paths, unless one is kept deliberately with a comment.
- [ ] The CAD README names the current CAD worker location.
- [ ] `python3 scripts/check.py repo` passes.
