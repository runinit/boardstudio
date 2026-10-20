# Spec: STEP import validation simplification

Status: resolved
Source: [research/cad/step-import-validation-approaches.md](../../research/cad/step-import-validation-approaches.md)
Map: [map.md](map.md). Tickets: [issues/](issues/).

Historical specification, not an active work order: the Solution, User Stories and
Implementation Decisions below record an earlier proposal. Do not implement
their aggregate-completeness refusal or proposed 007 edits. Current preview
admission follows the vendor-preview policy in
[map.md](map.md#current-vendor-preview-policy-2026-10-19). An aggregate loss is
reported as an explicit incomplete-model diagnostic; it does not by itself
refuse the surviving geometry. The resolved and wontfix issues below are not
reopened.

## Problem Statement

BoardStudio's STEP import validation (ticket 007) has ballooned into six packages
of per-route accounting work that attempt to certify every IO repair route as
identity/bounded-repair/refusal. Each slice requires failing tests first, mutation
checks, conversion reading, ~14-minute native test runs, and ~15-minute 34-asset
corpus censuses. This work consumes the cutover timeline without adding functional
value. The cutover (tickets 010, 011, 012, 014) is functionally complete — the
build passes, wasm is pure Rust Monstertruck, no cadrum/OCCT in production,
34/34 vendor STEP files import correctly — but remains formally blocked by 007's
deep accounting.

The root cause: 007 is solving a question that the STEP standard and every other
STEP implementation answer differently. ISO 10303 defines validity as manifold
B-rep (AIC 513/514) plus GVP agreement (volume/area/centroid comparison). It
does not require per-conversion-route accounting. monstertruck-io already ships
aggregate loss reports (`require_empty` / `require_lossless`). OpenCASCADE uses
load/check/transfer/heal/GVP. No implementation does per-route accounting for
import admission.

## Solution (historical proposal)

Replace the six-package per-route accounting with the industry-standard approach:

1. **Aggregate loss reports** (already exist in monstertruck-io): `EntityLoadReport::require_empty()`
   after table build; `ShellLoadReport::require_lossless()` after each conversion.
   The former proposal refused if either failed; current policy requires a
   reported loss and retention of safely usable geometry with a diagnostic.

2. **GVP comparison**: Dropped per research. ISO does not require GVP for import
   admission; no real vendor files carry it; no Rust crate parses it. Retain
   aggregate reports + invariants + budget gate instead. Revisit only if a
   certification contract (LOTAR/COPS) demands it.

3. **Single budget gate** (already exists): `max(2 × declared precision, 0.02 mm)`.
   Reject over-budget repairs. Do not attribute to routes.

4. **Post-import invariant suite** (mostly exists): face/edge/vertex counts match
   source census, mesh nonempty, no NaN/Inf, finite bounds, bytes hash match.

5. **Per-route accounting scoped to 014 only**: Movement bounds needed for collision
   queries stay in 014, scoped to collision-relevant routes only.

This collapses six packages into: one aggregate report call + one optional GVP
comparison + one budget check + a small invariant suite. Check times drop from
~30 min per 007 slice to minutes.

## User Stories (historical proposal)

1. As a developer, I want STEP import validation to use the standard's own
   acceptance test (GVP comparison), so that we're not inventing a stricter
   standard than ISO 10303 requires.

2. As a developer, I want import admission to be gated by aggregate loss reports
   that already exist in monstertruck-io, so that we're not reimplementing
   what's already there.

3. As a developer, I want a single budget check for repair, so that over-budget
   repairs are rejected without per-route attribution.

4. As a developer, I want a small invariant suite to catch incomplete/corrupt
   imports, so that basic sanity is verified cheaply.

5. As a developer, I want per-route accounting confined to the collision
   certificate (014), so that the expensive work is only done where needed.

6. As a developer, I want 007 to no longer block 010/011/012, so that the
   cutover can close.

7. As a developer, I want check times for validation changes to be minutes
   instead of 30+ minutes, so that iteration is practical.

8. As a vendor STEP consumer, I want imports to be validated by aggregate
   invariants (volume, area, centroid) that I can independently verify, so
   that I trust the import quality without understanding internal routes.

9. As a collision analyst, I want per-route movement bounds retained for
   collision-relevant routes, so that collision accuracy certificates remain
   available.

## Implementation Decisions (historical proposal; do not apply to 007)

- The primary seam is the public import function `import_step_with_options` in
  `cad/monstertruck/src/step/mod.rs`. All validation (aggregate reports, GVP,
  budget, invariants) runs inside this function before returning. This is the
  single seam; no new seams are introduced.

- Aggregate reports: `table.entity_report.require_empty()` is already called.
  Shell-level `require_lossless()` must be added after each conversion. The
  un-suffixed (report-discarding) conversion methods should be audited and
  converted where strictness is appropriate; lossy-by-design conversions (e.g.
  `DegenerateVertexLoop` for point boundaries) must remain non-strict.

- GVP comparison: Parse `UNCERTAINTY_MEASURE_WITH_UNIT` and related validation
  properties from the STEP file. Compute volume, surface area, and centroid from
  the imported B-rep. Compare using CAx-IF RP 4.2 thresholds. If GVP is absent,
  skip comparison and record "no GVP" in import metadata. The comparison result
  (green/yellow/red) is stored in `ImportedStep` alongside existing accuracy
  metadata.

- Budget gate: The existing `preview_repair_budget_mm` / `PreviewTolerance`
  mechanism remains unchanged. No new budget logic is added.

- Post-import invariants: A single function `validate_import_invariants` runs
  after conversion: (a) face/edge/vertex counts match source census,
  (b) mesh is nonempty, (c) no NaN/Inf vertices, (d) bounds finite and within
  sane multiple of declared precision, (e) bytes hash matches (already exists),
  (f) every source face has positive mesh coverage (already exists). Returns
  `Result<(), Vec<String>>` listing all failures.

- Per-route accounting for collision: The existing `repair::Ledger` and
  `StepRepairOperation` remain for 014's collision certificate. They are not
  removed, but are no longer required for import admission. The 007 ticket is
  updated to scope per-route accounting to collision-relevant routes only.

- 007 ticket: Packages 2–6 are re-scoped. Package 1 (resource accounting S1)
  is retained as a memory-hygiene measure. Packages 2–6 (per-route accounting)
  are retained only for 014's collision certificate, not for import admission.

- Blocker updates: 010 no longer blocked by 007 packages 2–6. 011 (performance)
  unblocked. 012 no longer blocked by 007. 014 retains 007's per-route work
  for collision routes.

## Testing Decisions

- Test at the highest seam: `import_step_with_options` (public API). All
  validation is exercised through this entry point. No new test seams are
  introduced.

- Good tests verify external behavior: given a STEP file (valid, corrupt,
  over-budget, missing GVP), the import succeeds or fails with the expected
  error. Tests do not check internal route instrumentation.

- Prior art: existing tests in `cad/monstertruck/src/step/` and
  `cad/monstertruck-qualification/tests/` that call `require_empty()` /
  `require_lossless()`. The new tests follow the same pattern but at the
  `import_step_with_options` level.

- Test cases:
  - Valid STEP file with GVP: import succeeds, GVP green.
  - Valid STEP file without GVP: import succeeds, "no GVP" recorded.
  - Corrupt STEP file (missing faces): import fails with aggregate report error.
  - Over-budget repair: import fails with budget error.
  - Mesh with NaN vertices: invariant suite fails.
  - Empty mesh: invariant suite fails.

- Mutation testing: not required for this plan. The aggregate reports and
  invariants are simple enough that mutation checks add little value. Per-route
  accounting in 014 retains mutation checks.

## Out of Scope

- COPS (per-face sampling points) — LOTAR/certified only, deferred.
- Per-route accounting for import admission — superseded by this approach.
- Exact generated cases/PCBs (013) — unchanged.
- Keycap mesh pipeline (009) — unchanged.
- Collision certificate per-route accounting (014) — retained, scoped to
  collision routes.
- Changing the repair budget formula — unchanged.

## Further Notes

- The research document at
  `docs/research/cad/step-import-validation-approaches.md` contains the full
  analysis, industry comparison, and source citations.
- The 34-asset corpus census remains as a one-time compatibility gate for the
  new enforcement, not a per-change requirement.
- Expected check time reduction: from ~30 min per 007 slice (native tests +
  corpus census) to ~2-5 min (native tests only, no census per change).
- This plan does not delete the 007 deep accounting code; it re-scopes it.
  The per-route instrumentation remains available for 014's collision
  certificate.
