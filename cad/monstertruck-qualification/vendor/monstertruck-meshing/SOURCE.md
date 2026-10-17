# Vendored source provenance

This is the published crates.io `monstertruck-meshing` version `0.4.1` package.
Its manifest declares Apache-2.0 and attributes Moritz Moeller; the Apache
License, Version 2.0 is included as `LICENSE-APACHE`.

The source archive `monstertruck-meshing-0.4.1.crate` has SHA-256
`ea03901ea0b3cfaf380c1cc8c2efee85500cdfe260c38bb8278c2a66a18de939`.
The first reviewed candidate is
`.evidence/monstertruck-qualification/step-mesh-fix/seam-fix-final.patch`,
SHA-256 `3ba6663b84a867f30d9b2b03a40d895cbdf164ca38592c013705660a2dc8346c`.
It reconciles closed edge-piece parameter branches only when the shifted
samples remain within the surface domain and match the paired 3D samples. Its
persistent tests cover candidate validation, a four-piece cylinder loop, the
aligned-exact pipeline, and parameterization/model-unit scaling.

The incremental reviewed candidate is
`.evidence/monstertruck-qualification/step-mesh-fix/co-sample-pairing.patch`,
SHA-256 `155c8cdf43854a9cc98ac2171eb8f7af45ccdf1f577ae54b20ba69c44fa0caac`;
its independent review report has SHA-256
`19d98734bc49ebbea679a0011aa298ad1f55329c7578605e2f5d57a509bbc15b`. It
preserves UV boundary samples only when evaluating each UV on the surface
reproduces its paired 3D edge sample within the caller's tolerance; otherwise
the existing arc-length resampling path remains in use. Tests cover valid
co-sampled pairs, a same-length independently sampled mismatch, tolerance
failure and non-finite samples. The patch changes only projection and boundary
test source; it adds no dependencies.

The normalized manifest has one qualification-only test metadata addition:
`monstertruck-modeling 0.4.1` with default features disabled. The published
source's ignored `par_bench` test module imports it; this declaration allows
the crate's existing test targets to compile without enabling modeling's broad
default feature set. `serde_json` was already present as a published dev
dependency. No benchmark source or production dependency is changed.

These local patches exist only in the standalone qualification workspace. They
reduce the measured THQ gate failure but do not resolve all THQ or MX mesh
failures, and they do not change the production CAD dependency graph.

The reviewed CDT triangle-winding patch is
`.evidence/monstertruck-qualification/preview-contract-assessment/production-candidate-0p1/face16333-cdt-winding-candidate-v2/winding-candidate-v2.patch`
(SHA-256 `4000b9e9c65200d8c876b8b752113110a5bfccc854e8aaf6e85218dead997bc2`).
Trimmed triangle mode retains Spade's counterclockwise UV winding instead of
using a whole-mesh physical-normal vote that coarse chords can reverse. Grid
and quad modes keep their existing behavior. Maintained analytic-sphere tests
cover UV/Jacobian orientation, one false-face inversion, and a baseline-red
trimmed-CDT regression. Remaining coarse-chord errors and STEP sphere-import
loss are not repaired by this change.

The reviewed tiny-trim v2 guard rejects explicit nonperiodic boundary pieces
with fewer than three projected points before they can become full-domain
faces or filled holes. All three tessellation routes honor the invalid
boundary. Empty-wire full surfaces are preserved. Evidence and regression
controls: `.evidence/monstertruck-qualification/step-tiny-tessellation-v2/`.
This adds safe rejection, not support for meshing below-resolution trims.

## Reviewed sphere pole qualification

Canonicalizes only exact source-declared singular parameter rows and emits pole fans beside them; generic meshes retain their sampled topology. Reviewed combined source and controls: `.evidence/monstertruck-qualification/step-sphere-vertex-loop-v1/revision-v7-combined-current/`, freeze SHA-256 `f9cb21e0afd40c950452908e407445d8181f64a7a5c8612585c4dd2ed1cf588a`. Bounded qualification only; full STEP parity is not established.

