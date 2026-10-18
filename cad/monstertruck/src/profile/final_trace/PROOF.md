# Private generated cylinder proof (014)

Frozen `5bc56d35` failed independent Spec review with a P1 soundness
counterexample. Its prior green checks did not cover that defect. Corrected raw
orientation and cap accounting in `b03cc095` were accepted and integrated by the
parent at `95e8c4a53`. The continuous-height extension below is a new private
feature pending parent review, with no activation.

This is a test-only `analytic-generated-v1` source proof under the pending
[contract](../../../../wasm/src/model/generated/ANALYSIS_CONTRACT.md).
It does not activate v2, publish a mesh identity, qualify a query or containment,
or change production `analysisErrorBoundMm`. The physical row is absent: the
proof is for body-local final f64 triangles, including native source elevation
and placement. Certificate binding and physical-row accounting belong to the
parent's atomic activation.

## Accepted geometry and binding

One retained native solid/shell, with canonical unit-Z cylindrical carriers,
translation-only cylinder processors, and exactly axis-aligned planes. Cylinder
generator lines have constant positive X radius and zero Y. Their nonzero axial
coefficient is an affine carrier; full circular trim loops determine the actual
axial domain, including native merged faces whose trims extend beyond the
generator's seed segment. Caps have nested, separated projected rectangular or
circular boundaries. Rectangles may contain exact collinear subdivisions.

The verifier binds every native face key, support, oriented wire and final
triangle/tag/UV to the actual shared-emitter trace. It checks final f64 and f32
positions, accepted triangle count, and complete face census. No alternate
mesh, deflection-derived angle, sampling maximum, normalized normal or global
closed-edge flag supplies a proof. Missing/placeholder UVs refuse. The native
UV lifting metadata is still unqualified; this proof independently authenticates
a geometric angular chart from exact radial directions and winding.

## Continuous source trim enclosure

Raw retained `edge.curve()` quadratic clamped NURBS coefficients and knots are
converted to rationals first. Cox-de Boor and three exact polynomial values
extract each span in that raw domain. A backward edge reverses the exact span
order and each span's homogeneous controls; no f64 knot reflection or source
reparameterization occurs. Raw endpoints are checked against the edge's absolute
vertices, then oriented span joins and wire winding are checked exactly. Positive
weights bound the denominator away from zero. For each span, with centered homogeneous
coordinates X,Y,W and retained cylinder radius R, Bernstein coefficients bound
the polynomial `X²+Y²-R²W²` on the **entire** interval. Consequently

`abs(radial_length-R) <= max_abs_coefficient / (min_weight² R)`.

This includes rounding already stored in native homogeneous coefficients.
Exact three-dimensional endpoint joins, minor-sector control hulls, strictly
signed angular derivative coefficients, and winding exactly one prove a single
continuous radial/height graph, without trig evaluation or a guessed epsilon.
For each raw span and exact reference height z0, the same positive denominator
also gives the continuous enclosure

`abs(Z/W-z0) <= max_i abs(Z_i-z0 W_i) / min_i W_i = H_span`.

Taking maxima over **every** span bounds the entire loop, not just its endpoints
or sampled evaluations. Equality to zero is a special case, not a tolerance
test. The reference heights are actual native end vertices, not requested
thickness or deflection. Source precision remains zero; retained coefficient
distortion belongs to the direct geometric contribution.

For a canonical translated unit-Z cylinder, the continuous inverse chart of a
non-axis point has its radial direction and unchanged world Z. Pulling each raw
trim back through this chart projects it radially onto the retained carrier;
its height is unchanged. Each authenticated full turn is therefore a continuous
height graph h(theta). If end reference levels are z0 < z1, the exact check
`2 max(H_lower,H_upper) < z1-z0` proves strict separation for all angles. The
source domain is the complete strip between these two graphs, not a constant-Z
rectangle. Same-angle affine interpolation of its endpoints gives a continuous
correspondence with the reference cylinder strip. In both directions its vertical
displacement is at most the maximum height enclosure. Raw off-carrier trim
points themselves are additionally enclosed by their radial contribution.

