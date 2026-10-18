# 162: Bound STEP occurrence expansion before geometry work

Status: claimed
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

- [ ] A regression fails on the current importer because an over-budget mapped
  representation is accepted. Use a bounded synthetic fixture and a deterministic
  refusal oracle rather than attempting memory exhaustion or asserting elapsed time.
- [ ] A finite import-wide budget limits traversal work and emitted occurrences
  before excess instances or their geometry are materialized. It covers branching,
  multiple roots and assembly paths; starting another root does not reset it.
- [ ] Record the chosen limits and their units. Keep enforcement private to import
  admission and preserve existing cycle/depth refusals and completeness checks.
- [ ] Refusal names the exceeded limit and reaches the caller through the normal
  import error path. No partial model is returned as successful geometry.
- [ ] Supported repeated and nested mapped assemblies retain their counts, placement,
  source provenance and mesh coverage below the limits.
- [ ] An actual CAD-worker regression submits an over-budget asset, observes failure,
  then imports an ordinary asset successfully using the same worker.
- [ ] Run affected native STEP/import tests in release mode and the focused worker
  check. Complete one combined Standards+Spec review of the finished behavior under
  the CAD review policy, and record exact results and remaining limits.

## Comments

Created from audit finding 1 at the user's request. This ticket is independent of
repair-movement certification and the other three audit follow-ups.
