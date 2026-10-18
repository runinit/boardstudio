# Collision qualification: finite implementation plan

Date: 2026-10-18. Inspected revision: `797ab74e31f608a630c6983216842c7ea9b829e8`.
This is a research/specification checkpoint for [007](../../plans/cad-backend-replacement/issues/007-strict-step-import.md)
and [014](../../plans/cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md),
under the [accepted scope](../../plans/cad-backend-replacement/map.md).
It changes no production code, contract, ticket status or acceptance criterion.
The derivations below are proposed proof obligations, not an installed certificate.

## Recommendation and first deliverable

Qualify **complete planar polygonal vendor assets with native LINE boundaries**
first, starting with the original bundled `SMD_0805_Capacitor.step`. This gives
a real vendor positive pair without solving arbitrary spline approximation.
Qualify generated planes, cylinders and cones against their **actual emitted
triangles**, using retained UV/domain correspondence and the bounds below.
Keep Parry's existing raw observations and independently enclose their mesh
distance using interval filters and exact rational triangle predicates. Do not
attempt to prove a global accuracy guarantee for GJK by selecting an epsilon.

The first coherent implementation checkpoint is an internal, wire-free planar
qualifier plus the capacitor positive/negative controls in the existing importer
tests. A second internal checkpoint is a checked triangle-pair distance enclosure
with exact analytic controls. Parent agreement on the existing authoritative
contract and shared fixture must precede production producer/consumer changes.
No new general geometry kernel, parallel harness or mesher replacement is needed.

## Current behavior and evidence boundary

- [Importer](../../../cad/monstertruck/src/step/mod.rs), `import_step_with_options`:
  retains original bytes/AST, definition solids, every admitted occurrence,
  source face catalogue, per-occurrence tags and f64 mesh positions. Its
  `analysis_error_bound_mm` and `max_repair_displacement_mm` are `None`.
  The internal per-face `repair_displacement_mm: 0.0` is a placeholder;
  [vendor binding](../../../cad/wasm/src/model/vendor_step.rs),
  `face_occurrence_dto`, correctly exports **null**, not this placeholder.
- [Generated mesh](../../../cad/monstertruck/src/profile.rs),
  `mesh_profile_for_analysis`: validates strict shell tessellation, then calls
  `triangulation_with` and emits that second result's face-tagged f64 triangles.
  Qualify that emitted result; a successful first tessellation does not certify
  the second. It currently drops UVs when assembling output and returns a null
  bound. Its `REVOLUTION_SURFACE` string does not identify cylinder versus cone.
- [Generated topology](../../../cad/monstertruck/src/layered/topology.rs),
  `surface_key`: walls are planes or translated revolutions of a **line** about Z;
  the generator uses lower/upper authored radii. Equal radii give cylinders,
  unequal radii give cones. Face merging, authored carriers and material senses
  must survive qualification. [mesh.rs](../../../cad/monstertruck/src/mesh.rs),
  `closed_oriented_edges`, only counts oriented edge incidences.
- [Raw adapter](../../../cad/wasm/src/model/collision.rs),
  `TriangleSoupMesh::from_transformed_soup` and `observe_mesh_pair`: transforms
  f64 vertices once, constructs `TriMeshFlags::empty()`, then runs intersection,
  contact or distance/closest-points queries. `MAX_SAFE_COORD_MM` guards overflow;
  `CLOSEST_POINTS_SLACK_MM` checks consistency. Neither is an error certificate.
- [Runtime](../../../web/crates/runtime/src/model_delivery.rs),
  `assess_subject_collision_observation`: accepts optional numerical bound and a
  `containment_ruled_out` boolean, sums three scalar bounds, and otherwise stays
  unknown. It has no independently certified contact region. Its scalar sum is
  not yet adjusted for a non-isometric final row transform. Input validation
  permits nonsingular affine maps, not only rigid placements.
- The single [contract](../../../cad/wasm/src/model/generated/ANALYSIS_CONTRACT.md)
  and [fixture](../../../cad/wasm/tests/fixtures/generated-analysis-contract.json)
  already distinguish generated identity from vendor SHA, deflection from
  accuracy, raw surfaces from source conclusions and unknown movement from zero.
  Preserve that architecture. The current full native/browser evidence and
  remaining gaps are in tickets 010/014; no build, benchmark, suite or provider
  replay was run for this note.

## 1. Geometry movement and a useful vendor class

### Actual stages, rather than a presumed generic healing pass

The current importer does **not** call a generic solid-healing pipeline.
Its source conversion, normalization and tessellation have the following effects.
The vendored source linked here is the actual patched 0.4.1 code selected by
[cad/wasm/Cargo.toml](../../../cad/wasm/Cargo.toml), not an upstream marketing claim.

1. [Native edge parsing](../../../cad/monstertruck-qualification/vendor/monstertruck-io/src/step/load/step_types/topology.rs),
   `parse_curve3d_with_connectivity` / `sub_parse_curve3d`: contextual LINEs
   project topological endpoints onto the authored line. The non-contextual path
   constructs `Line(p,q)`; preview allows a native direction not to be retained
   bit for bit. These paths can change the selected boundary relative to source
   constraints. Circle/ellipse intervals come from nearest parameters; the
   near-equal endpoint rule can extend an interval by a full period. PCURVEs
   project endpoints onto their surfaces; SURFACE_CURVEs retain a selected master
   and resolve an active interval. Trim extension/restriction changes a **set**
   even when every retained point lies on its original support.
