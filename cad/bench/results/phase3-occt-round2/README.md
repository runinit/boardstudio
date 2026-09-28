# Continued OCCT optimization screening

[Report and decisions](../../../../docs/occt-optimization-round2.md),
[remaining E0–E9 coverage](../../../../docs/generation-optimization-status.md), and
[reproduction commands](../../../experiments/gasket/README.md).

- `metadata.json`: host/browser, variant list, fixture hashes, session sizes.
- `raw.json.gz`: 2,610 observations: 2,250 warm, 135 cold/warmup, 225 attribution.
- `summary.json`, `analysis.json`: paired session medians, percentiles, attribution and capacity snapshots.
- `native/`: 45 native equivalence and independent STEP comparisons. Incidental native validation timings are not screening measurements.
- `preview-validation.json`: all 45 meshes have zero material-sampling mismatches and zero welded-edge defects.
- `validation-geometries.tar.gz`: native STEP exports and browser geometry buffers.
- `inputs/`: four retained original inputs plus an explicitly synthetic 0.31-radian rotated stress input.
- `eligibility.json`: real tabbed-plate grid sizes and stable primitive inputs; this is an audit, not implemented meshing or reuse.
- `threading-audit.json`: existing diagnostic WASM declares unshared memory; complete threading feasibility remains open.
- `provenance.json`: staged sources/binary hashes, reproducible patch verification, and failed initial build setup.

All five sessions, including the slower fifth session, are retained. No causal
host explanation, full-app improvement, live-memory/RSS result, combination gain,
or production acceptance is claimed. No case geometry or manufacturing tolerance
was changed to make a candidate faster.
