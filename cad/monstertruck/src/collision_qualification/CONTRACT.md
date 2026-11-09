# Private planar qualification checkpoint

The authoritative meanings, wire types, versions, cap/cache identity and
contribution accounting are the v2 section of
[ANALYSIS_CONTRACT.md](../../../wasm/src/model/generated/ANALYSIS_CONTRACT.md).
The single [shared fixture](../../../wasm/tests/fixtures/generated-analysis-contract.json)
holds its `qualificationV2` examples. Active v1 payloads remain unchanged.

This implementation stays private to the crate. Production access runs through
the qualification facade in `cad/monstertruck/src/qualification.rs`, which owns
the sealed evidence and the only public preparation and query entry points; it
uses the `num-rational`/`num-traits` dependencies of `boardstudio-cad`. It
publishes no certificate on its own and measures no repair ledger. It does not
change detection, preview admission or frontend delivery. No public/member
visibility is widened.

Proof obligations: original decimal source and full face/occurrence census;
complete rational projected/snapped coverage of FINAL exported f64 triangles;
maximum paired vertex displacement for symmetric distance; outward affine norm
and exact transform-rounding accounting; bounded AABB/interval/exact query;
conservative capped paired contact boxes; separate source slab evidence.
Unsupported or incomplete proof remains unknown. The capacitor's touching solids
do not gain parity eligibility. Generated analytic proof is a separate lane and
may use retained mesher parameters; it must still qualify numeric/coverage terms.

WASM size is recorded in the [Monstertruck release baseline](../../../../docs/research/cad/monstertruck-performance-baseline-2026-10-18.md);
the bounded query cost and retained memory of the qualified path are measured
by `cad/wasm/tests/qualified-analysis.mjs`. The private proof uses the
already locked exact-rational dependency; it does not assume normalized
BigRational arithmetic is an acceptable final hot path.
