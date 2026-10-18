# Private generated cylinder proof (014)

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
height and parameter interval, including native merged faces whose trims extend
beyond the generator's seed segment. Caps have nested, separated rectangular or
circular boundaries. Rectangles may contain exact collinear subdivisions.

The verifier binds every native face key, support, oriented wire and final
triangle/tag/UV to the actual shared-emitter trace. It checks final f64 and f32
positions, accepted triangle count, and complete face census. No alternate
mesh, deflection-derived angle, sampling maximum, normalized normal or global
closed-edge flag supplies a proof. Missing/placeholder UVs refuse. The native
UV lifting metadata is still unqualified; this proof independently authenticates
a geometric angular chart from exact radial directions and winding.

## Continuous source trim enclosure

Retained quadratic clamped NURBS spans are extracted in exact homogeneous
rationals using Cox-de Boor and three exact polynomial values. Positive weights
bound the denominator away from zero. For each span, with centered homogeneous
coordinates X,Y,W and retained cylinder radius R, Bernstein coefficients bound
the polynomial `X²+Y²-R²W²` on the **entire** interval. Consequently

`abs(radial_length-R) <= max_abs_coefficient / (min_weight² R)`.

This includes rounding already stored in native homogeneous coefficients.
Exact planar-height and endpoint joins, minor-sector control hulls, strictly
signed angular derivative coefficients, and winding exactly one prove a single
continuous radial graph, without trig evaluation or a guessed epsilon. Trim
height coefficients must be exactly planar; a current 1.6 mm fixture fails that
test and remains Unknown. Source precision is zero, separately from this direct
geometric contribution.

## Continuous final cylinder enclosure and coverage

For each **final** triangle, exact convex-hull distance to the cylinder axis
gives its minimum squared radial distance; the maximum is attained at a vertex.
For every point in that triangle, with squared radius s,

`abs(sqrt(s)-R) = abs(s-R²)/(sqrt(s)+R) <= abs(s-R²)/R`.

Taking the maximum gives a whole-triangle radial band. It includes all actual
surface evaluation, sampling, triangulation and coordinate rounding in the
emitted f64 positions; no additional per-ULP or evaluation allowance is assumed.

Directed internal triangle edges must cancel exactly once. The only remaining
boundaries must be two planar end cycles at the native trim heights. Exact minor
steps and ray-crossing winding prove each end cycle covers one full turn. Every
triangle's radial-projection Jacobian has the same strict sign at all three
vertices, hence throughout that triangle. Thus the oriented triangle chain's
radial projection has degree one on the open cylinder: its boundary has degree
one, internal projected edges cancel, and all interior contributions have the
same sign. Every regular target point therefore has a preimage; closure covers
projected edges/vertices. Projection is defined everywhere because the exact
radial lower bound is positive. This proves both mesh-to-cylinder distance and
cylinder-to-mesh distance, with identical Z, bounded by the radial band.

An actual open seam adds boundary cycles or invalid end edges and refuses. No
nearby vertices are welded. In particular the current radius-2/height-4 default
drilled fixture stays Unknown rather than receiving a fictitious seam proof.

## Complete cap and hole domains

All triangles on a plane must lie exactly on its retained mathematical plane
and have the plane's oriented sign, computed from the retained affine
coefficients in rationals. The complete directed boundary chain must equal the
source rectangles (exact interval partitions of all four sides) and the actual
adjacent cylinder end polygons. Boundary orientation distinguishes the exterior
from every hole. This is a chain identity, not an area match: winding of the
boundary equals the sum of triangle interior multiplicities. Positivity and
boundary winding one in material / zero in holes and exterior therefore force
complete coverage without overlaps or omitted slivers, including hole interiors.

Native circular boundaries and final end polygons are each radial graphs. Their
radii differ by at most `source_trim_band + final_triangle_band`. Enclosing bands
are proved disjoint and strictly nested using exact boxes (and exact squared
distance for a circular exterior). Each changed boundary can therefore move
radially without meeting another boundary. Every point in either cap's symmetric
difference has a same-ray boundary witness in the other cap within that sum.
Unchanged rectangular boundaries contribute zero. Both directed cap distances
are bounded. The maximum over all retained faces bounds the whole surface union.

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

Final checks: 40 native profile tests (13 new proof controls), 6 affected native
integration tests, and one actual Node/WASM entry invoking those same 13 proof
controls passed. Final WASM execution took 55.90 seconds; this is a correctness
run, not a performance comparison. Formatting, production strict Clippy and
test Clippy with only `items_after_test_module` / `chunks_exact_to_as_chunks`
baseline allowances pass. Unadjusted `--lib --tests -D warnings` still finds six
unchanged library-test findings and one integration-test finding. The four owned
generated-helper findings were removed in the separate lint cleanup commit.
Logs are `.evidence/cylinder-proof-{profile-tests,generated-integration,wasm-final,
clippy-production,clippy-scoped,clippy-strict,fmt}.log`.
