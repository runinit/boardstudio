# Tight bounds for retained Monstertruck STEP geometry

Date: 2026-10-17. Research/design only; production STEP module ownership remains with ticket 007. Sources below are the local patched Monstertruck 0.4.1 source selected by the production manifest, not claims about an unpatched release. The manifest pins the versions and selects the vendor directories. [S1]

## Recommendation

Compute bounds in the STEP IO representation. The smallest defensible first implementation is **finite native edge extrema for compact planar, cylindrical and conical faces**, with an explicit refusal for a face/edge whose enclosure cannot be proved. This fixes the rotated-cylinder regression without changing tessellation, converting the source solid to modeling geometry, or losing the retained source. This is a partial implementation route, not evidence that every currently meshed STEP family has tight bounds. The retained shell exposes the exact converted surfaces, edges and face bindings; its closure certificate explicitly does not certify trim coverage. [S2, S3, S4]

For the other families, require either analytic interior extrema plus certified trim membership, or a convergent enclosure with an explicit gap certificate. No inspected public API supplies a complete tight, trim-aware IO face AABB. Existing public modeling bounds provide useful enclosure arguments, but intentionally use whole spheres/tori and unsplit control hulls. [S5, S6, S7]

## Actual contract and failure

- The current Cadrum import seam reports millimetre bounds from `Solid::bounding_box()` before meshing. The seam describes these as kernel bounds. [S8]
- Existing source tests distinguish contracts: VIK mesh bounds are within 0.10001 mm of independent tight bounds; 0.03 mm there also appears as containment tolerance. THQ checks mesh extrema within 0.03 mm and requires imported bounds to contain the mesh. A separate baseline consistency test uses 0.01 mm. There is no single universal 0.03 mm tolerance for every check. [S9, S10, S11]
- Ticket 007's new public rotated-cylinder regression explicitly requires each imported bound within 0.03 mm of `[-10,-10,0]..[10,10,10]`; its recorded RED is `9.96405982346029 != 10`. The tetrahedron/placement helper separately requires error below 1e-9. [S4, S12]
- At inspection, production bounds were accumulated from accepted f64 triangle vertices, while tessellation requested 0.1 mm and `Some(0.5)` radians. Exact source solids, per-solid unit scale, occurrence transforms and original bytes were already retained. A 0.1 mm mesh error contract does not imply a 0.03 mm extrema contract; the RED is a direct counterexample. [S3, S4, S12]

Use a numerical source-bounds budget below the **strictest applicable fixture threshold**, e.g. 0.001 mm for a proposed bounded fallback, and retain exact analytic results for analytic cases. This budget is a design recommendation, not an existing product setting or an established proof of the fallback. [S9–S12]

## Small analytic implementation seam

Keep a private `step/bounds.rs` helper consuming `&SourceBoundStepShell`, the authenticated `mm_per_unit`, and the occurrence matrix. Call it for each retained shell in the existing occurrence loop, and union its result independently of mesh accumulation. No public API or member visibility needs to change: `shell()` already exposes the immutable compressed shell. Keep the existing mesh/completeness/volume/coordinate checks and original-byte carrier. [S2, S3]

Apply the complete affine transform **before computing world-axis extrema**: occurrence × unit scale × curve placement. Transforming just the two corners of a local AABB under rotation is insufficient; transforming all eight corners is safe but generally loose. This follows directly from the affine transformed-circle formula below, and the production occurrence loop already has the independent unit scale and placement. [S3, S13]

### Why boundary extrema suffice for these surfaces

**Derived proof, not a sampled observation:** a straight-profile revolution has `S(u,v) = origin + rotation(v) * (p + u*d - origin)`. Fixing the angular parameter leaves a straight ruling, and any world coordinate after an affine transform is affine in `u`. Intersect a complete compact trimmed face with that ruling. Every nonempty connected intersection is a finite closed interval (or a point); a coordinate's maximum and minimum occur at interval endpoints. These endpoints lie on the geometric face boundary, except for a coordinate constant along the ruling, which has the same value at those endpoints. Unioning over all rulings therefore gives the face's coordinate extrema on its boundary. The loaded cylinder/cone are precisely revolutions of lines; the processor's inversion only swaps parameter axes/orientation. Planar faces satisfy the same argument using any fixed straight direction in the plane. [S13, S14, S15]