## Tiny STEP composite on sphere-v8

Normalizes tiny UV working spans for CDT and inclusion, maps back before source evaluation/output, preserves close distinct samples, and tightens sample pairing when the chord budget exceeds the physical extent. The independently reviewed six-file candidate is `.evidence/monstertruck-qualification/normalized-cdt-sphere-v8-combined-v1/`; report SHA-256 `dd297d2451e73f6b1661e7ed2191f0f575b3b583520fc5ac7ddf5c79d7372d20`, artifact ledger `d9a587883e5ace2a587d02c5764264df73c9e457af5d45155e8582661c591989`. PICO geometry restoration is bounded evidence; its fixed-quantization strict gate still rejects.

## Exact collision qualification

The reviewed collision candidate uses outward interval rejection with exact rational fallback, a finite-coordinate sweep and exact endpoint-distance ordering. It fixes the reproduced oblique boundary miss and tiny-scale endpoint collapse without coordinate welding. The final rebased source is frozen in `.evidence/monstertruck-qualification/astra-collision-integration-v2/` (source inventory `d96077094eb10a7f83226541804896bb6124403c14e9c1c961201f295ffce437`). Rational arithmetic is a runtime dependency; robust predicates are used only by test diagnostics. The scratch candidate passed 68 meshing tests (1 ignored) and the WASM library check. Full production throughput, WASM runtime and complete Boolean qualification remain open.

## Reviewed tangent-metric refinement

Ports constraint-preserving diagonal flips into actual trimmed-face CDT emission. Only unconstrained internal diagonals change; source positions, UVs, normals and trim boundaries remain intact. Bounded nonconvergence and invalid metrics propagate typed failures through strict and lenient paths. Three FaceDropReason variants affect downstream exhaustive matches. Frozen reviewed source: `.evidence/monstertruck-qualification/astra-surface-refinement/vendor-metric-candidate/frozen-source-inventory.sha256`, SHA-256 `9b73944385832246b13d86bf7c8c66dbf07653fe06ba27f5b2c63e2a296b627b`. Real unseeded THQ face distance improves, but angular diagnostics and complete-model coverage still fail. No general accuracy or production qualification is claimed.

## Adaptive collision sweep

The independently reviewed sweep chooses the coordinate axis with the fewest inclusive projected interval pairs, preserving exact narrow predicates and endpoint construction. Durable sparse-separation and touching/zero-width regressions are included. Source and performance evidence: `.evidence/monstertruck-qualification/astra-sweep-integration-v15/` and `astra-collision-performance-v13/`. Three sorts add overhead to already-separated x workloads; dense rational intersections remain costly. Segment order and first witness can change; the API does not promise either. Root rebased only collision.rs and verified meshing tests, Boolean smoke controls and WASM compilation.

## Exact pole-row loop assembly

Canonicalizes repeated boundary representatives only on exactly equal source-declared singular rows; nearby UV rows remain distinct even when XYZ rounds to the pole. Reviewed supplement: `.evidence/monstertruck-qualification/v16-exact-row-integration-supplement/`, with additive review in `sol-pole-review/`. Root corrected the test sphere mixed derivative before integration. The permanent test covers local/translated forward/reversed V-pole loops, not full STEP validity; full Xivik remains unqualified. Root integration evidence: `vendor-integration/pole-v18/`.

Dish-v28 admits resolved constrained UV wedges only with finite physical geometry, source orientation, positive domain winding and resolved local tangent area at the unchanged tolerance. Reviewed mesher-v2 patch SHA256 `a11b28ee5a6627b1747fff6c25161058f9ef8e23adde148de8c6d1c54c93a61b`; evidence: `.evidence/monstertruck-qualification/dish-shared-events-v1/mesher-checkpoint-v2/`. Exact transformed-cylinder red/green and DSA gates pass. This is local admission, not general trim certification; ordinary-path physical-collapse regressions remain open.