2. `try_normalized_edge_bspline` in that same file reparameterizes only a short,
   directly used, clamped, non-rational spline, then trims it. Knot normalization
   is set-preserving in real arithmetic under the stated full-domain conditions;
   computed knot/evaluation error still needs a bound. [bounded_trim.rs](../../../cad/monstertruck-qualification/vendor/monstertruck-io/src/step/load/step_types/topology/bounded_trim.rs)
   restricts active curves and uses exact-parameter insertion instead of the
   generic tolerance-snapping cut. Neither should be labelled zero movement
   merely because its intended operation is a reparameterization.
3. [Unit/angle normalization](../../../cad/monstertruck/src/step/units.rs),
   `normalize_conical_angles`, `normalize_shell_to_mm`, and
   [source-bound tessellation](../../../cad/monstertruck/src/step/tessellation.rs):
   conical angle-unit conversion and uniform length conversion implement the
   source meaning; occurrence placement is applied later. These are coordinate
   transformations, not a preview repair allowance. Their evaluation/rounding
   error is a separate contribution.
4. [Edge sampling](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/tessellation/triangulation/edge_sampling.rs)
   produces chords. Its interior angular/deflection tests are sampled. In
   [triangulation/mod.rs](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/tessellation/triangulation/mod.rs),
   exact/lifted parameter boundaries can be simplified, resampled or used in
   place of a shared edge representation. These affect derived boundary coverage;
   sample count or endpoint agreement does not bound the whole active curve.
5. [Boundary projection](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/tessellation/triangulation/boundary/projection.rs)
   projects/lifts UVs, pins seams and normalizes/clamps chart coordinates.
   [Loop assembly](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/tessellation/triangulation/boundary/loops.rs)
   changes repeated pole representatives and closes/compacts periodic loops.
   Several paths retain original XYZ while changing UV. That has zero immediate
   XYZ movement but can change which domain is triangulated. Recording only
   `distance(old_xyz,new_xyz)` would miss that failure mode.
6. [Triangle emission](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/tessellation/triangulation/mesh.rs)
   retains boundary-map XYZ, evaluates interior UVs, coalesces CDT handles,
   canonicalizes declared singular rows and filters triangles. Metric refinement
   flips unconstrained diagonals without moving source points. This changes
   interpolation error even when vertex movement is zero.
7. [Preview fallback](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/tessellation/triangulation/fallback.rs)
   merges nearby samples, cancels slits, bridges loops, triangulates by area,
   projects new refinement points and can finally emit a fan. In the importer,
   fallback triangles failing f32 rendering are removed, possibly leaving a
   catalogued face without triangles. The comment calling this preview geometry
   does not certify its distance. Any such face makes the first qualification
   class ineligible; preserve existing preview behavior and report unknown.

`SnapCurveEndpoints` definitions alone are not evidence of a production repair:
[curve_traits.rs](../../../cad/monstertruck-qualification/vendor/monstertruck-io/src/step/load/step_geometry/geom_impls/curve_traits.rs)
makes SURFACE_CURVE snapping a no-op and does not rewrite native lines/conics/
splines. Likewise `sampled_residual_mm` in
[bounds/support.rs](../../../cad/monstertruck/src/step/bounds/support.rs) measures
distance to an untrimmed support at samples, not movement or symmetric coverage.

### Instrumentation and accounting

Use a definition-space internal qualification record keyed by source edge/face
IDs, then bind it to each representation/solid occurrence. Keep original selected
master, active range, support and loop ownership available from the retained AST.
For each eligible conversion record the old/new active sets, correspondence,
and an outward enclosure `[movement_lower,movement_upper]`. For a line segment
with corresponding endpoints, the maximum paired displacement is bounded by
the maximum endpoint displacement: a difference of affine interpolants is a
convex combination of endpoint differences. Exact zero requires both endpoint
differences to be zero **and unchanged interval, connectivity and support**.
Changed ranges need a set-coverage bound too; unchanged support residual is
insufficient. Spline/PCURVE/general repaired surfaces stay unqualified initially.

For generated or vendor planar faces, compare full domain before/after, not only
edge events. A piecewise affine, topology-preserving map across a verified domain
triangulation bounds face movement by its maximum vertex displacement. Missing
strips/holes, overlapping patches or an unavailable map produce unknown. Log
conversion and meshing stages separately so intentional repair is distinguishable
from interpolation and floating-point evaluation. Do not force new diagnostics
into every mesher path merely to qualify a class that never uses those paths.

For the admitted class, `maxRepairDisplacementMm` can report an outward upper
enclosure of measured movement, with that meaning documented; per-face values
are maxima over their applicable events. A zero is justified by an identity
proof, not by an empty event list unless the eligibility audit proves every
reachable mutating branch was excluded. Reject over-budget repairs; if the
enclosure straddles the selected limit, refine it or refuse qualification.
Default allowance remains `max(2 * declared_precision,0.02 mm)`, or the explicitly
selected override. Never substitute either number for observed movement.