The proof requires **actual compact, complete trimmed faces**, with all boundary edges/vertices retained, including degenerate apex endpoints. An untrimmed unbounded plane/cylinder/cone, missing wires, or a ruling with no finite boundary does not satisfy it and must refuse this path. Holes and disconnected ruling intervals do not invalidate the proof; each interval has its own endpoints. `[0,1]` ranges on planes and cylinder/cone axial directions are placeholders, not evidence of compactness. The retained closure flags certify boundary equality, not compact trim coverage. [S2, S15, S16]

This proof also applies to linear extrusion, whose source family exists in IO, if its finite trim and boundary conditions can be established. It does not apply to a general curved-profile revolution merely because that surface is called a revolution. [S13, S17]

### Native curve dispatch

The IO enum already retains line, polyline, conic, B-spline, NURBS, parameter, surface and intersection curves. Its conic is a `Processor<TrimmedCurve<Unit…>, Matrix4>` with public entity, transform, orientation and finite trim range accessors. [S17, S18]

| Curve | Minimum sound tight method |
|---|---|
| Line / polyline | Union transformed endpoints / vertices. These are the stored geometric carriers. [S17] |
| Ellipse / circle | For each world axis write `f(t)=c+a*cos(t)+b*sin(t)` using the composed affine matrix's translation and first two columns. Include finite trim endpoints and every `atan2(b,a)+k*pi` in the retained interval. A full turn can use `c±hypot(a,b)` directly. When evaluating via the processor, map the candidate parameter through its orientation reversal; alternatively evaluate the raw unit curve and composed matrix consistently. Do not normalize an arc into a full circle. This is the derivative-zero solution of the actual unit-circle evaluator. [S18, S19] |
| Parabola | The source evaluator is `(t²,2t,0)`: for `f=c+a*t²+2*b*t`, include endpoints and `t=-b/a` when finite/in range. [S20] |
| Hyperbola | The evaluator is `(cosh(t),sinh(t),0)`: for `f=c+a*cosh(t)+b*sinh(t)`, include endpoints and `atanh(-b/a)` only when its ratio has magnitude below one and the result is finite/in range. Overflow or an invalid finite interval must refuse. [S21] |
| B-spline / positive-weight NURBS | A control hull encloses but need not approximate extrema tightly. Subdivide exact homogeneous Bezier pieces and compare projected-control outer extrema against actual endpoint/midpoint inner witnesses until their six extrema gaps meet the source-bounds budget. Polynomial curves use weight one. The positivity check is mandatory. The source trim code already validates positive weights and exact partial interval cuts. [S22, S23, S24] |
| IO `SurfaceCurve3D` | Its evaluation and finite range delegate to the retained 3D leader; recurse on that leader under the same accepted-source contract. Keep associated source trim data intact. [S25] |
| `ParameterCurve` / `IntersectionCurve` | Do not silently replace by a sampled/fitted leader. Bound the composition or intersection structurally; current modeling helpers demonstrate safe but potentially loose enclosing strategies. Tight support needs further subdivision and gap proof. [S6, S7, S26] |

For spline subdivision, the useful reuse is `control_points`, `knot_vector`, homogeneous representation and Bezier mathematics, **not unrestricted repeated calls to generic `Cut`**. Both curve and surface cuts snap parameters near knots; repeated midpoint cuts can cease shrinking in parameter space while geometry still exceeds a millimetre budget. The STEP partial-trim implementation contains a private exact-knot-insertion split explicitly because of this issue. A small private de Casteljau split on already extracted Bezier controls avoids the snapping rule and does not require widening that helper's visibility. Existing `bezier_decomposition` itself invokes `Cut`, so validate its suitability for narrow knot spans or extract by exact insertion. [S23, S24, S27]

