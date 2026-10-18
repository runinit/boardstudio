# Generated analysis wire contract

This is the authoritative boundary between `generated.rs` and the Host CAD worker
(`web/crates/host/src/cad_worker.rs`, with validation in `cad_jobs.rs`). Tickets
010 and 014 share the [contract fixture](../../../tests/fixtures/generated-analysis-contract.json).
Change the producer, consumer and their fixture tests together when this boundary changes.

## Request and ownership

The on-demand worker operation is `analyze-generated-body`. It uses the existing
captured prepared assembly and `generatedAnalysisRequest`:

```json
{"bodyId":"contract-plate","geometryIdentity":"<worker-derived identity>","targetDeflectionMm":0.01}
```

The worker validates that exactly one prepared body matches `bodyId`, that body
and assembly revisions match the captured snapshot, and that `geometryIdentity`
matches the complete typed `PreparedCaseIR`. The identity is
`m1/generated-analysis-v1/<SHA-256 of serde_json::to_vec(prepared body)>`.
`m1` is the existing generated geometry cache version; `generated-analysis-v1`
versions this analysis contract and algorithm. Bump the relevant version when
its semantics change. These are manually versioned identities, not accuracy certificates.

The worker invokes `analyze_generated_body(selected PreparedCaseIR, options)`;
`options` contains only optional `targetDeflectionMm`. Omission or null selects
the existing 0.1 mm default; a supplied target must be finite and positive, and
the selected deflection is `min(0.1, target)`. The backend reports the actual
selection in `deflectionMm`. Requested meshing deflection, requested collision
accuracy and the physical clearance threshold are separate quantities.

Ordinary preview and STEP output do not request this operation. It may reuse
the exact generated body cache; display f32 buffers cannot become analysis data.

## Direct backend result

The backend returns this object directly, without a `generatedAnalysis` wrapper,
`geometryIdentity`, `sourceSha256` or `vendorStep`:

| Field | Meaning |
| --- | --- |
| `bodyId`, `revision` | Selected prepared body identity and accepted revision; revision is an integer no greater than 2^53 - 1. |
| `surfacePositionsMmF64` | Nonempty finite `Float64Array`, xyz triangle soup, length divisible by 9. Coordinates use the exact generated body's millimetre frame. |
| `triangleFaceOccurrenceIndices` | `Uint32Array`, one valid occurrence index per triangle. |
| `sourceFaces` | Generated face records: `solidIndex`, `shellIndex`, `faceIndex` (u32), and nonempty `supportKind`. These are generated indices, not STEP entity IDs. |
| `faceOccurrences` | Records with `sourceFaceIndex` and `solidOccurrenceIndex` (u32). Every triangle tag resolves through an occurrence to a retained generated source face. |
| `analysisErrorBoundMm` | Null until a symmetric source-to-mesh bound is qualified. A supported future value must be finite and nonnegative. |
| `meshClosedOriented` | Measured mesh topology metadata; it does not certify containment, source approximation or numerical query error. |
| `deflectionMm` | Finite positive selected meshing setting; it does not certify an error bound. |

The worker checks `bodyId` and `revision`, attaches its validated
`geometryIdentity`, and wraps the result under `generatedAnalysis`. Transfer the
original typed buffers with the metadata; validate sizes, index ranges, finite
coordinates, identity, revision and settings again at the consumer. Reuse the
existing 64 MiB collision buffer limit. Apply the physical `rowTransformF64`
exactly once when querying, not when storing these buffers.

Cache identity includes the captured snapshot, complete generated geometry
identity and requested refinement settings. A coarser result cannot satisfy a
tighter request. Cancellation or a changed snapshot prevents publication.

## Collision and vendor identity

Each collision operand contains `modelId`, `rowTransformF64`, and exactly one of:

- `sourceSha256`: verified immutable STEP bytes, with the matching STEP import options;
- `generatedGeometryIdentity`: the generated identity above, with default STEP import options.

Missing or dual identities reject. Generated identity never occupies a STEP
SHA field. Vendor `targetDeflectionMm` and `previewRepairBudgetMm` both enter
derived asset cache identity. Preserve declared source precision separately
from explicit `selectedRepairBudgetMm`. Unmeasured actual repair movement is
null (`maxRepairDisplacementMm` and per-face `repairDisplacementMm`), not zero
or a sampled support residual.

Raw surface intersection or separation is a mesh observation. Unknown source,
query-numerical, containment or contact-location bounds remain uncertain. A
nested solid can have separated surfaces; zero unsigned distance does not
establish penetration depth or a unique contact location.

## Shared fixture checks

The fixture independently authors a 20 x 10 x 2 mm plate and a valid example
of its direct wire payload. JSON arrays must be materialized as the typed arrays
above at the WASM/worker boundary. Face and triangle ordering are not pinned;
the producer may tessellate differently while preserving the bounds, complete
coverage and resolvable provenance.