Choose one error decomposition in the contract:
`e_operand = e_source + e_repair + e_mesh + e_evaluation`, with each term charged
once. If a direct symmetric source-to-actual-mesh proof already includes repair,
retain the movement diagnostic but do not add it again. The vendored
[connectivity context](../../../cad/monstertruck-qualification/vendor/monstertruck-io/src/step/load/context/mod.rs)
explicitly treats declared precision as attachment uncertainty, independent of
tessellation or manufacturing variation. Retain and conservatively account for
that authenticated policy term; it does not prove an unknown mesh/repair term,
nor the dimensions of a manufactured part. State whether a reported interval
concerns the nominal source set plus that allowance. Missing declarations never
authorize assuming a repair floor was actually used as displacement.

### Named class and concrete positive pair

First class: finite plane faces bounded by complete simple native-LINE loops;
all source constraints consistent, or an explicitly bounded piecewise affine
repair correspondence; complete non-overlapping triangle coverage with holes;
no fallback, omitted face, selected PCURVE, spline, conic edge, singular chart or
ambiguous face ownership. Keep **all** solids/occurrences. Admission of a file is
not face cherry-picking: one unsupported face leaves the aggregate bound unknown.
Treat assembly transforms and unit scales using enclosed linear maps. Rigid or
signed-axis/unit transforms are the smallest first implementation subset.

Read-only raw Part21 reconnaissance at the inspected revision found:

| Bundled original | Solids / plane faces / LINE edges | Exact-decimal incidence check |
| --- | --- | --- |
| [0805 capacitor](../../../ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step) | 4 / 30 / 64 | All 128 line endpoint incidences and 256 face endpoint incidences exactly zero. |
| [0805 resistor](../../../ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Resistor.step) | 4 / 32 / 72 | Nonzero discrepancies; maxima about `5.56e-16 mm` for both checks. |
| [display cover](../../../ergogen/library/vendor/vik/3d_models/sadekbaroudi-fingerpunch/3d-models/display-housings/ili9341/ili9341-4inch-cover.step) | 1 / 33 / 80 | Nonzero discrepancies; maxima about `7.33e-14 mm` for lines and `6.67e-15 mm` for planes. |

These counts and incidence checks were computed directly from original records,
without the importer/mesher: parse each decimal as Python `Fraction`, follow
EDGE_CURVE → LINE/VECTOR and ADVANCED_FACE → bound/loop/oriented edge/vertex;
test squared line residual `|(p-o) cross d|^2/|d|^2` and squared plane residual
`((p-o) dot n)^2/|n|^2`. Every maximum was rationally checked `<= 1e-20 mm^2`;
the displayed square roots are diagnostics. This is a small source prerequisite
check, **not** proof of loop coverage, actual f64 movement, import success or a
complete mesh certificate. It created no harness or source copies.

Correction after the private regression: the capacitor's assembly transform
between `#552` and `#583` is **not identity**. Origins `#715`/`#850` are zero and
axes agree, but reference directions are `(1,0,0)` and
`(2.4980018054066e-16,-1,0)`. Native units are millimetres. The first qualifier
must independently reconstruct this bounded planar frame placement and its
normalization error; an identity-only source class excludes this positive.
Its full native vertex x range is `[-1,1]`; vertices `#226=(-1,-0.625,0)` and
`#217=(1,-0.625,0)` attain the extremes. Two complete instances with the second
translated by `(3,0,0)` in the **native part frame** therefore have nominal
source surface gap exactly 1 mm in that frame:
the disjoint x slabs prove a lower bound, those two vertices prove the upper
bound. This is an independently known positive **vendor/vendor** fixture to run
through the actual provider after implementation. Preserve all four solids and
30 faces on both sides. Record every occurrence, rather than extracting one box.
The [source manifest](../../../ergogen/library/manifest/sources.json) already
identifies this original asset; no new hash manifest is needed.

Its exact source incidences support a no-intentional-repair path: LINE projections
and plane restrictions are identities in real arithmetic. Still verify actual
conversion branches and enclose f64 evaluation separately before publishing zero
repair. The resistor/cover are useful **nonzero** controls, not zero-repair cases.
The cover is also a selected catalogue asset in
[asset-ledger.json](../../../catalogue/modules/asset-ledger.json); two complete
cover instances with translation `(111,0,0)` have the analogous nominal 1 mm gap
using source vertices `#256=(-55,-31.67,6.5)` and `#253=(55,-31.67,6.5)`.
Qualifying the capacitor is the first required coverage checkpoint; the cover
adds catalogue-population coverage after its small nonzero discrepancies are
independently bounded. None of these has a running Supported result today.

## 2. Symmetric generated source-to-actual-triangle bound

### Domain proof comes first

For each emitted face retain its original analytic support, oriented active
boundary loops, each emitted triangle's UV vertices and **actual f64 XYZ**.
Do this inside `mesh_profile_for_analysis` before UVs disappear; no wire UV array
is necessary. Vendor planar qualification can consume the equivalent normalized
face mesh before `append_face_triangles`. Domain verification must establish:

