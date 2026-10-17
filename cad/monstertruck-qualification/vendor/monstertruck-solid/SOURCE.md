# Vendored source provenance

This directory is the published crates.io `monstertruck-solid` version
`0.4.1`, extracted from `monstertruck-solid-0.4.1.crate` with SHA-256
`29e8fab1d370bfe553a171c0b0948810eefc66ecb2e3dca87e26c985bda27e50`.
The package manifest declares Apache-2.0 and attributes Moritz Moeller. The
published archive does not include license text; `LICENSE-APACHE` is the
standard Apache-2.0 license text also bundled with the other vendored
Monstertruck crates in this qualification workspace.

The first source patch is
`.evidence/monstertruck-qualification/boolean-fix/endpoint-priority.patch`,
SHA-256 `cc9cce211c40dda8abc7020a82892897ac0361917ba210285176edbe925579f7`.
It changes `src/transversal/classic/loops_store.rs`; the patched file SHA-256
is `ced1a284adec5b44da0cae2b02343d4f9501dbde52a2b33a725b49d109bc7599`.
The exact endpoint regression and nearby-interior control are included in that
reviewed patch. The upstream nightly formatter config is not in the crate
archive; the source-tree config is used for changed-file formatting. The
format-only delta on top of the approved patch is
`.evidence/monstertruck-qualification/vendor-integration/endpoint-candidate/fmt/endpoint-format-only.diff`
(SHA-256 `cf62353f04f577048a7994934cd26bf418f61f57d05d7573d7ca3b4218a8dabe`).
It touches only the added helper and regression tests. The final formatted source
SHA-256 is
`56b0ec68b16ef7c4aef7d74b31739ceb6681dcbeee2fe1677f7c4c39c503888d`. The
frozen pristine source file from the archive has SHA-256
`9ac5b8d7c49c0beb04febfe24b22fd3936c0f9a2ed7e0b3a4663c2aa88428fd2`.

The fix checks exact equality with existing absolute edge endpoints before
falling back to the unchanged approximate curve search. It does not change
search tolerance. When multiple loop edges have coincident endpoints, the
first matching loop edge still wins; this patch does not disambiguate that
case. The candidate is qualification-only and is not a production dependency
change or a general Boolean pass claim.

The approved sub-tolerance intersection converter patch is
`.evidence/monstertruck-qualification/boolean-fix/converter-short-edge.patch`
(SHA-256 `100c7ea5c825abea121e0eabe04921af2911d7aa5af1606eedd55d8a55ee5987`).
It changes `src/transversal/classic/mod.rs` only for `Alternative::SecondType`
leader samples: reject non-finite coordinates, remove consecutive exactly equal
points, and retain distinct samples regardless of distance. It preserves the
existing degree-1 B-spline and closure validation; generic tolerances are
unchanged. The patched source SHA before formatting is
`56ca4bc6a9a091892b057f73141b308a34e9864a7409fc66aee2e62bcdb236bc`. The
format-only delta is
`.evidence/monstertruck-qualification/vendor-integration/converter-candidate/fmt/short-edge-format-only.diff`
(SHA-256 `805d87e37a7ede901803171116e29f789c6644ba22c02e76c272d7fc8b65f3ba`);
the final formatted file SHA-256 is
`912b093214f8a46b028647ee6096b51f8bd0a5285dc4f0557994b446d522ed36`.

The endpoint baseline-red regression run is
`.evidence/monstertruck-qualification/boolean-fix/profiles/endpoint-priority-final-baseline-red.log`
(SHA-256 `e5161bdcdb2658a5a94f5ee4c9aca1e5ef5dbf1b8b0aff1ca057c1f863dcda4d`);
the isolated fixed library run passed all 11 tests at
`.evidence/monstertruck-qualification/boolean-fix/profiles/endpoint-priority-clean-full-lib-tests.log`
(SHA-256 `392668d25e12581a672720398dca5cde4665a9fd1a3372e3eb4e507ccab91dbf`).
The integrated Boolean gate logs and current tests are separately recorded in
`.evidence/monstertruck-qualification/vendor-integration/endpoint-candidate/`.
The converter baseline-red report is
`.evidence/monstertruck-qualification/boolean-fix/profiles/converter-clean-baseline-red.log`
(SHA-256 `01eac0b2c749d662af6649646a064950f19da3d9df4a50590690d22f4977868f`);
the isolated fixed full-library report passes 14/14 at
`.evidence/monstertruck-qualification/boolean-fix/profiles/converter-clean-full-lib-tests.log`
(SHA-256 `9e5dd294e135e8781bba2df97fbefd886ac8592132a1cb3117962da76a417522`).
The final integrated follow-up checks are recorded in
`.evidence/monstertruck-qualification/vendor-integration/converter-candidate/`.
The diagnostic context and remaining Boolean failures are recorded in
`.evidence/monstertruck-qualification/boolean-fix/endpoint-priority-diagnostic.md`.

