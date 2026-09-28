# Phase 3 gasket comparison artifacts

See [the report](../../../../docs/gasket-comparison-results.md) and
[reproduction commands](../../../experiments/gasket/README.md).

- `metadata.json`: browser/CPU, input hashes, session/sample counts.
- `provenance.json`: exact staged source, input, lockfile, dependency, and binary hashes.
- `summary.json`, `analysis.json`: per-fixture/session comparisons, stage timing, capacity observations.
- `raw.json.gz`: all 1,390 observations, including 1,200 warm speed observations, 90 cold/warmup observations, and 100 separately instrumented attribution observations.
- `inputs/`: the unchanged regression bottom and three captured gasket-edit states.
- `native/`: four sets of native equivalence and independent STEP checks.
- `preview-validation.json`: material occupancy, closure, normals, volumes, and approximation limits.
- `monstertruck/`: failed native construction runs and independent validation of the same input solids.
- `failure-investigations.json`: retained initial geometry and browser-harness failures and their resolution.
- `validation-geometries.tar.gz`: browser geometry JSON, native STEP outputs, original pre-fix profile STEP files, and Monstertruck input solids. Paths inside the archive identify their run.

The native geometry-check reports include incidental timings captured while
validating; they are not comparable benchmark samples. Use browser warm samples
for the Cadrum/Manifold diagnostic. Monstertruck failure wall times are not
completed generation latencies. No production promotion, full UI acceptance,
process-memory comparison, or Monstertruck browser ranking is claimed.