The producer test invokes the actual WASM export with `preparedBody`, verifies
the fixture's direct field shapes/ownership, analytic bounds, default/refined
settings, null accuracy and valid provenance. The Host consumer test reads this
same file, materializes `directPayload`, adds the worker identity/wrapper and
checks decoding and request/reply validation. Mutations must reject stale
body/revision/identity, invalid provenance and malformed buffers. Both tests
must run in their owning acceptance commands; passing only one side does not
accept the boundary.

## Pending qualification v2 — not active wire

This section and the same fixture's `qualificationV2` section pin the next
boundary. They add no fields to today's `directPayload`. Existing exact-key v1
checks remain required. Parent integrates v2 producer, consumer and both fixture
checks atomically. No production certificate or preview admission change is
created by this private checkpoint.

Pending identities: `m1/generated-analysis-v2/` uses the existing prepared-body
digest ownership; `vendor-step-analysis-v2/` binds the authenticated STEP SHA and
selected import options through existing derived-cache identity machinery.
Source proof versions are `planar-final-v2` and `analytic-generated-v1`; query
algorithm is `aabb-interval-exact-v1`. Active v1 identities remain unchanged.

### Exact pending types and units

All objects reject unknown keys. Numbers are finite; bounds/errors are nonnegative
and ordered, with outward arithmetic. `xyz` is three world-mm numbers. `u32` and
`i32` obey their named integer ranges; `u53` is an integer in `[0,2^53-1]`.
Every string below is nonempty; every reasons list is nonempty.

```text
SourceQualification = {state:"unknown", reasons:[string,...]}
 | {state:"bounded", class:"complete-native-line-plane"|"analytic-generated",
    version:1|2, directBoundMm:number, sourcePrecisionMm:number,
    totalBoundMm:number, meshIdentity:string}
QueryQualification = {version:1, algorithm:"aabb-interval-exact-v1",
 binding:Binding, limits:Limits, meshDistance:MeshDistance,
 contactRegion:ContactRegion, containment:Containment}
Binding = {snapshotRevision:u53, leftMeshIdentity:string, rightMeshIdentity:string,
 leftRowTransformF64:number[16], rightRowTransformF64:number[16]}
Limits = {maxTriangles:u32, maxNodePairs:u32, maxExactPairs:u32,
 maxContactBoxes:u32, minimumExponent:i32, maximumExponent:i32,
 requestedNumericalWidthMm:positive number}
MeshDistance = {state:"unknown", reasons:[string,...]}
 | {state:"bounded", lowerMm:number, upperMm:number, witnessTriangles:[u32,u32]}
ContactRegion = {state:"unknown", reasons:[string,...]}
 | {state:"boxes", sourceInflationMm:[number,number],
    pairs:[{leftMm:[xyz,xyz], rightMm:[xyz,xyz]},...]}
Containment = {state:"unknown", reasons:[string,...]}
 | {state:"ruledOut", method:"slab", axis:0|1|2,
    leftUpperMm:number, rightLowerMm:number}
 | {state:"ruledOut", method:"parity", correspondenceVersion:string,
    componentCount:[u32,u32]}
 | {state:"containedLeft"|"containedRight", correspondenceVersion:string}
```

Pending generated/vendor analysis adds `sourceQualification`; collision results
add `queryQualification` alongside the unchanged raw Parry observation.
`containedLeft` means left material contained in right, and conversely for right.
The backend/worker authenticates evidence against `Binding`; a caller boolean or
unvalidated JSON assertion cannot produce qualified containment.

### Source accounting and final geometry

`directBoundMm` is a symmetric Hausdorff upper bound from the complete nominal
source surface set to FINAL exported f64 triangles/tags, after polygon fans,
source placement/mirroring and render filtering. Missing source faces/occurrences
or incomplete proof return unknown. The first vendor class is limited to
consistent native LINE/plane geometry, millimetres and at most one independently
checked planar source-frame placement per solid. `complete-native-line-plane`
uses proof version 2; `analytic-generated` retains version 1. This source-proof
version change corrects the previously mistaken identity-frame assumption; it
does not activate a new wire format or change the query algorithm version.
It uses original decimal lexemes, rational plane/boundary
correspondence and complete source/mesh census; stage residuals alone are not
proof. Generated qualification can use retained exact-body analytic parameters
and actual emitted triangle/trim coverage. Cone sagitta alone omits the possible
slope/angle/height mixed term; numeric/evaluation terms remain required.