## SSI topology-aware closedness candidate

The qualification-only patch
`.evidence/monstertruck-qualification/boolean-fix/vendor-topology-closedness-v2.patch`
(SHA-256 `a736aa2b8cfac9a0e10b5c0c09d2c65a192f34706b7e0a1964f4f009fc5860f2`)
adds a `closed` bit to each polyline/SSI-curve pair. The classic intersector
derives it from exact equality of the emitted polyline's first and last
coordinates; SSI endpoint refinement and loop storage then use the bit rather
than independently inferring closure from distance tolerance. The patch adds
four focused intersection-refinement tests and a legacy graph-walk closed/open
control. The
legacy `PointIndex` still buckets endpoints before the walk: distinct physical
endpoints that collide in one bucket can be merged before this marker exists.
The corrected legacy control has a 2.000001e-7 mm gap across adjacent cells;
same-cell preservation is not tested or claimed.

The patch was applied to the current endpoint-priority plus short-edge
converter candidate. The changed files use the upstream `rustfmt.toml` at
`.evidence/monstertruck-feature-assessment/upstream/rustfmt.toml`, with
`rustfmt 1.10.0-nightly (e457a7b0d3 2026-08-27)` and edition 2024. The
format-only delta is limited to newly added patch lines and recorded at
`.evidence/monstertruck-qualification/vendor-integration/closedness-candidate/revision-v2/format-only.diff`.
The frozen prior integration remains in `revision-v1/`; the revision-v2
inventories and formatting check are under `revision-v2/`.

The full solid test command passed 19 unit tests, 4 Boolean smoke tests and 1
doctest; workspace Clippy with `-D warnings`, the native `boolean_gate` build,
the wasm32 library build, and focused formatting checks also passed. The
three changed solid files pass the upstream nightly rustfmt check. The Boolean
gate still fails its regression difference with `EmptyOutputShell` after the box
control and profile-hole extrusion pass. This does not qualify full Boolean
behavior or production parity.

## Shared edge split identity

The independently reviewed general-splits-v2 patch replaces every occurrence of a split edge in a wire and reuses child identities across operand stores. Cached cuts retain parent allocation ownership and are keyed by parent edge and split vertex; fresh absolute-parent parameters must match exactly. Geometry cache hits recheck the current opposite surface. Branch/projection failures propagate before cached wire or vertex mutation. Retraced-curve and incompatible-surface controls are included. Provenance: `.evidence/monstertruck-qualification/astra-boolean-topology/general-splits-v2/`. This contains no analytic recovery experiment and does not qualify general Boolean closure or coplanar trim handling.

## Kernel-v22 conservative face bounds

`src/transversal/classic/loops_store.rs` caches finite face bounds with the existing BoundingBox helper and skips only strictly separated pairs. Touching/unknown/malformed-index bounds preserve the narrow path. Reviewed base patch SHA-256 `ba23750bd8b5441dd7d1bcc9c4cc6240f1649df5831ee9f66878975ab46dcc5f`, v2 delta `bcb41be321e22dce886c336d4166f5f51037a170c0af1aa7399a12fd1ad05a46`; evidence: `.evidence/monstertruck-qualification/dish-collision-broadphase-v{1,2}/`. This does not establish full Boolean correctness or controlled performance.

Dish-v28 applies reviewed shared intersection-event/endpoint handling and geometric contact ownership for loop partitioning. Source-v4 evidence: `.evidence/monstertruck-qualification/dish-shared-events-v1/source-checkpoint-v4/`. Valid tapered public cut and DSA material/topology checks pass; general IntersectionCurve cut endpoint semantics remain a separate unresolved issue.

## Boolean-round-v45 bounded prerequisites

The independently Spec/Standards-reviewed production006 v1/v2 patches modify
`classic/mod.rs`, `classic/faces_classification.rs` and
`polyline_construction/{mod.rs,tests.rs}`. Whole-material classification includes
oriented inner boundaries, closed unknown components remain available for global
classification, and each surface-pair interference graph removes exact undirected
duplicate segments without tolerance-merging distinct geometry. Source/test
preimages, reviewed postimages and subsequent notice-only changes are retained at
`.evidence/monstertruck-qualification/vendor-integration/boolean-round-v45/`;
the original candidates remain at `.evidence/production-case-boolean-006/v{1,2}/`.
Fresh shared46 native tests and WASM compilation pass. Contact/conical successors,
valid-empty handling and general curved containment remain separate; this is not
full production006 or keycap Boolean acceptance.