Chart-v30 completes a certified seam pin with the assigned endpoint only after validating its active domain and actual XYZ residual at the unchanged tolerance. Reviewed combined patch SHA256 `db231af77b36fec4a257146c0909a93c8144c486374d566002e2d3b8ed4c596e`; evidence: `.evidence/monstertruck-qualification/choc-closed-chart-representative-v1/`. The original STEP fixture retains five source trailing-whitespace lines. Full Choc mesh validity remains unqualified.

Identity-dish-v32 carries canonical source-vertex identity through native, compressed and trimmed boundary paths and excludes triangles repeating a known source vertex. Coordinates, UVs, sampling and tolerances remain unchanged; unknown/conflicting identities remain conservative. Reviewed production patch SHA256 `dd791eeab7a0723d9df33f164563ec11331ae05e8184e262ce7eb1d276359b63`; frozen source, matched full gates and independently referenced sample retention: `.evidence/monstertruck-qualification/crenshaw-regression-v1/FINAL-BINDING.json`. This is not a general degeneracy or whole-Choc validity certificate.

Surface-v33 replaces sampled-normal planarity inference with ordinary surface divisions unless the carrier explicitly proves an affine plane. The default-false capability is forwarded through wrappers; exact finite nondegenerate Plane and safe affine Processor carriers preserve boundary-only tessellation. Reviewed combined patch SHA256 `ba8bec3bcd31b69e7669ef562c848afe6ac904af01a7cd0142ffecbb3cd7b4b1`; frozen source, twelve-face coverage and mutation receipts: `.evidence/monstertruck-qualification/choc-post-chart-validity-v1/HANDOFF.md`. Whole Choc validity remains unqualified.

Pole-v37 adds reviewed canonical source-pole boundary lifts and preserves original XYZ through periodic UV normalization. Frozen production patch SHA256 `5d7957520eba11154eccb5b4069a0e7ad183a3d79ac8c578dc44fa3c1dc3f186`; regression patch `d26dc500972b467982fa77e2c9412ccdf800900a79d5a9b38507c53359f3142c`. Evidence: `.evidence/monstertruck-qualification/choc-remaining-incidence-v1/FINAL-BINDING.json`. The private checked/angular Choc gates pass and exact retention has zero newly missing originals; numerical coverage is bounded to authenticated spherical faces. Default Choc and full corpus qualification remain open. Shared integration receipts are under `vendor-integration/pole-v37/`.

Choc-v41 integrates the reviewed finite-boundary coefficient proof and ordinary inferred-chart route, preserving successful aligned source samples before chart reconstruction. This is finite boundary equality, not periodicity or general surface injectivity. Final reviewed production patch SHA256 `2769aef56108ac12a86b8d2047cc8df976dcfdd3eb9263103122f79e5e1b4d1c`; final private source manifest `f2b66f3445084aacec04adab536bc78cc6919aaba0071caf1fb0bb69a7965bbe`. Two adjacent trait/derive capability additions are separately reviewed with Plane-v40 (composition patch `77697632c0345708a50e7641e82369826a7e5a3c211bffdd6b7365dba7b13c25`). Frozen evidence: `.evidence/monstertruck-qualification/choc-default-incidence-proof-v1/`; shared combined checks: `vendor-integration/choc-v41/`. Existing rejected heuristics and endpoint-loss predecessors remain rejected; whole STEP/feature acceptance and OCCT/C++ removal remain open.

## Reviewed finite collapsed-boundary support (2026-10-17)

Collapsed-pole-v49 integrates the reviewed finite polynomial B-spline point-boundary proof, wrapper forwarding and axis-specific shared-sample projection. Proof transport requires a conservative affine determinant enclosure excluding zero; unsupported or ambiguous source cases receive no proof. Original source geometry and shared XYZ samples are retained. Combined patch SHA-256 `f804cd140dd3dab104f775f26f0fc8f098a9b646fe87511ae2760e8e3b164990`; immutable candidate and both review axes: `.evidence/production-step-007/collapsed-v-pole-fix/`. Actual shared integration receipts: `.evidence/monstertruck-qualification/vendor-integration/collapsed-pole-v49/`. This does not certify arbitrary source trim coverage, whole-shell closure or collision accuracy.