## Continuous final cylinder enclosure and coverage

For each **final** triangle, exact convex-hull distance to the cylinder axis
gives its minimum squared radial distance; the maximum is attained at a vertex.
For every point in that triangle, with squared radius s,

`abs(sqrt(s)-R) = abs(s-R²)/(sqrt(s)+R) <= abs(s-R²)/R`.

Taking the maximum gives a whole-triangle radial band. It includes all actual
surface evaluation, sampling, triangulation and coordinate rounding in the
emitted f64 positions; no additional per-ULP or evaluation allowance is assumed.

Directed internal triangle edges must cancel exactly once. The only remaining
boundaries must be exactly two end cycles. Exact minor steps and ray-crossing
winding prove each end cycle covers one full turn and is a height graph after
radial projection. Every final end vertex is enclosed about exactly one native
reference end level; the exact midpoint and half-interval tests prove disjoint
slabs and a unique lower/upper assignment. A chord's height stays between its
end heights, so this is a whole-edge enclosure `H_mesh`, including final f64
evaluation rounding. This assignment is linked to each cap by the actual shared
native edge set and actual final boundary edge set, not a fitted level. Every
triangle's radial-projection Jacobian has the same strict sign at all three
vertices, hence throughout that triangle. Thus the oriented triangle chain's
radial projection has degree one **between its variable-height boundary graphs**
and degree zero outside them: internal projected edges cancel, the two end turns
are opposite, and all interior contributions have the same sign. Every regular
point between the graphs has a preimage; positivity excludes images outside.
Closure covers projected edges/vertices. Projection is defined everywhere
because the exact radial lower bound is positive.

At each angle the source and final projected domains are two closed Z intervals.
Their endpoint differences are at most `H_source + H_mesh`; clamping Z to the
other interval proves both directed interval distances. Final projection
coverage supplies an actual triangle preimage for the reverse direction.
Combining that vertical displacement with the whole-triangle radial enclosure
gives a symmetric distance at most `E_mesh_radial + H_mesh + H_source`.
Including the raw source trim radial enclosure is conservative and also covers
the retained three-dimensional boundary curves. The former exactly planar
degree argument is therefore not being applied to a variable-height domain.

An actual open seam adds boundary cycles or invalid end edges and refuses. No
nearby vertices are welded. In particular the current radius-2/height-4 default
drilled fixture stays Unknown rather than receiving a fictitious seam proof.

## Complete cap and hole domains

The source cap is its original retained plane carrier restricted by the
continuous parameter pullback of **all** its oriented trims. For an exactly
axis-aligned plane, its exact nonsingular tangent frame and normal form a basis;
the first two coordinates of the inverse do not depend on normal displacement.
Re-evaluation at those coordinates is precisely orthogonal projection onto that
same plane. This is the mathematical chart used by native `Plane::parameter`
and its nearest-parameter method, not an arbitrary fit or the mesher's rounded
inverse samples. Circular source trims project with exactly the same XY radial
graph; their complete height enclosure bounds the raw trim-to-plane discrepancy.
Line trims have an affine normal coordinate, so their exact endpoint maximum
bounds their whole discrepancy. Their projected rectangle sides must still
match exactly. No noisy line fitting or general affine surface class is added.

Each actual final cap triangle is projected affinely onto that same source
plane solely for the proof. Its maximum normal displacement is attained at a
vertex, giving `H_cap_mesh` exactly. Its projected oriented area must have the
retained plane's sign; collapsed or folded projections refuse. Directed edges
of the complete projected triangle chain must cancel once, and its boundary
must be exactly the projected boundary of the original final chain. The actual
unprojected circular boundaries must also equal the actual adjacent cylinder
end polygons. Source rectangles require exact interval partitions of all four
projected sides. Thus there is no alternate tessellation or shadow delivered
mesh. Boundary orientation distinguishes the exterior from every hole. Winding
equals the sum of positive projected triangle multiplicities, forcing complete
coverage in material and zero coverage in holes/exterior, without overlaps or
omitted slivers. Each projected material point has an actual final preimage
within `H_cap_mesh`.

