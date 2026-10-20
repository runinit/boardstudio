# 005: Remove leftover Cadrum references

Status: resolved
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

- [x] CAD scripts have no Cadrum benchmark or cleanup paths, unless one is kept deliberately with a comment.
- [x] The CAD README names the current CAD worker location.
- [x] `python3 scripts/check.py repo` passes.

## Comments

Kept deliberately, each with a comment in the file: the Cadrum paths in `measure-cad.py`
(they reproduce the recorded old-versus-new baseline from a historical checkout, and have
370 lines of tests) and the stale-output cleanup in `cad_build.py` (has its own test). The
README now names `web/crates/host/src/cad_worker.rs`. `check.py repo` was also failing on
four dead `GLOSSARY.md` links in older docs; they now point at `CONTEXT.md`.
