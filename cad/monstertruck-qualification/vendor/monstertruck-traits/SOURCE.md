# Vendored source provenance

This directory is the crates.io `monstertruck-traits` version `0.4.1` package,
licensed by its manifest as Apache-2.0 and attributed upstream to Yoshinori
Tanimura and Moritz Moeller. The full Apache License, Version 2.0 is included in
`LICENSE-APACHE`.

The clean package archive was obtained from the local Cargo registry cache:
`monstertruck-traits-0.4.1.crate`, SHA-256
`77b8f72eb09b5f613f6ccc314c53bc4c43de1a0d1d5a29eae61f8b9b8b04eed1`.
The published `src/` files match the frozen clean STEP-candidate package
source. Before applying this candidate patch,
`src/algo/surface/mod.rs` had SHA-256
`428782034100a996a2209ade00f62aafc9930797e539d9828c427e393776afec`; this is
also the surface-module hash in the frozen STEP source. The frozen Boolean
candidate snapshot already contains the reviewed result and has module hash
`5f66ca597e530c1e857cadff408e404b7ff73852f35578cce1a4b0f1950a5863`.

Only the following reviewed patch was applied:

- Patch: `.evidence/monstertruck-qualification/boolean-fix/fix.patch`
- SHA-256: `0c6d3ebd5ec9fbf6861b525055a3d815fbdb35e54c0fe0e791397d3a4b66e8ed`
- `src/algo/surface/mod.rs`: corrects directional error measurement when choosing which parameter axis to refine.
- `src/algo/surface/axis_division_tests.rs`: adds regression tests for one-axis, two-axis, cancellation, and bilinear-saddle cases.

The patch was reviewed independently with no blockers. The source is vendored
only so the qualification crate can reproducibly apply this candidate through
its local Cargo patch; it is not a production dependency change. See the
qualification README and the Boolean-fix evidence handoff for test results and
known limits.

## Reviewed sphere pole qualification

Adds a default-none analytic singularity capability with reference and Box forwarding. Reviewed combined source and controls: `.evidence/monstertruck-qualification/step-sphere-vertex-loop-v1/revision-v7-combined-current/`, freeze SHA-256 `f9cb21e0afd40c950452908e407445d8181f64a7a5c8612585c4dd2ed1cf588a`. Bounded qualification only; full STEP parity is not established.

## Reviewed Plane source-transform qualification

Plane-v40 adds an exact rational affine-plane certificate constructed from actual source coefficients, including homogeneous projection, and default-none surface capability with reference/Box forwarding. It adds the already-pinned num-rational dependency. Reviewed v3 full patch SHA256 `a4c879d1964a93a04a9e98d282dc20ed5a9a04c12d52f811ba6667b5abd00157`; immutable source and compatibility notes are under `.evidence/monstertruck-qualification/plane-source-transform-v3/primitive-freeze-v1/`. The certificate is unoriented and does not establish material orientation or complete STEP parity.

Choc-v41 integrates the reviewed finite-boundary coefficient proof and ordinary inferred-chart route, preserving successful aligned source samples before chart reconstruction. This is finite boundary equality, not periodicity or general surface injectivity. Final reviewed production patch SHA256 `2769aef56108ac12a86b8d2047cc8df976dcfdd3eb9263103122f79e5e1b4d1c`; final private source manifest `f2b66f3445084aacec04adab536bc78cc6919aaba0071caf1fb0bb69a7965bbe`. Two adjacent trait/derive capability additions are separately reviewed with Plane-v40 (composition patch `77697632c0345708a50e7641e82369826a7e5a3c211bffdd6b7365dba7b13c25`). Frozen evidence: `.evidence/monstertruck-qualification/choc-default-incidence-proof-v1/`; shared combined checks: `vendor-integration/choc-v41/`. Existing rejected heuristics and endpoint-loss predecessors remain rejected; whole STEP/feature acceptance and OCCT/C++ removal remain open.

## Reviewed finite collapsed-boundary support (2026-10-17)

Collapsed-pole-v49 integrates the reviewed finite polynomial B-spline point-boundary proof, wrapper forwarding and axis-specific shared-sample projection. Proof transport requires a conservative affine determinant enclosure excluding zero; unsupported or ambiguous source cases receive no proof. Original source geometry and shared XYZ samples are retained. Combined patch SHA-256 `f804cd140dd3dab104f775f26f0fc8f098a9b646fe87511ae2760e8e3b164990`; immutable candidate and both review axes: `.evidence/production-step-007/collapsed-v-pole-fix/`. Actual shared integration receipts: `.evidence/monstertruck-qualification/vendor-integration/collapsed-pole-v49/`. This does not certify arbitrary source trim coverage, whole-shell closure or collision accuracy.