Native circular boundaries and final end polygons are each radial graphs. Their
radii differ by at most `source_trim_radial_band + final_triangle_radial_band`. Enclosing bands
are proved disjoint and strictly nested using exact boxes (and exact squared
distance for a circular exterior). Each changed boundary can therefore move
radially without meeting another boundary. Every point in either cap's symmetric
difference has a same-ray boundary witness in the other cap within that sum.
Exact projected rectangular boundaries contribute zero radial displacement;
their raw normal discrepancy is already enclosed separately. Both directed cap distances
are bounded after adding `H_cap_mesh` for the actual triangle preimage. The
source trim height contribution is also retained conservatively, including
raw off-plane boundary curves. The maximum over all retained faces bounds the
whole surface union.

Each cap independently authenticates **every** circular trim and takes the
maximum of its radial-plus-height errors, line height errors, and the inherited
cylinder-source contribution, then adds the mesh radial contribution and that
cap's final normal displacement once. Cylinder results likewise add the source
radial-plus-height maximum and final end-height maximum to the mesh radial
contribution. Cap and cylinder results combine by maximum, not addition.
This same cap band is used for axis exclusion,
nesting/separation, and its returned surface bound. The whole-body result takes
the maximum of the cylinder and all cap bands. A matching cylinder carrier
identity never authorizes dropping a cap trim's independently derived error.

## P1 reproduction and correction

The old `edge.oriented_curve()` inverted NURBS in f64 before extraction. With
interior knots `0.00008, 0.00015` and endpoint `1e12`, reflection merges the two
distinct knots at `999999999999.9999`. Raw homogeneous semicircle controls with
stored `sqrt(0.5)` weights can then have a large radial deviation while their
rounded replacement looks almost circular. The cylinder's backward use received
the small replacement bound; the cap's forward use computed a larger raw bound
but discarded it. Source binding still passed because the edge was shared.

The genuine Rust RED is preserved in commit `e98e0efc` and
`.evidence/cylinder-p1-red.log`: the frozen proof reported
`0.015384615384616942` mm although an actual final cap point, at signed
`(47/64,47/64)` relative to the retained hole center, is more than `1/32` mm
from the entire source surface. The source-only mutation preserves the real fine
emission trace and has a binding-success precondition, including actual opposite
native edge uses on cylinder and cap. An independent exact homogeneous
control-hull enclosure covers every dyadic subinterval of each raw trim, proves
distance from the witness ball, and preserves winding under chord homotopy.
It proves the witness is inside the deformed source hole; exact radial and
plane inequalities exclude every other source face. This oracle is a continuous
enclosure, not a sampled maximum or floating tolerance.

The focused command is `cargo test --manifest-path cad/monstertruck/Cargo.toml
--lib p1_asymmetric_shared_trim_cannot_publish_a_smaller_surface_bound --
--nocapture`. At the preserved RED checkpoint only the regression was added to
the frozen proof; no proof implementation was changed to manufacture failure.

Four corrective regressions cover that whole-surface counterexample, exact
orientation invariance despite the near-ULP reflected-knot collision, a cap's
returned error with zero inherited cylinder allowance, and cap-only error
entering the nesting separation check. The same four join the original 13
controls in the actual private WASM entry. No numeric limits or oracle tolerances
were weakened; unsupported source classes and production bounds are unchanged.