`sourcePrecisionMm` is the authenticated declared attachment allowance, zero for
generated inputs without a declaration. It does not describe manufacturing
accuracy. `totalBoundMm` outwardly encloses `directBoundMm+sourcePrecisionMm`.
On activation, existing `analysisErrorBoundMm` equals this total or null for
unknown. Charge it once per operand. A direct proof can absorb conversion,
repair and meshing error; never add those terms again. It does not separately
measure repair movement: unmeasured `maxRepairDisplacementMm` and per-face
`repairDisplacementMm` stay null. 007's actual-movement ledger and selected-budget
enforcement remain required and unimplemented by this checkpoint.

The bounded placement subset has one `ITEM_DEFINED_TRANSFORMATION`, Z axes
`(0,0,1)` and finite nonzero XY reference directions. Read original decimal
origins/axes/refdirs and authenticate the representation/occurrence links.
Normalize each XY frame with an independently proved sqrt enclosure; compose
child-to-parent as `parentFrame * inverse(childFrame)`, including frame origins
in millimetres. Bound rational frame approximation versus the normalized source
frame and compose that bound with final f64 transform rounding. Copying an
unvalidated importer float matrix or comparing frame IDs is insufficient.
General mapped/multiple-level assemblies and other frame classes stay unknown.

For the actual capacitor, `#552` uses `#715=(0,0,0)`, `#586=(0,0,1)`,
`#587=(1,0,0)`; `#583` uses `#850=(0,0,0)`, `#712=(0,0,1)`,
`#713=(2.4980018054066e-16,-1,0)`. They are not identical. The nominal 3 mm
translation and 1 mm gap are defined in the native part frame. The equivalent
world translation along normalized native X is interval-valued in the fixture.
Actual supplied f64 row translations and their differences from that reference
must be accounted for; STEP assembly placement and the physical row each apply
once. Slabs are tested on actual complete inflated **world-axis** hulls only.
If necessary reorder both operands/bindings together before emitting the ordered
slab form; a rotated native axis is not a world-axis slab certificate. The common
30-degree physical-row control retains this distinction and independent numeric
accounting. A case without world-axis separation stays unknown in this subset.

For physical `T(x)=Ax+t`, scale asset-frame total by an outward `||A||_2`
enclosure, e.g. `sqrt(||A||_1*||A||_infinity)`; maximum column length is insufficient
for general affine maps. Bound actual vertex transform rounding against exact
rational `A*p+t`; its maximum bounds the transformed triangle. Charge rounding
once. If the direct source/final proof already includes this physical row, bind
the proof to that row and apply neither term twice. No guessed ULP allowance.

### Query, location and containment

`meshDistance` encloses actual f64 triangle-surface distance. Independently check
seed indices and feasible upper bounds. Raw Parry distances/points alone are not
certificates. Outward AABB and interval triangle filters use exact ambiguous/zero
fallback; justify sqrt endpoints by exact square comparisons. Resource/exponent
caps return unknown. The source interval is
`[max(0,L-eLeft-eRight), U+eLeft+eRight]`, outwardly evaluated, only if both source
bounds exist. Requested accuracy, deflection and clearance threshold stay separate.
Unsigned zero proves neither penetration depth nor source crossing.

Contact boxes have finite min <= max on each axis, and 1..`maxContactBoxes` pairs.
Retain every pair with certified distance lower bound <= `U+2*(eLeft+eRight)`,
for both crossing and separated surfaces. Inflate each side by its source bound.
On cap pressure merge into containing boxes; truncation is invalid. Without both
source bounds return unknown. Broad parallel-face regions are expected. Scalar
clearance does not qualify a unique point, normal or penetration depth;
localization and scalar accuracy have separate qualification.

Slab evidence requires complete bounded source-solid hulls with strictly disjoint
allowance-inflated projections (`leftUpperMm < rightLowerMm`). Swap operand order
before producing this form if necessary. It handles the capacitor's coincident/
touching constituent solids without claiming one-shell parity eligibility.
Parity/contained states require eligible oriented embedded manifold boundaries
and a continuous global source/mesh correspondence. Holes/cracks can retain a
finite symmetric surface band without containment transfer. Closed-edge counts,
summed volume, separated surfaces or a caller boolean are insufficient.

### Validation and identity

Reject nonfinite/negative/reversed intervals, invalid indices, empty reasons or
regions, over-cap/truncated boxes, overlapping slab endpoints, invalid affine
rows, inconsistent contribution sums, stale revisions and mismatched mesh/row/
algorithm/settings identities. The shared pending examples and invalid mutations
pin these cases without changing v1 expected keys or copying its prepared input.

Certificate/cache identity binds source SHA or complete generated identity,
revision/snapshot, FINAL buffers/provenance, proof versions, physical rows,
selected repair/deflection settings, query algorithm and every limit above.
Assessment also binds requested accuracy and physical threshold. Use existing
cache/token machinery, not new hash manifests. Coarse output cannot satisfy a
tighter request; cancellation or changed revision prevents publication. Active
transfer limits and both actual producer/consumer tests remain required at v2
integration. The private proof has no active frontend schema implementation.