1. A bounded chart (or explicitly cut periodic charts) with the intended holes,
   connected components and source-edge ownership. Unwrap seams consistently;
   reject an unproved full-period jump or apex chart in the first implementation.
2. UV triangles tile the polygonal domain `D_h`: consistent positive orientation,
   exact shared edges, no triangle overlap or extra exterior triangle, no omitted
   cell, matching boundary chains and holes. A source-face tag/count alone does
   not prove this. Exact 2D predicates plus edge incidence and domain membership
   supply a finite check. Equal total areas alone can hide overlap plus a hole.
3. A symmetric boundary/domain bridge between exact trim domain `D` and `D_h`.
   Native line trims coincide after measured endpoint errors. For planar circular
   arcs with ordered samples and maximum subarc angle `alpha <= pi`, the arc/chord
   sagitta is `s=R*(1-cos(alpha/2)) <= R*alpha^2/8` in real arithmetic. Check
   directed trim coverage, shared endpoints and every nonadjacent feature.
   Certify noncrossing arc/chord strips and unchanged hole/component winding;
   these strips furnish a filled-domain correspondence with displacement at
   most their enclosure. A small boundary residual without these topology checks
   does not bound the filled domain. A vanished small hole is a counterexample.

For ordinary generated cylindrical/conical walls, trim boundaries in an unwrapped
`(theta,z)` chart are generators and horizontal arcs, hence straight chart edges.
Prove that fact from the retained **active curves**, including merged faces;
do not assume every revolution has rectangular trims. If the emitted chart
boundary differs, enclose `h_theta` and `h_z` for a topology-preserving bridge.
Unsupported trim shape keeps a null bound. This is post-meshing certification;
001/002's mesher behavior and separate acceptance remain unchanged.