The final outward f64 conversion is checked against the exact rational bound;
`next_up` is used only when an exact comparison proves conversion rounded down.
Translation has norm one and is included directly in source/final coordinates.
General source affine maps, cones, unproved trims/seams, invalid normalization,
out-of-domain numerics and exhausted limits return Unknown. No global continuous
solid correspondence or containment follows from this facewise surface bound.

## Evidence and private WASM harness

The expected RED is `.evidence/cylinder-proof-red.log`. Native tests include
real drilled 20x10 plates, the authored-circle PCB with an additional drill,
the existing tray/boss route, refinement and independent radius/height controls,
clearance-band refusal, consistently bound missing/duplicate/sliver triangles,
UV/census mutations, trim coefficients, spans, normalization, translation,
large coordinates and the real open seam. Production bounds remain null.

The private WASM harness uses the **same source library path**, with the existing
standalone CAD manifest's dependency/patch paths made absolute in a temporary
manifest and only `wasm-bindgen-test = "=0.3.79"` added as a harness dev dependency.
Its target is outside `.evidence/`. The WASM test entry invokes these same proof
and adversarial test functions; a compile alone is not acceptance. Repository
manifests/locks and the parent-owned cgmath provenance work are untouched.

Frozen pre-review checks: 40 native profile tests (13 proof controls), 6 affected native
integration tests, and one actual Node/WASM entry invoking those same 13 proof
controls passed. Final WASM execution took 55.90 seconds; this is a correctness
run, not a performance comparison. These checks predate P1 and do not establish
soundness of the frozen proof. Formatting, production strict Clippy and
test Clippy with only `items_after_test_module` / `chunks_exact_to_as_chunks`
baseline allowances pass. Unadjusted `--lib --tests -D warnings` still finds six
unchanged library-test findings and one integration-test finding. The four owned
generated-helper findings were removed in the separate lint cleanup commit.
Logs are `.evidence/cylinder-proof-{profile-tests,generated-integration,wasm-final,
clippy-production,clippy-scoped,clippy-strict,fmt}.log`.

P1 correction checks: 44 native profile tests (original 40 plus four P1
regressions), all 6 affected integration tests, formatting, production strict
Clippy and the same scoped test-Clippy gate pass. The unchanged actual Node/WASM
entry executes the original 13 controls plus all four P1 controls: one entry
passed in 78.92 seconds. Only its execution timeout was extended to 300 seconds
for the additional continuous exact oracle; no proof limits or oracles changed.
The unadjusted strict test gate still reports the same six preexisting library
findings and one integration finding, with none in this correction.

The original runtime RED and checkpoint remain available. Corrected logs are
`.evidence/cylinder-p1-{focused-green,profile-tests,integration,fmt,
clippy-production,clippy-scoped,clippy-strict,wasm}.log`. The focused GREEN reports
`0.48205128205128323` mm for the adversarial source/final pair; the independent
whole-source witness lower bound remains strictly greater than `1/32` mm.
That correction was subsequently accepted by the parent; these historical
checks do not validate the new height feature. Production bounds, contracts,
public APIs and dependencies remain unchanged.

## Ordinary 1.6 mm height extension

The genuine missing-proof RED is preserved in `9d4cf616` and
`.evidence/cylinder-height-red.log`. It builds the real authored/source circular
PCB plus drill and the rectangular drilled PCB, checks successful source/final
binding, and requests a symmetric bound. Frozen `b03cc095` refuses the first
fixture at `nonplanar or nonpositive circular trim`. The delivered mesh also has
top vertices at `1.6000000000000003`, so merely allowing nonzero source residuals
would not have supplied the missing final-domain proof.

Seven new native controls join all original 17 proof controls, including all
four P1 regressions. Real 1.6 mm fixtures require nonzero retained trim-height
and final end-height enclosures, complete source and cap domains, independently
authored outline/thickness, refinement, and null production bounds. For the
20x10 plate with radius-1 drill, default/tighter bounds are respectively
`0.05882352941176469` / `0.01538461538461578` mm. For the source-circle PCB
(center 10,8; outer radius 2; drill radius 0.5), they are
`0.11764705882352981` / `0.01797752808988804` mm. In particular, the 0.01 mm
meshing request does not establish a 0.01 mm source accuracy certificate.