**Derived stopping certificate:** for each coordinate, if `L ≤ true_min ≤ sampled_min ≤ sampled_max ≤ true_max ≤ U`, returning `[L,U]` has extrema error at most the two outer/inner gaps. For a union of patches/curves, union all outer boxes and all actual witnesses first, then test the six global gaps. Bound the iteration count and refuse non-progress/nonfinite arithmetic; exhaustion is not acceptance. This is stronger than comparing successive sampled grids, which can both miss the same extremum. Hull validity depends on the positive-weight and valid active-domain assumptions implemented by the cited source validation. [S22–S24]

For a formal floating-point certificate, knot insertion/projection/trigonometry also require outward rounding or a demonstrated roundoff enclosure below the allocated budget; ordinary f64 control-hull mathematics alone is not such a certificate. Keep coordinate scale and occurrence translation in that accounting. The current importer already rejects nonfinite/range failures and a mesh whose f32 projection destroys a triangle. [S3, S22–S24]

## Remaining families: exact missing obligations

| Face family | Necessary interior treatment and present gap |
|---|---|
| Sphere | Add the two support points for each world-axis linear functional on the placed sphere, **only if the points belong to the finite trimmed face**. Boundary-only misses a spherical cap's interior pole; using the whole sphere overbounds a cap which excludes other support points. STEP sphere swaps the underlying geometry's parameter axes and uses latitude `[-pi/2,pi/2]`; preserve that mapping, seam and pole cases. [S5, S28] |
| Ring/horn torus | Solve both coordinate partial derivatives for all stationary candidates, include singular/degenerate cases, and certify each candidate's finite trim membership. A full torus bound is a safe outer bound, not a tight trimmed face result. The evaluator supplies `(R+r*cos(v))*(cos(u),sin(u)), r*sin(v)`; positive radii are available, while spindle regimes are explicitly refused by IO. [S5, S29, S30] |
| B-spline / NURBS surface | Use native active-knot patches, positive homogeneous weights, exact subdivision, source-domain clipping, and outer/inner extrema gaps. Whole control hulls alone are loose. Surface accessors expose control nets and active domains, and IO marks only B-spline/NURBS as supporting exact patch domains. This avoids changing trig parameterization by conversion. [S5, S22, S31, S32] |
| General swept surface | Needs a proof specific to the profile or certified patch subdivision. A curved profile does not inherit the straight-ruling proof. The IO enum retains extrusion and revolution separately. [S13, S17] |

The missing common primitive is **certified finite trim membership**, not merely surface projection. The available parameter-boundary routine explicitly has a projected-sampling fallback. `SurfaceCurve3D::parameter_curve_on` can compare basis surfaces/leader consistency using finite samples. The tessellator's `PolyBoundary::include` uses polygon segments and is private to triangulation; neither gives a proof for all points on an exact curved trim. Closure flags do not prove periodicity or domain coverage. Do not expose these helpers or relabel their results as exact membership to finish bounds. [S2, S16, S33, S34]

A viable bounded extension is to partition a true source UV domain, bound each cell's world image by native homogeneous/control/analytic intervals, and exclude cells only after certified trim separation/winding. For cells touching a trim, refine until their world boxes are within the extrema gap budget of certified in-face/boundary witnesses. Use retained exact UV curves where authenticated; absent exact UV curves require certified inversion or a typed refusal. Periodic charts, holes, poles and closed seams need coverage certificates. This is a proposed algorithm with missing implementation/proof obligations, not an existing reusable API. The existing source code's distinction between exact patch domains and placeholder/sample-derived domains is why it is necessary. [S16, S31–S34]

