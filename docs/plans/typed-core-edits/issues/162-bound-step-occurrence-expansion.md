# 162: Bound STEP occurrence expansion before geometry work

Status: resolved
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [CAD 007](../../cad-backend-replacement/issues/007-strict-step-import.md), [CAD 010](../../cad-backend-replacement/issues/010-production-cache-worker-browser.md)

## What to build

Importing a small STEP file with many nested mapped copies must either deliver a
complete model within a finite work budget or return a clear resource-limit error.
The CAD worker must remain usable after refusal. Keep ordinary mapped assemblies
and their source occurrence identities intact.

The 2026-10-18 audit at `f955050c3` reproduced 2, 16 and 256 occurrences at mapping
depths 1, 4 and 8. The depth-eight file was only 4,066 bytes. Each level references
the preceding representation twice, so file-size and nesting-depth guards do not
bound the total expanded geometry. The probe demonstrated expansion; it did not
attempt to exhaust memory.

## Acceptance criteria

- [x] A regression fails on the current importer because an over-budget mapped
  representation is accepted. Use a bounded synthetic fixture and a deterministic
  refusal oracle rather than attempting memory exhaustion or asserting elapsed time.
- [x] A finite import-wide budget limits traversal work and emitted occurrences
  before excess instances or their geometry are materialized. It covers branching,
  multiple roots and assembly paths; starting another root does not reset it.
- [x] Record the chosen limits and their units. Keep enforcement private to import
  admission and preserve existing cycle/depth refusals and completeness checks.
- [x] Refusal names the exceeded limit and reaches the caller through the normal
  import error path. No partial model is returned as successful geometry.
- [x] Supported repeated and nested mapped assemblies retain their counts, placement,
  source provenance and mesh coverage below the limits.
- [x] An actual CAD-worker regression submits an over-budget asset, observes failure,
  then imports an ordinary asset successfully using the same worker.
- [x] Run affected native STEP/import tests in release mode and the focused worker
  check. Complete one combined Standards+Spec review of the finished behavior under
  the CAD review policy, and record exact results and remaining limits.

## Comments

Created from audit finding 1 at the user's request. This ticket is independent of
repair-movement certification and the other three audit follow-ups.

## Answer — 2026-10-18

Implemented in `add195088`: private import-wide limits of **1,024 solid/curve-set
occurrences** and **16,384 traversal steps** (representation visit, item visit,
assembly path or edge). Budgets cover every root and assembly path before excess
occurrences or geometry are emitted. Existing completeness, cycle/depth,
placement and provenance checks remain.

RED accepted the over-budget fixture before enforcement; GREEN covers six budget
regressions, including traversal-only branching and exact admission at 1,024.
Fresh actual Host/CAD worker smoke passed **7 checks**: named refusal, no successful
partial result, captured identity preserved, and ordinary mapped import succeeds
on the same worker with authenticated source bytes. The initial stream lacked
packages; a parent attempt paired a stale Host artifact with fresh CAD bindings
and failed on missing sourceSha256. The final run rebuilt both providers and
passed (`occurrence-worker-fresh.log`). These are admission limits, not proof of
allocator or process-memory bounds.

Integrated verification at `ed7fe9d5f`: `python3 scripts/check.py` passed
**repo, tooling, lint, build, test and browser** in **1005.7 s**, including native
name guards and freshly built provider/page artifacts. Logs: `.evidence/tickets162-165/full-check.log`;
stream RED/GREEN and mutation receipts: `.evidence/tickets162-165/streams/`.

Claude Opus medium combined closure review: **Standards zero findings; Spec
zero blocking findings**. The initial low eviction concern was withdrawn after
reviewing the existing bounded retry policy and LRU regression. Its optional
error-wording suggestion remains outside this slice; the capacity behavior is
recorded in ticket 164. Reports: `.evidence/tickets162-165/opus-final-review-round1.md`
and `opus-final-review-round2.md`. Review used source inspection and parent test
evidence. Broader CAD migration and performance acceptance remain outside these
four tickets.
