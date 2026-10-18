# Map: STEP import validation simplification

## Destination

Replace BoardStudio's six-package per-route STEP repair accounting (007) with the
industry-standard approach documented in
[docs/research/cad/step-import-validation-approaches.md](../../research/cad/step-import-validation-approaches.md):
aggregate loss reports (`require_empty` / `require_lossless`), GVP comparison
(volume/area/centroid per ISO/TS 10303-1039 and CAx-IF RP 4.2), a single
declared-budget repair gate, and a small post-import invariant suite. Reserve
per-route accounting for the collision certificate (014) only. Cut check
times from ~30 min per change (007 slices) to minutes.

## Why

The current 007 approach attempts to certify every IO repair route as
identity/bounded-repair/refusal through six packages of per-route
instrumentation. Each slice requires failing tests first, mutation checks,
conversion reading, 14-minute native test runs, and 15-minute 34-asset corpus
censuses. This is not what ISO 10303 or any other STEP implementation does:

- ISO 10303 validity is **manifold B-rep** (AIC 513/514) plus **GVP agreement**
  (volume/area/centroid within 1%/0.1%). No per-route accounting.
- monstertruck-io already ships `EntityLoadReport::require_empty()` and
  `ShellLoadReport::require_lossless()` — one aggregate call per conversion.
- OpenCASCADE uses load/check/transfer/heal/GVP. No per-route accounting.
- IfcOpenShell validates at schema level and delegates geometry to OCCT.

The six-package approach is a legitimate but expensive refinement that only the
collision certificate (014) needs. For import admission (display/alignment), the
standard's GVP comparison plus monstertruck's aggregate reports are sufficient.

## Decisions

1. **Aggregate reports first.** Use `require_empty()` and `require_lossless()`
   as the primary import gate. These already exist in monstertruck-io.
2. **GVP comparison.** Add volume/area/centroid comparison against embedded
   STEP validation properties. This is the standard's own acceptance test.
   Thresholds: green <1% volume/area, <0.1% centroid (CAx-IF RP 4.2).
3. **Single budget gate.** Keep `max(2 × declared precision, 0.02 mm)` as the
   only repair gate. Reject over-budget. Do not attribute to routes.
4. **Post-import invariants.** Face/edge/vertex counts vs source census, mesh
   nonempty, no NaN/Inf, finite bounds, bytes hash match.
5. **Per-route accounting scoped to 014 only.** Movement bounds needed for
   collision stay in 014, not in import admission.
6. **Defer COPS.** Per-face sampling points are LOTAR/certified-only.
7. **007 packages 2–6 are superseded** by this plan for import admission.
   Per-route accounting that 014 needs is retained there.

## Vocabulary

- **GVP**: Geometric Validation Properties (ISO/TS 10303-1039). Aggregate
  invariants (volume, area, centroid) compared between exporter and importer.
- **Aggregate report**: monstertruck-io's `EntityLoadReport` / `ShellLoadReport`
  with `require_empty()` / `require_lossless()`. One call per conversion.
- **Import admission**: the decision to accept a vendor STEP file for display,
  alignment and collision. Does not require per-route certification.
- **Collision certificate**: the separate question of whether repair movement
  bounds are sufficient for collision queries. Owned by 014.

## Scope

In scope:
- Replace 007's per-route accounting for import admission with aggregate reports
- Add GVP comparison
- Keep single budget gate
- Add post-import invariant suite
- Update 007 ticket to reflect supersession for import admission
- Update 010/012 blockers

Out of scope:
- Per-route accounting for collision (stays in 014)
- COPS (deferred)
- Exact generated cases/PCBs (013, unchanged)
- Keycap mesh pipeline (009, unchanged)

## Issues

1. [01 — Audit current aggregate report usage](issues/01-audit-aggregate-reports.md)
2. [02 — Add GVP comparison](issues/02-gvp-comparison.md)
3. [03 — Post-import invariant suite](issues/03-post-import-invariants.md)
4. [04 — Update 007 ticket and blockers](issues/04-update-007-and-blockers.md)
5. [05 — Verify and close](issues/05-verify-and-close.md)

## Progress

- (none yet)