Safe opportunistic shortcut: an already sound whole-support/control-hull **outer** box may be accepted if independently certified on-face witnesses close all six gaps below the required budget. It remains a bounded source result, not an exact result. If gaps fail, returning the loose outer box as production tight bounds is unsupported. Do not use triangle vertices as certified witnesses without proving their source-face membership and accounting for projection/snapping error. [S3, S5, S33, S34]

## Focused verification seam

1. Reuse `tests/step_import.rs::curved_geometry_bounds_are_independent_of_the_product_mesh_grid` and its recorded RED; keep 0.1 mm / 0.5 rad product settings. The exact radius and frame rotation are independent of kernel output. [S3, S4, S12]
2. Add a partial ellipse/cylinder sector with a critical angle excluded by the trim; this rejects a full-circle shortcut. Add an included between-samples critical angle and reversed edge orientation. Expected values follow the formula above, not a round trip. [S18, S19]
3. Use one rotated/translated/unit-scaled occurrence whose tight world bounds differ from transformed local extrema; keep existing mixed-unit/placement expectations. [S3, S12]
4. For any spline fallback, use a quadratic Bezier with known interior peak and a narrow knot-domain variant; assert containment and the actual gap certificate. Mutating away subdivision or the positive-weight guard must be detected. [S22–S24, S27]
5. Before claiming curved-face coverage, add a spherical cap which includes a pole, a cap which excludes a whole-sphere support point, and a NURBS patch with an interior bulge. These discriminate boundary-only and whole-support shortcuts. Include an unsupported trim case which refuses, rather than returning a proxy. [S5, S28, S31–S34]

Research verification: source inspection, comparison confirming that all 23 cited vendor files match the private 007 vendor copies, `git diff --check`, and reuse of the already recorded RED only. No new build, corpus run, production edit, public API change, tracker, or harness was performed. Tight bounds for all accepted source families remain an acceptance obligation; the analytic ruled-face slice alone cannot close ticket 007. [S35]

## Primary source index

All paths are relative to the repository root. `V` below means `cad/monstertruck-qualification/vendor/`; `P` means `.evidence/production-step-007/crate/`. The latter is current private owner evidence and is not a committed production API.

