# Private planar qualification checkpoint

The authoritative meanings, future wire types, versions, cap/cache identity and
contribution accounting are the pending-v2 section of
[ANALYSIS_CONTRACT.md](../../../wasm/src/model/generated/ANALYSIS_CONTRACT.md).
The single [shared fixture](../../../wasm/tests/fixtures/generated-analysis-contract.json)
holds its `qualificationV2` examples. Active v1 payloads remain unchanged.

This implementation is private and test-only (`cfg(test)`), using existing
`num-rational`/`num-traits` dev dependencies. It publishes no production
certificate and measures no repair ledger. It does not change detection,
preview admission or frontend delivery. No public/member visibility is widened.

Proof obligations: original decimal source and full face/occurrence census;
complete rational projected/snapped coverage of FINAL exported f64 triangles;
maximum paired vertex displacement for symmetric distance; outward affine norm
and exact transform-rounding accounting; bounded AABB/interval/exact query;
conservative capped paired contact boxes; separate source slab evidence.
Unsupported or incomplete proof remains unknown. The capacitor's touching solids
do not gain parity eligibility. Generated analytic proof is a separate lane and
may use retained mesher parameters; it must still qualify numeric/coverage terms.

Before production wiring, measure WASM size, memory and bounded query cost. The
private proof uses the already locked exact-rational dependency; it does not
assume normalized BigRational arithmetic is an acceptable final hot path.