Map the mesher's native UV coordinates to these variables explicitly. The actual
[revolved-curve implementation](../../../cad/monstertruck-qualification/vendor/monstertruck-geometry/src/decorators/revolved_curve.rs)
uses generator parameter **u** and rotation angle **v**; `theta=v` (with the
carrier's sense) and `z` is affine in `u` for a line generator. Enclose that map,
including reversed generators and Processor transforms. Treating native `u` as
an angle would invalidate both the bound and its seam/domain checks.

### Interpolation derivation

Let a chart triangle have vertices `u_i=(theta_i,z_i)`, barycentric weights
`lambda_i`, `u=sum(lambda_i*u_i)`, and actual emitted vertices `p_i`.
Let `delta=max_i |p_i-F(u_i)|`, **enclosed independently**, including evaluation
and off-support boundary error. Every point on the actual triangle is
`P=sum(lambda_i*p_i)`. The same barycentric map covers both the complete triangle
and its curved chart patch. Taylor expansion about `u` cancels the first-order
term because `sum(lambda_i*(u_i-u))=0`.

- Affine plane `F(u)=o+B*u`: interpolation error is at most `delta`.
- Cylinder `F(theta,z)=(R*cos(theta),R*sin(theta),z)`:
  `|F_thetatheta|=R`, other second derivatives vanish. The error is at most
  `delta + R*DeltaTheta^2/8`, since a distribution in an interval of width
  `DeltaTheta` has variance at most `DeltaTheta^2/4`.
- Cone/frustum `F(theta,z)=(r(z)*cos(theta),r(z)*sin(theta),z)`,
  `r(z)=r0+k*(z-z0)`, positive radius over the chart:
  `|F_thetatheta|<=r_max`, `|F_thetaz|=|k|`, `F_zz=0`.
  Taylor's integral remainder is bounded by
  `delta + r_max*DeltaTheta^2/8 + |k|*DeltaTheta*DeltaZ/4`.
  The mixed term follows from Cauchy–Schwarz on the two barycentric variances;
  omitting it is invalid for a slanted triangle on a cone.

These are symmetric **set** bounds on `F(D_h)` versus the actual triangles because
the barycentric domain is shared. They hold for unstructured triangles, not only
regular ring strips. They require continuous second derivatives on each chart;
an apex/zero-radius crossing must be split and separately proved or remain unknown.
The usual cylinder strip sagitta can tighten the bound when an actual ruled-strip
correspondence is verified; it cannot replace this proof for arbitrary triangles.

For the trim bridge, the cylinder/cone Jacobian columns are orthogonal with
lengths `r(z)` and `sqrt(1+k^2)`. Thus an additional bridge contribution is at most
`r_max*h_theta + sqrt(1+k^2)*h_z`; for planes use the norm of their actual affine
map. Charge planar arc/chord strips in physical millimetres directly. Take the
maximum complete-face bound across all faces, then all solids/occurrences; a
single unknown face prevents a global bound. This maximum, not a sum over faces,
is the symmetric Hausdorff error for their union.

### Frames and numerical contributions

For `T(x)=A*x+t`, `d_H(T(S),T(M)) <= ||A||_2*d_H(S,M)`.
Use a proved upper enclosure, e.g. `sqrt(||A||_1*||A||_infinity)`, for affine maps;
maximum column length is not sufficient for shear. Reflections require orientation
bookkeeping for containment. Enclose actual `A*p+t` rounding separately and apply
physical row placement exactly once. Import units/occurrence transforms are
already in vendor f64 analysis positions; do not apply them again.

Use outward basic arithmetic (`next_down`/`next_up` after each operation), with
finite-range guards and exact dyadic-rational fallback where needed. Compare the
actual emitted vertex to an independent interval enclosure of `F(u_i)`; neither
the mesher's own repeated evaluation nor a guessed ULP count is an oracle.
For vendor source literals, include decimal-to-f64 parsing and unit-factor
rounding: bound each original decimal by rational comparison with its floating
enclosure, using retained bytes/lexemes, before normalization. Starting from the
already rounded AST alone proves a bound for that AST, not automatically for the
original decimal source. Generated JSON f64 values are the accepted input values;
the claim here is their built analytic model versus emitted mesh, not an extra
claim about fabrication or the separate 008 export/authoring-accuracy gate.
Rust specifies correctly rounded [sqrt](https://doc.rust-lang.org/std/primitive.f64.html#method.sqrt)
and adjacent representable [next_up](https://doc.rust-lang.org/std/primitive.f64.html#method.next_up),
but gives [sin/cos unspecified precision](https://doc.rust-lang.org/std/primitive.f64.html#method.sin).
For circular/conical certificates use bounded-angle interval sine/cosine from
Taylor polynomials with explicit remainder `|x|^(n+1)/(n+1)!`, exact/outward
coefficients and proved range reduction; restrict the first charts to bounded
angles instead of adding an unbounded argument reducer. Include the error of
constants/ranges and arithmetic in `delta`. The
[NIST series](https://dlmf.nist.gov/4.19#E1) and
[derivatives](https://dlmf.nist.gov/4.20) supply the analytic functions/derivative
bounds; the finite enclosure and remainder arithmetic still need implementation
and independent rational checks. No platform libm error guarantee is assumed.

Dense samples remain a valuable **separate** regression oracle: cap edges,
interior patches, largest-angle cylinders, steep cones, narrow holes, reflections
and transformed seams. They may disprove a bound; passing them cannot prove it.

## 3. Numerical qualification of the actual query

The selected dependency is `parry3d-f64 = 0.31.1` in
[cad/wasm/Cargo.lock](../../../cad/wasm/Cargo.lock). The installed locked source
`parry3d-f64-0.31.1/src/query/default_query_dispatcher.rs` dispatches composite
meshes down to triangle support-map/GJK queries for distance and closest points.
Its `query/gjk/gjk.rs` uses absolute/relative termination tests and support-scale
logic, not an exported end-to-end roundoff certificate. See the matching
[versioned dispatcher](https://github.com/dimforge/parry/blob/v0.31.1/src/query/default_query_dispatcher.rs).
Keep raw Parry data, including selected triangles, contact normals/distances and
failure reasons. Do not reinterpret a local `Contact.dist` as global penetration.

### Smallest finite verifier using existing dependencies

There is already runtime outward-interval/exact-rational code in the vendored
[collision analyzer](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/analyzers/collision.rs):
`Interval`, `certified_miss`, `exact_collide_seg_triangle`, exact squared-distance
ordering and an inclusive sweep. `num-rational 0.4.2`, `num-bigint 0.4.8` and
`num-traits 0.2.19` are already locked; `num-rational`/`num-traits` are runtime
dependencies of that vendor crate. Their private implementation is not a public
general query API: its advertised collision operation deliberately excludes
coplanar contact, and its exact hit returns rounded points. Reuse the arithmetic
approach/dependencies, not `collide_with` as a complete certificate. No visibility
widening or vendored refactor is authorized by this note.

Implement a BoardStudio-internal checked verifier alongside the adapter, with
outward filters and exact rational fallback for ambiguous triangle features:

1. Interpret each **actual transformed f64 vertex** as its exact dyadic rational
   value (`BigRational::from_float`, not decimal approximation). Preserve original
   triangle indices/tags. Separately enclose transform error relative to the
   intended affine image, so the certificate does not start after losing it.
2. For each potentially closest triangle pair, exactly classify noncoplanar
   segment/triangle hits, endpoint hits, and coplanar overlap. In the coplanar
   case project along a provably nonzero normal component and use exact 2D
   segment/orientation/point-in-triangle tests. Degenerate triangles refuse.
3. On disjoint triangles, enumerate six vertex-to-opposite-face projection
   candidates whose rational barycentrics lie in the triangle, and nine
   segment/segment minima including constrained endpoints. For edges solve the
   2x2 quadratic system with exact dot products and determinant; parallel edges
   use endpoint/segment cases, not division by a tiny floating denominator.
   These exhaust the closest features of two compact disjoint triangles:
   face-interior/vertex or edge/edge, including parallel face minima whose region
   reaches a boundary feature. Exact intersections supply distance zero first.
4. Select the minimum exact squared distance across **all** triangle pairs. Use
   outward AABB distance lower bounds to reject only pairs whose lower bound is
   strictly above an already established upper bound. Parry can seed ordering
   and witnesses; its BVH rejection is not the independent completeness proof.
   Start with a bounded exhaustive scan for the small planar assets and analytic
   fixtures. A later BVH may accelerate the same predicate, without changing it.
5. Convert rational squared distance to an f64 interval by exact rational
   comparisons with candidate floating endpoints, then outward `sqrt`. This
   yields `[L,U]` containing the minimum distance of the actual triangles.
   Rational feature coordinates likewise produce outward witness boxes. Handle
   huge denominators/operation limits by explicit unknown, never approximate
   fallback disguised as certification. Put resource limits in algorithm identity.

This is finite triangle geometry, not NURBS or exact Boolean research. It needs
no new mathematical dependency; direct declarations of already locked crates
may be needed in the owning crate. The
[BigRational API](https://docs.rs/num-rational/0.4.2/num_rational/type.BigRational.html)
supports exact rational arithmetic. [Shewchuk's primary paper/code](https://www.cs.cmu.edu/~quake/robust.html)
explains robust determinant signs and arithmetic assumptions; exact signs alone
do not supply metric distance or contact-location bounds. Existing `robust 1.2.0`
is locked through Spade and offers orientation predicates, but does not replace
the metric construction. Avoid adding an interval library before this path is
tested against the existing rational dependency.

Given raw Parry distance `d_p`, an outward
`q=max(abs(d_p-L),abs(U-d_p))` bounds that observation's scalar error **for this
pair**. Prefer reporting `[L,U]` directly; then the source interval is
`[max(0,L-e_A-e_B), U+e_A+e_B]`. If retaining the current centered scalar seam,
use `d_p +/- (q+e_A+e_B)` with outward sums. Do not combine both formulations and
charge `q` twice. Raw intersecting gives `d_p=0` for this purpose, but if an exact
mesh verifier disproves that intersection, record disagreement and stay uncertain.
A certified zero surface distance establishes contact of the **mesh** surfaces;
positive source errors allow source separation. Prove source crossing separately
or report a clearance-threshold violation only when its entire interval is below
the threshold. Threshold zero needs explicit intersection/overlap semantics.

### Contact location is a region, not the clearance half-width

Two large parallel faces have a whole nearest region. An arbitrarily small tilt
can move the selected closest point across a face while scarcely changing the
distance. Thus neither `q` nor `e_A+e_B+q` bounds displacement of Parry's selected
point from an arbitrary true closest point. An exact witness proves one feasible
mesh pair, not uniqueness. Retain the raw point as an observed witness.

Provide an outer **paired contact region** with triangle/face provenance. A simple
first enclosure is the union of boxes for all candidate triangle pairs, each
operand's box inflated by its own source bound. For tighter regions, subdivide
candidate triangles in barycentric space using outward affine evaluations.
Keep a cell pair whenever its certified distance lower bound is
`<= U+2*(e_A+e_B)` (including query/placement terms in their chosen accounting).
Why: a source minimizing pair maps to mesh points with distance at most
`d_source+e_A+e_B <= d_mesh+2*(e_A+e_B) <= U+2*(e_A+e_B)`.
All such source witnesses therefore remain in the inflated retained cells.
Prune only by strict certified separation; keep ties and all minimizing pairs.
Return unions of paired boxes or a coarser containing region on resource limits,
with explicit width/diameter. A bounded region may be broad; it is not a
0.01 mm point certificate. If the agreed request requires that localization,
mark that part uncertain. Clearance, location and intersection need separate
qualification states so a scalar band never silently certifies a point.

## 4. Solid containment eligibility

The present closed-edge flag, positive summed volume and a surface miss are
insufficient. The vendored
[in_out_judge.rs](../../../cad/monstertruck-qualification/vendor/monstertruck-meshing/src/analyzers/in_out_judge.rs)
uses tolerance/inexact matrix inversion and strict interior ray hits; do not
promote it to the certified path. As an external mathematical/API reference,
[CGAL's side-of-mesh contract](https://doc.cgal.org/latest/Polygon_mesh_processing/classCGAL_1_1Side__of__triangle__mesh.html)
requires closed triangle surfaces without self-intersections and warns about
nested shells. This is a requirements reference, not a proposal to add C++.

First eligible class: each finite material solid has one connected embedded,
closed, consistently outward oriented two-manifold shell, no internal shell,
no duplicate/degenerate triangles and a proved source/mesh topology-preserving
correspondence. Check exact coordinate identity, two opposite incidences per
edge **and one circular vertex link**; two shells pinched at a vertex pass an
edge-only test. Check nonadjacent triangle intersections, and adjacent triangles
for overlap beyond their common edge/vertex, including coplanar duplicates.
Use exact rational signed volume/orientation after these topology checks.
An invertible reflection changes orientation; recompute/correct interpretation
without rewriting source metadata. Treat each retained source solid/occurrence
separately; a soup-level volume sum cannot authenticate their material ownership.

For nonintersecting eligible connected boundaries, exact ray parity of one
boundary representative from each component against the opposite material solid
detects nesting: boundary membership is constant while there is no intersection.
Test **both directions** and every component. A common interior point chosen
arbitrarily, e.g. both solids' centers, is an invalid shortcut: nested concentric
solids would misleadingly put both centers inside. Reject/retry rays hitting
edges, vertices, coplanar faces or unresolved predicates; a bounded list of exact
integer directions followed by unknown is sufficient, with no majority vote.

Mesh containment transfers to source solids only with the proved oriented domain
correspondence and a separation margin exceeding the source/mesh motion envelope
(or an independent source membership proof). A Hausdorff surface bound alone does
not preserve topology or material side. A closed thin shell can have almost the
same boundary but different interior. A certified global separating slab for
bounded source solids can independently rule out nesting; use a named reason,
not a caller-supplied unconditional boolean.

Initially ineligible: open shells; nonmanifold edges or vertex links; zero area,
self-intersection, duplicate/coincident faces; ambiguous orientation/material
sense; sealed/internal void shells; shell self-inclusion; overlapping/touching
components whose union ownership has not been proved; zero-thickness material;
unsupported source repairs/trims; singular transforms; unproved topology bridge.
Generated through-holes/handles can still have one valid connected shell;
do not confuse a through-hole with a separate internal void shell. For later
internal-shell support require an exact shell inclusion forest, alternating
material/cavity orientation and no intersections; never infer this from parity
over an unlabelled aggregate soup. Ineligible containment remains uncertain even
when unsigned surface distance is positive. A separately proved small surface
gap can still establish a positive-threshold clearance violation.

## 5. Proposed contract change, to agree before parallel implementation

Parent owns changes to
[ANALYSIS_CONTRACT.md](../../../cad/wasm/src/model/generated/ANALYSIS_CONTRACT.md)
and the **same**
[JSON fixture](../../../cad/wasm/tests/fixtures/generated-analysis-contract.json).
This note proposes meanings; it does not modify them. Add only the following
concepts, retaining existing IDs, f64 buffers, tags, null semantics and raw enum:

- Per operand: a qualification version/class and the accounting meaning of
  `analysisErrorBoundMm`; populate existing repair fields only when measured.
  Carry a containment eligibility result with explicit scope (solid/occurrence,
  mesh and source correspondence), reasons and algorithm version. It must be
  bound to the precise analysis mesh, not to a reusable source SHA alone.
- Per query: a `queryQualification` object containing either unknown/reasons or
  certified mesh distance `[lowerMm,upperMm]`, numerical method/version and,
  only if the current centered formulation is retained, its derived scalar
  `numericalErrorBoundMm`. Retain the original observation alongside it.
- Per query: `contactRegion` as unknown/reasons or conservative paired world-mm
  boxes/regions with triangle/occurrence tags, source inflation and achieved
  localization size; say whether it encloses mesh or source minimizing pairs.
  No uniqueness or normal/penetration accuracy is implicit. For intersection
  use an intersection region/state rather than claiming nearest points are a
  unique physical contact. Unsupported location remains explicitly unknown.
- Per query: a qualified containment outcome `ruled-out`, `contained-left`,
  `contained-right` or `unknown`, with per-component evidence/reasons. Separate
  this from surface intersection and the numeric clearance interval. Replacing
  the free boolean prevents a stale caller from asserting eligibility.

Minimum identity extension: bump `generated-analysis-v1` when certificates gain
semantics; version vendor derivation and query algorithms too. Bind certificates
to source SHA **or** generated geometry identity, accepted revision/snapshot,
selected repair and deflection options, actual analysis mesh generation,
operand row transforms and query settings. A backend-owned immutable mesh token
can bind the existing buffer to its qualification; do not add a custom SHA
manifest ceremony. Use the existing cache identity infrastructure. Query cache
keys additionally include requested accuracy, physical threshold, qualification
versions and refinement/resource settings. Coarse results cannot satisfy tighter
requests, and cancellation/revision changes prevent publication. Retain the
64 MiB transfer cap and validate new regions/counts/finiteness/index ranges.

Extend the existing fixture with independent planar positive and unknown/invalid
qualification examples and pair expectations; keep generated/vendor ownership
distinct. Producer `cad/wasm/tests/collision-analysis.cjs` and Host consumer tests
must consume those same additions, plus Runtime stale/refinement tests. Do not
repurpose `targetDeflectionMm` as requested accuracy or threshold. A reported
scalar Supported band must not be read as a supported point or solid claim.

## 6. Finite implementation order and completion criteria

1. **Internal planar vendor qualification.** Target
   `cad/monstertruck/src/step/{mod.rs,tessellation.rs}` plus a private qualification
   module and `cad/monstertruck/tests/step_import.rs`. Start with unchanged
   native-LINE/plane identity cases, complete domain tiling, no fallback and
   conservative parsing/evaluation bounds. Use capacitor, independent tetra and
   transformed occurrences. Add the resistor/cover only with independently
   bounded nonzero correspondence. Finish when the full capacitor retains all
   geometry and gets a finite internal symmetric bound/justified repair result,
   while missing-strip/hole, moved-line and unsupported-face controls fail
   qualification. The public wire remains unchanged in this checkpoint.
2. **Internal numerical verifier.** Target a private helper used from
   `cad/wasm/src/model/collision.rs`, with direct declarations of already locked
   rational dependencies if needed. Implement exact triangle feature cases,
   outward distance endpoints and conservative candidate regions. Finish when
   native and existing actual-WASM tests enclose analytic gaps, exact touching,
   crossing and coplanar overlap, plus large-translation/tiny/near-parallel
   counterexamples, and raw Parry provenance/results remain inspectable.
   Do not turn unknown resource exhaustion into a numerical bound.
3. **Parent contract checkpoint.** Agree/edit the authoritative document and
   shared fixture atomically, including term accounting, location semantics,
   containment states and identity versions. Decide whether the compatibility
   scalar is retained or the interval becomes authoritative; this is an
   implementation seam choice within the accepted scope, not a new user scope
   decision. Both producer and consumer owners read this exact revision before
   working in parallel. The internal checkpoints can be combined or reordered
   if they expose a concrete contract constraint.
4. **Generated bound and vendor producer.** Target
   `cad/monstertruck/src/profile.rs` and private analytic qualification helpers;
   consume existing layered carriers without changing their construction.
   Qualify emitted plane, cylinder and cone triangles with trim/UV/evaluation
   proofs. Target WASM `generated.rs` / `vendor_step.rs` only after step 3.
   Finish with finite bounds on plate, nonconvex drilled PCB, circular wall and
   countersink/frustum fixtures, and null bounds on a deliberately uncovered
   trim or unsupported surface. Default/refined meshes each need their own
   certificate. Keep 001/002 algorithm/acceptance changes separate.
5. **Containment and assessment.** Target CAD private topology/query helpers,
   `collision.rs`, Host `cad_jobs.rs`/`cad_worker.rs` and Runtime
   `model_delivery.rs`. Implement the one-shell eligibility checks, exact ray
   classification both ways, transform/error accounting and contact region
   delivery. Finish when nested positive surfaces become a contained result,
   open/pinched/self-intersecting/internal-shell controls remain unknown, and
   only eligible source conclusions become Clear/Violation. Keep scalar,
   intersection and location uncertainty distinct.
6. **Required acceptance checkpoint, parent-owned integration.** Add complete
   capacitor/vendor pair at translation `(3,0,0)`, the catalogue cover pair when
   qualified, generated/generated and vendor/generated positives to the existing
   actual-provider tests and fail-fast real-page probe. Default checks must have
   supported bands; 0.01 mm is explicit and may stay uncertain where a measured
   source/repair/query/location budget exceeds it. The capacitor's declared
   attachment allowance is 0.01 mm per operand, so under the additive source
   policy its pair cannot meet a 0.01 mm half-width even with zero repair and
   perfect meshing; that is a required honest tighter-request negative control.
   Run affected checks first, then parent's required full native/browser gates,
   once-per-completed-slice independent Standards/Spec reviews, performance 011
   and removal/audit 012. This research does not replace those gates.

Required analytic controls for these steps are intentionally small and independent:

- Unit boxes/tetrahedra at gaps 1, zero and crossing; nested boxes `[0,10]^3`
  and `[3,7]^3` have separated surfaces at distance 3 but overlapping material.
  Check both containment directions, far translations, 90-degree rotation,
  reflection and nonuniform scale/shear with correct norm inflation.
- Plate with authored hole; drop a triangle/strip, fill a hole, duplicate an
  exterior triangle, reverse one face, duplicate a coincident face, and join two
  closed components at one vertex. Edge-count/area/support-residual shortcuts
  must fail on these controls.
- Cylinder quadrant/closed seam with known `R*(1-cos(alpha/2))`, plus a large
  deliberately coarse subarc whose bound exceeds requested accuracy; use the
  actual emitted samples and intervals, not the configured deflection.
- Cone with unequal radii and large `DeltaZ*DeltaTheta`; a mixed-term omission
  must fail. For example `r0=1`, `r1=11`, `DeltaZ=100`, `DeltaTheta=0.1`:
  on the diagonal between opposite strip corners, the midpoint error has a
  tangential component `5*sin(0.05)` (about 0.25 mm), while the isolated angular
  term is only 0.01375 mm. Apex crossing, period-wrap error, wrong radius or
  unsupported trim stays unknown. Planar arc holes need exact winding and
  strip-coverage checks.
- Parallel broad plates and almost parallel plates: a narrow clearance interval
  accompanies a broad contact region; switching Parry's selected triangle must
  not silently shrink that region. Oblique triangle crossing and coplanar touch
  exercise cases excluded by the existing public vendor collision analyzer.
- Source master/trim change with unchanged support, moved endpoint at/beyond the
  selected repair limit, unsupported extra occurrence and stale source/revision/
  mesh token/transform. These must prevent reuse of a previous qualification.
- Cavity control: outer box `[-5,5]^3`, internal void shell `[-4,4]^3`, and a
  second box `[-1,1]^3` located in the void. The surfaces are separated but the
  second box is outside the first box's material. The initial one-shell qualifier
  must return ineligible/unknown, never apply an unlabelled parity shortcut or
  classify the cavity as solid material. Reverse the cavity sense as a separate
  negative control.

No additional user scope decision is required to start these checkpoints. The
remaining unknowns are implementation/proof gates: complete actual mesh domain
coverage, observed nonzero repair correspondence outside the first class,
finite numeric/contact enclosures and source containment topology. Arbitrary
spline/PCURVE/repaired fallback assets may remain uncertain under the accepted
scope. **All-vendor-uncertain**, a blanket 0.01 mm promise, or replacing null with
the selected meshing/repair setting would not complete this migration.