The distortion oracle uses the actual raw first knot span and an independently
worked B-spline basis at t=1/8: `(2,5,1)/8`. Its homogeneous quotient gives an
actual upper source trim point (and the same-height carrier boundary point)
above **every** final triangle. The reported symmetric bound must exceed this
whole-final-surface lower distance. Adding 0, 1/32, 1/8, 1/4 to one retained Z
coefficient grows the bound to approximately 0.015385, 0.065385, 0.215385,
0.415385 mm; adding 1 refuses the separated-height-domain proof. A cap checked
without inherited cylinder allowance must include its own height contribution.
Other controls consistently bind a missing subarc, discontinuous knots, zero or
negative denominator, and a final displaced hole end/sloping cap. The latter
also has an independent raw positive-weight source-height hull and a final
point above it. Small displacement grows the bound; large displacement refuses.
A narrow hole that qualified before source Z distortion refuses after its
complete trim band reaches the exterior. Moving the original source cap plane
by 1/8 must grow the bound rather than accepting its old nominal value.

Authored elevation 7 followed by native translation (32,-16,8) is independently
checked against final coordinates, for both literal 1.6 and literal 2.0 mm.
All contributions use retained source and delivered body-local coordinates
exactly once; no physical row is accepted. A reflected native cylinder processor
is consistently bound and explicitly Unknown (`unsupported source affine
transform`), rather than being silently canonicalized. Cones still lack the
joint mixed/evaluation/trim proof and remain Unknown. This feature adds no
general affine maps or continuous containment correspondence.

Current native evidence is `.evidence/cylinder-height-profile-tests.log` (all
51 profile tests, including all 24 proof controls) and
`.evidence/cylinder-height-integration.log` (all 6 tests across `layered_case`,
`native_revolution_frame`, `profile_validation`). Commands were
`cargo test --manifest-path cad/monstertruck/Cargo.toml --lib profile:: -- --nocapture`
and the same manifest with `--test layered_case --test native_revolution_frame
--test profile_validation`. Formatting and production strict Clippy pass;
test Clippy passes with only the two existing baseline lint classes allowed.
The unadjusted strict test gate still fails at the same six library-test sites
and one integration-test site, with no owned-helper finding. Logs are
`.evidence/cylinder-height-{fmt,clippy-production,clippy-scoped,clippy-strict}.log`.
The parent's separate integration cleanup and cgmath dependency patch were not
replayed: these native checks and the temporary private WASM harness use this
lane's existing standalone resolution (registry cgmath 0.18.0). Repository
manifests, locks and dependency declarations are unchanged.

Final actual Node/WASM execution passed in 203.15 seconds: one
`generated_cylinder_proof_actual_wasm` entry invokes **all the same 24 native
proof controls once**, including the final literal source dimensions and every
P1 control. The entry/call census is `.evidence/cylinder-height-wasm-controls.log`;
execution is `.evidence/cylinder-height-wasm.log`. The earlier pre-assertion pass
is retained separately as `cylinder-height-wasm-initial.log`. The command used
the existing `/tmp/boardstudio-generated-proof-harness/Cargo.toml`,
`--target wasm32-unknown-unknown --lib generated_cylinder_proof_actual_wasm --
--nocapture`, runner `wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner`, and
`CARGO_PROFILE_DEV_DEBUG=false`. `WASM_BINDGEN_TEST_TIMEOUT=600` extends only the
test execution budget; numerical-domain limits, continuous proof predicates and
independent distance oracles are unchanged. No source was copied, no target was
placed in `.evidence/`, and compilation alone was not counted as validation.
All checks remain private harness evidence, not a production runtime certificate
or approval to integrate/activate this new feature.