- S1: `cad/monstertruck/Cargo.toml:12–30` (0.4.1 pins and path patches).
- S2: `V/monstertruck-io/src/step/load/convert/mod.rs:18–84` (`StepCompressedShell`, source carrier/accessors, closure caveat).
- S3: `P/src/step/mod.rs:16–48,80–128,159–207` (retained state, product options, mesh bounds and checks; line positions are the inspected pre-bounds implementation).
- S4: `P/tests/step_import.rs`, symbol `curved_geometry_bounds_are_independent_of_the_product_mesh_grid`, inspected lines `213–229` (owner may add preceding tests).
- S5: `V/monstertruck-modeling/src/bounding/mod.rs:124–197` (public surface/face bounds and whole-support/control-hull cases).
- S6: `V/monstertruck-modeling/src/bounding/mod.rs:273–334` (structural parameter/intersection curve hulls, not leader fitting).
- S7: `V/monstertruck-modeling/src/bounding/mod.rs:337–390` (UV hull and restricted patch support).
- S8: `cad/wasm/src/model/kernel.rs:16–24,49–75` (current import seam and kernel bounds).
- S9: `cad/wasm/src/model/source_validation_tests.rs:100–138` (VIK containment and tight mesh tolerances).
- S10: `cad/wasm/src/model/source_validation_tests.rs:309–328` (THQ tight mesh bounds/import containment).
- S11: `cad/wasm/src/model/source_validation_tests.rs:472–485` (0.01 mm baseline comparison).
- S12: `.evidence/production-step-007/logs/geometry-bounds-red.log:20–30`; `P/tests/step_import.rs:5–12`, symbols `mixed_child_metre_parent_mm_scales_geometry_without_rescaling_parent_translation`, `shared_exact_solid_keeps_both_rotated_and_translated_occurrences` (existing tight helper and unit/occurrence tests).
- S13: `V/monstertruck-geometry/src/decorators/revolved_curve.rs:70–124` (revolution evaluator/domain).
- S14: `V/monstertruck-io/src/step/load/step_types/surface.rs:203–215,422–441` (line-profile cylinder/cone construction).
- S15: `V/monstertruck-geometry/src/decorators/processor/mod.rs:318–355` (surface parameter axes/orientation); `V/monstertruck-modeling/src/bounding/mod.rs:25–83` (existing enclosure assumptions).
- S16: `V/monstertruck-io/src/step/load/step_geometry/geom_impls/parameter_trims.rs:123–139`; `V/monstertruck-io/src/step/load/convert/mod.rs:55–57` (placeholder ranges/closure limitation).
- S17: `V/monstertruck-io/src/step/load/step_geometry/mod.rs:16–36,253–318,338–375` (concrete native IO curve/surface types).
- S18: `V/monstertruck-geometry/src/decorators/trimmed_curve.rs:4–44`; `V/monstertruck-geometry/src/decorators/processor/mod.rs:26–36,92–104,147–184` (finite range, transform, orientation semantics).
- S19: `V/monstertruck-geometry/src/specifieds/circle.rs:34–55` (unit-circle evaluator and derivatives).
- S20: `V/monstertruck-geometry/src/specifieds/parabola.rs:29–45` (3D parabola evaluator and derivatives).
- S21: `V/monstertruck-geometry/src/specifieds/hyperbola.rs:27–42` (3D hyperbola evaluator and derivatives).
- S22: `V/monstertruck-modeling/src/bounding/mod.rs:144–156,482–494`; `V/monstertruck-geometry/src/nurbs/bspline_curve.rs:471–506` (hull/positive weight and active knot domain).
- S23: `V/monstertruck-geometry/src/nurbs/bspline_curve.rs:963–995,1130–1185`; `V/monstertruck-geometry/src/nurbs/nurbs_curve.rs:15–37,522–529` (Bezier access, subdivision and projected control box).
- S24: `V/monstertruck-io/src/step/load/step_types/topology/bounded_trim.rs:9–75,151–213` (finite/positive validation, exact private split).
- S25: `V/monstertruck-io/src/step/load/step_geometry/geom_impls/curve_traits.rs:6–38` (IO surface-curve evaluation delegates to its leader).
- S26: `V/monstertruck-geometry/src/decorators/intersection_curve/mod.rs:32–83,96–104` (intersection Newton projection and leader distinction).
- S27: `V/monstertruck-geometry/src/nurbs/bspline_surface/split.rs:44–99,141–146` (surface split and near-knot snapping).
- S28: `V/monstertruck-io/src/step/load/step_geometry/sphere.rs:4–38,58–69` (STEP sphere frame, finite latitude, poles).
- S29: `V/monstertruck-geometry/src/specifieds/torus.rs:20–28,60–96` (torus radii, evaluator and derivatives).
- S30: `V/monstertruck-io/src/step/load/step_types/surface.rs:287–329` (spindle/invalid-radius refusal).
- S31: `V/monstertruck-geometry/src/nurbs/bspline_surface/accessors.rs:99–129,297–322,450–460` (net, active domain, hull).
- S32: `V/monstertruck-io/src/step/load/step_geometry/geom_impls/bspline_nets.rs:87–155`; `V/monstertruck-geometry/src/bspline_conversion/mod.rs:9–27,95–133` (exact net conversion, frame flags and exact domain support).
- S33: `V/monstertruck-io/src/step/load/step_geometry/geom_impls/parameter_trims.rs:1–14,413–440`; `V/monstertruck-io/src/step/load/step_geometry/mod.rs:109–145,180–193` (sampled fallback and finite-sample trim matching).
- S34: `V/monstertruck-meshing/src/tessellation/triangulation/boundary/loops.rs:830–890` (private polygon-domain containment).
- S35: `docs/plans/cad-backend-replacement/issues/007-strict-step-import.md:10–20` (full production acceptance scope).
