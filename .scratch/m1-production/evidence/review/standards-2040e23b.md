# Standards review — `2040e23b`

Compared `4cb24113...2040e23b` in `application`, `web` and `scripts`; the only production change is the result-shape validator in `web/src/cad_jobs.rs` plus focused native regression coverage.

## Actionable hard breaches

None. The public provider implementation at `cad/wasm/src/model.rs:229-269` serializes case results as revision, STEP, mesh and optional bodies, without bounds; its STEP import serializer at `:271-286` does include bounds. The candidate accepts absent bounds for `Exact`, rejects malformed supplied bounds, and continues to require valid bounds for `ReadStep` ([cad_jobs.rs](/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/src/cad_jobs.rs:183)). This narrows validation to the actual operation contract, preserves the provider API/serialization, and adds no public API or visibility changes. The regression tests cover all three distinctions.

## Carried judgment-call smells

- `web/src/lib.rs` exposes a broad host/CAD/offline/renderer module surface. Binary/example uses are concrete and the package is unpublished, so this remains a nonblocking API-surface question.
- `web/src/runtime.rs` remains a long coordinator for session effects and browser workflows. Its ownership is accepted for the root runtime; split only around a distinct lifecycle owner.

No source mutation or heavy checks were performed.
