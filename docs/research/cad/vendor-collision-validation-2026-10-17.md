# Vendor STEP collision validation research

Date: 2026-10-17. Proposal only; Parry 0.31.1 was latest on inspection date. No ticket or product contract changed. [P1]

Subsequent user decision,2026-10-17: approved supported tolerance bands by default, on-demand refinement and0.01 mm as an explicit tighter check. Unsupported accuracy remains uncertain. The recommendation below is now adopted in [014](../../plans/cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md); this does not certify any existing mesh or change source-completeness requirements.

## Recommendation

Keep existing product-preview tessellation settings. Report each occurrence's error band from authenticated source precision, mesh approximation, bounded repair displacement, and validated numeric error. Sum the per-part bounds for a pair; if any is missing, the band is unknown. Parry supplies no error certificate.

Against requested clearance `c`, use mesh-surface separation `d` and pair band `B`: report **collision / clearance violation** if an eligible pair has `d + B < c` or a robust solid-level witness proves overlap; **clear** only if `d - B >= c` and a separate containment check rules out enclosure; otherwise **uncertain**. `Contact.dist` is one local signed witness, not a global gap or penetration certificate. Intersection/contact within the band, unknown bounds, ineligible solids, unsupported queries, or invalid/non-finite data stay uncertain unless a robust witness proves violation. The approved 0.01 mm target stands pending user decision. Proposal: report measured bands without forced remeshing by default; retain 0.01 mm as an explicit tighter request, returning uncertain when unsupported.

## Parry capability and limits

The `parry3d-f64` 0.31.1 crate is the 3D f64 Rust variant. Its default features include `dim3` and `f64`; the package is Rust-only in the published manifest and is categorized for WASM, so a pure-Rust wasm32 route looks plausible, but this research did not build it. [P1, P2]

`TriMesh` is a composite of triangle subshapes; the default dispatcher recurses through composite pairs for `intersection_test`, `distance`, `contact`, and `closest_points`. Intersection is boolean; distance is nonnegative and zero for touching or penetration. In 0.31.1, public `Contact` has `subshape1/2`; composite contact fills these from selected part IDs, and a `TriMesh` part ID is its triangle index. Map those IDs through occurrence triangle-to-source-face provenance. `contact` gives one witness, not a complete contact set or global penetration certificate. `closest_points` returns separated locations but drops the selected part ID, so that path needs explicit BVH/triangle attribution. [P3–P6]

These queries test triangle surfaces, not solid volume. Nested closed shells can have separated surfaces without crossing, so surface distance/intersection alone cannot call them clear. `HALF_EDGE_TOPOLOGY` checks mesh adjacency but does not prove a closed, outward solid; `ORIENTED` explicitly assumes outward orientation. Require a separate solid/containment eligibility check for a global clear result. [P4, P7]

Do not enable merge/delete flags for collision preparation: those can merge vertices or omit triangles and conflict with retaining source faces and provenance. Validate finite transformed vertices and in-range indices before constructing `TriMesh`; 0.31.1's BVH construction indexes the vertex buffer directly. Reject non-finite query outputs. Treat unsupported/failed queries and any result whose numeric band is unbounded as uncertain. [P7, P8]

## Existing blockers independent of 0.01 mm

The v8 handoff's two positive previews use 0.1 mm / 0.5 rad tessellation, report `analysis_error_bound_mm=None`, and `mesh_closed_oriented=false`; they therefore cannot yet produce a bounded clearance or solid-clear result at any requested threshold. It also records a remaining selected-PCURVE regression losing 12 edges / 24 uses, an unsupported THQ wireframe representation, and no corpus-wide qualification. These block collision qualification independently of whether 0.01 mm remains a target; source-complete import and retention requirements remain in ticket 007. [L1, L2]

## Sources

- P1: [Parry f64 crate 0.31.1](https://docs.rs/crate/parry3d-f64/0.31.1) (release date, precision, package metadata); [features](https://docs.rs/crate/parry3d-f64/0.31.1/features).
- P2: [Parry 0.31.1 Cargo manifest](https://docs.rs/crate/parry3d-f64/0.31.1/source/Cargo.toml) (features and Rust dependencies).
- P3: [query module](https://docs.rs/parry3d-f64/0.31.1/parry3d_f64/query/index.html); [default dispatcher source](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/query/default_query_dispatcher.rs).
- P4: [TriMesh API](https://docs.rs/parry3d-f64/0.31.1/parry3d_f64/shape/struct.TriMesh.html); [composite intersection source](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/query/intersection_test/intersection_test_composite_shape_shape.rs); [distance source](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/query/distance/distance.rs).
- P5: [Contact API](https://docs.rs/parry3d-f64/0.31.1/parry3d_f64/query/contact/struct.Contact.html); [Contact struct fields](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/query/contact/contact.rs), [composite contact attribution](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/query/contact/contact_composite_shape_shape.rs), and [TriMesh part IDs](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/shape/trimesh.rs).
- P6: [closest-points API](https://docs.rs/parry3d-f64/0.31.1/parry3d_f64/query/closest_points/enum.ClosestPoints.html); [composite closest-points source](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/query/closest_points/closest_points_composite_shape_shape.rs).
- P7: [TriMesh flags and builder source](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/shape/trimesh.rs).
- P8: [composite distance source](https://docs.rs/crate/parry3d-f64/0.31.1/source/src/query/distance/distance_composite_shape_shape.rs).
- L1: Production STEP 007 v8 handoff (`../../../.evidence/production-step-007/review-v8/HANDOFF.md`), local private evidence.
- L2: [ticket 007](../../plans/cad-backend-replacement/issues/007-strict-step-import.md).

Research was limited to primary docs/source and existing local evidence; no build, corpus run, new target, or application change was made.
