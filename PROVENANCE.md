# v2 provenance

The candidate workspace is a private copy derived from the frozen v1 collision diagnosis workspace; the only collision.rs delta against the current qualification vendor is `collision-scale.patch`. V1 files are not changed. The source test harness includes v1's unrelated private seam/winding tests, so use the focused collision command in REPORT.md for attribution.

Baseline input source: `cad/monstertruck-qualification/vendor/monstertruck-meshing/src/analyzers/collision.rs` SHA-256 `658c4da33f80f79b8e92b582a47dcfdf685b3723ebe00d4f386217087ccb0e33`.
Candidate source: `workspace/vendor/monstertruck-meshing/src/analyzers/collision.rs` SHA-256 pending refresh.
Candidate-only patch: `collision-scale.patch` SHA-256 pending refresh after the explicit skinny counterexample test.
Focused/full library output: `lib-test.log` SHA-256 pending refresh.
Standalone copied test executable: `artifacts/collision-tests-v2`.
