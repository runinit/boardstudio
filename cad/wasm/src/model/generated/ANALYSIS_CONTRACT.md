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
establish penetration depth or a unique contact location. Repaired or incomplete
geometry may still be queried approximately, and the observation reports the
geometry actually checked. An incomplete model cannot establish clearance for
the whole model, and a measured mesh gap is not a guaranteed physical gap.

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

## Qualification v2 — active wire

This section and the same fixture's `qualificationV2` section are the v2 wire
contract. The v2 producer, the Host/Runtime consumer and both fixture checks are
integrated: the WASM provider emits `analysisMesh`, `sourceQualification` and the
opaque handle, the worker authenticates them against its own request, and
`cad/wasm/tests/qualified-analysis.mjs` plus the worker, Runtime and page checks
exercise the qualified query. The CAD replacement map records delivery in its
010 and 014 outcomes; 014 cites `01680d84`. Existing exact-key v1 checks remain
required for v1 payloads. This section is a wire contract, not a preview-admission gate.
Admission follows the current vendor-preview policy: ADR-0017 and ADR-0018
(accepted 2026-10-19) and the CAD backend replacement plan
([map](../../../../../docs/plans/cad-backend-replacement/map.md), ticket 015 and its
successors). Tickets 016–018 own delivery of surviving partial geometry, visible
incomplete-model errors and approximate queries; these are policy requirements,
not claims that those routes already work. Unsupported accuracy and whole-model
clearance stay uncertain.

Active identities: `m1/generated-analysis-v2/` uses the existing prepared-body
digest ownership; `vendor-step-analysis-v2/` binds the authenticated STEP SHA and
selected import options through existing derived-cache identity machinery.
Source proof versions are `planar-final-v2` and `analytic-generated-v1`; query
algorithm is `aabb-interval-exact-v1`. Active v1 identities remain unchanged.

### Exact types and units

All objects reject unknown keys. Resource counts/caps are positive integers.
Numbers are finite; bounds/errors are nonnegative
and ordered, with outward arithmetic. `xyz` is three world-mm numbers. `u32` and
`i32` obey their named integer ranges; `u53` is an integer in `[0,2^53-1]`.
Every string below is nonempty; every reasons list is nonempty. WASM validators
must enumerate JS object keys before serde-wasm-bindgen decoding;
`deny_unknown_fields` alone does not enforce this boundary.

```text
SourceQualification = {state:"unknown", reasons:[string,...]}
 | {state:"bounded", class:"complete-native-line-plane"|"analytic-generated",
    version:1|2, directBoundMm:number, sourcePrecisionMm:number,
    totalBoundMm:number, meshIdentity:string,
    accounting:"direct-includes-conversion-repair-trims-evaluation"}
QueryQualification = {version:1, algorithm:"aabb-interval-exact-v1",
 binding:Binding, limits:Limits, meshDistance:MeshDistance,
 sourceBounds:[WorldSourceBound,WorldSourceBound],
 contactRegion:ContactRegion, containment:Containment}
Snapshot = {token:u64decimal, sessionEpoch:u64decimal, documentId:string,
 boardId:string, instanceId:string|null, revision:u53}
Binding = {snapshotRevision:u53, snapshot:Snapshot, workerGeneration:string,
 leftModelId:string, rightModelId:string,
 leftMeshIdentity:string, rightMeshIdentity:string,
 leftAnalysis:AnalysisRef, rightAnalysis:AnalysisRef,
 leftMesh:AnalysisMesh, rightMesh:AnalysisMesh,
 leftRowTransformF64:number[16], rightRowTransformF64:number[16],
 clearanceThresholdMm:number, requestedAccuracyMm:positive number|null}
WorldSourceBound = {state:"unknown", reasons:[string,...]}
 | {state:"bounded", meshIdentity:string,
    method:"scaled-local-plus-row-rounding"|"direct-world",
    normUpper:number, directBoundMm:number, sourcePrecisionMm:number,
    transformRoundingMm:number, totalBoundMm:number}
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
Source = {kind:"vendor-step", sourceSha256:sha256, importOptions:ImportOptions}
 | {kind:"generated-body", bodyId:string, revision:u53, geometryIdentity:string}
ImportOptions = {previewRepairBudgetMm:positive number|null,
 targetDeflectionMm:positive number|null}
MeshSettings = {targetDeflectionMm:positive number|null,
 selectedDeflectionMm:positive number, angularRad:positive number,
 previewRepairBudgetMm:positive number|null}
AnalysisMesh = {version:"final-f64-v1", meshIdentity:string,
 sourceProof:"planar-final-v2"|"analytic-generated-v1",
 settings:MeshSettings, sourceLimits:SourceLimits}
SourceLimits = {profile:"planar-final-v2", maxSourceBytes:u32,
 maxSourceRecords:u32, maxDecimalCharacters:u32,
 minimumDecimalExponent:i32, maximumDecimalExponent:i32,
 minimumExponent:i32, maximumExponent:i32, maxFinalTriangles:u32,
 maxDirectBoundMm:number, cornerSnapDistanceSquaredMm2:number}
 | {profile:"analytic-generated-v1", maxFinalTriangles:u32,
 minimumNonzeroMagnitude:positive number, maximumMagnitude:positive number}
AnalysisRef = {workerGeneration:string, meshIdentity:string,
 source:Source, derivationIdentity:string}
```

`sha256` is 64 lowercase hexadecimal characters. `u64decimal` is a canonical
base-10 string for `[0,2^64-1]`, preserving token/session precision when emitted
as JS metadata. It must equal the corresponding existing frame identity integer.
All nullable fields above are present; omission is invalid. Existing v1 optional
field omission rules remain unchanged. Bounded source class/version/proof pairs
are exactly LINE/plane / 2 / `planar-final-v2` and generated / 1 /
`analytic-generated-v1`. Unknown source proof still has an `AnalysisMesh` identity.
Source/mesh identities are selectors bound to owned evidence, not certificates
by themselves. Identity equality never substitutes for the buffer checks below.

Generated/vendor analysis adds `sourceQualification`; collision results
add `queryQualification` alongside the unchanged raw Parry observation.
`containedLeft` means left material contained in right, and conversely for right.
The backend/worker authenticates evidence against `Binding`; a caller boolean or
unvalidated JSON assertion cannot produce qualified containment.

### Producer values, worker-owned references and transport

The following are the **active** executable shapes. v1 producers/consumers retain
their existing exact keys.
`AnalysisHandle` is an opaque WASM class with private Rust-owned immutable
buffers/provenance and proof data; no public constructor, JSON deserializer,
mutation method or caller-supplied bound can construct one.

| Boundary | Exact addition / result ownership |
| --- | --- |
| Direct `analyze_generated_body` | Existing direct generated fields plus `analysisMesh:AnalysisMesh`, `sourceQualification:SourceQualification`, `analysisHandle:AnalysisHandle`. No `geometryIdentity`, `analysisRef` or wrapper. |
| Direct `read_step_model` | Root stays `{mesh,bounds,vendorStep}`. Add the same three fields **inside** `vendorStep`. Existing `sourceSha256` is only the provider's binding label until the worker's byte check. |
| Worker generated result | `{revision,generatedAnalysis}`; generatedAnalysis is the direct value **without** `analysisHandle`, plus worker-owned `geometryIdentity` and `analysisRef:AnalysisRef`. |
| Worker STEP result | Existing `{revision,mesh,bounds,vendorStep}`; vendorStep excludes `analysisHandle`, adds `analysisRef:AnalysisRef`. |
| v2 collision operand | Existing `modelId`, exclusive STEP/generated identity, `importOptions`, `rowTransformF64`, plus required `analysisRef:AnalysisRef` and `analysisMesh:AnalysisMesh`. These are selectors/expected metadata, never proof input. |
| New qualified provider query | Direct `{observation:<unchanged MeshObservation>,queryQualification:QueryQualification}`. Worker emits `{revision,collision:observation,queryQualification}`; never nest the qualifier in the raw Parry union. |

V2 query metadata has exactly `{left,right,clearanceThresholdMm,
requestedAccuracyMm,qualificationLimits:Limits}`. Optional accuracy is explicit
null at this version. Result `limits` must equal that entire effective request;
requested caps may reduce, never exceed, the supported profile. Operand source
identity matches its reference, after canonicalizing existing omitted import
options to null defaults. The alternate STEP/generated identity key is absent.
`analysisMesh` must equal the retained descriptor, including source limits;
proof class/version and accounting are never inferred from caller JSON.

Generated `analysisErrorBoundMm` and vendor
`accuracy.analysisErrorBoundMm` equal the bounded **asset-frame** total, otherwise
null. The worker does not take these numbers back from a collision caller.
Existing source-face/occurrence records and array ordering are retained exactly;
a generated support/index is never interpreted as a STEP entity identifier.

`AnalysisMesh.settings` records both requested and selected settings. The
requested default/null selects 0.1 mm; generated selection is `min(0.1,target)`.
Vendor selection reports the actual importer selection, including its precision
policy, rather than assuming that same formula. `angularRad` is the actual
emitter setting; generated `previewRepairBudgetMm` is null. Nullable import
options in `Source` are the canonical expansion of existing omitted defaults.

`meshIdentity` is an immutable provider-issued generation token prefixed
`final-f64-v1/`, unique for that module lifetime, not a source hash or a numerical
certificate. Together with a fresh never-reused `workerGeneration`, it identifies
one exact final buffer/provenance/setting/proof object. The same tuple may be
reused only while that immutable object remains owned. Triangles, f64 bit patterns
(including signed zero), u32 tags, source/occurrence catalogues, selected settings
or source proof/caps changing requires a new identity. A token is not a content
hash: the opaque owner and bitwise comparison supply the binding.

The worker obtains the handle only from its own provider call. For STEP it first
checks immutable byte SHA against the request; for generated input it uses the
existing captured assembly/body/revision validator and derives the complete
prepared-body identity itself. Its owned entry binds the handle to that `Source`,
`AnalysisMesh`, full source report, scope and derivation identity. Vendor derivation
identity is `vendor-step-analysis-v2/` plus the existing
`derived_step_cache_key(SHA,options)` suffix; generated derivation identity is its
`m1/generated-analysis-v2/<prepared-body SHA>` plus requested deflection's exact
f64 bits (or `default`) and `analytic-generated-v1`. Source profile/caps and mesh
settings are additionally compared as typed fields, not inferred from a prefix.

The current parent (`2dd3b87d9`, unchanged relevant transport through `ec4712647`)
transfers generated/vendor result buffers in `result_transfers`; they detach in
the worker. Runtime's `ModelMeshCache` retains the validated **full** vendor asset;
its pending/task-token/LRU rules remain the delivery authority. `WorkerQueue`
currently owns jobs, not retained source certificates. Add only a bounded owned
sidecar map there, not a replacement cache framework. Keep the provider's Rust
immutable final buffers/handle before transferring the existing JS output arrays;
never retain a transferred typed-array view as proof storage. This retains the
same mesh, not another tessellation. Metadata/source hulls needed for proof stay
with the handle; they are not reconstructed from a caller's report.

Before invoking a qualified query, look up each reference in that worker's map;
match its scope, source kind/identity/options, full descriptor and provider
handle. Compare the returned query arrays **bit-for-bit** and tags/census against
that handle, with the 64 MiB pair check before allocations. Detached, zero-length,
SharedArrayBuffer-backed or concurrently mutable input is invalid; worker-owned
Rust copies are the only views queried. Runtime already sends copies, preserving
its immutable cached asset. Reject a different mesh with the same token. The
provider prepares the world f64 view once for Parry and verifier, checking exact
row rounding against the retained local view; neither reimports nor remeshes.
Provider source placement is already baked in; physical row applies once here.

A page may send references and arrays, never `analysisHandle`,
`sourceQualification`, world bounds, hull eligibility or containment evidence.
Reject such extra query keys. The worker constructs provider context from the
existing frame snapshot and its own generation; it never uses a second snapshot
supplied in query JSON. Binding includes the complete snapshot, both model IDs,
source/mesh/settings/caps descriptors, physical rows, threshold and requested
accuracy. `snapshotRevision == snapshot.revision`; each reference/mesh identity
and generation must agree. Generated source revision equals the frame revision.
Immutable STEP evidence can cross revisions only for the same source/options and
scope; each query is newly bound to the current captured snapshot. Existing
Runtime checks before dispatch/after await and Host reply/frame checks remain
mandatory. A mismatch is a rejected reply, not a supported band from another job.

### Ownership lifetime and Rust seam

New preparation results are staged until the existing post-kernel worker yield
and cancellation check succeeds. Only then may the worker publish a reference
and retain its handle. Cancellation, stale settlement or post/transfer failure
removes that staged entry and frees its handle. It cannot replace/remove a newer
task's entry. Query completion is checked again before publication; a cancelled
query does not invalidate another live job's immutable asset. Pin both handles
for the duration of the synchronous query and post-call liveness check.

The sidecar uses the existing delivery bound of 80 retained entries and the
existing 64 MiB bound as its aggregate retained-byte ceiling. Count all owned
buffer capacities and proof/catalogue storage conservatively; provider must
report that cost internally. Eviction drops retention; pinned jobs finish, then
release. Cloning a JS reference is not a second Rust ownership lease: free the
opaque object only after the last pinned owner releases it. An oversized object is not retained and yields `evidence-unavailable`
on a later query. A missing/evicted reference or restarted worker may still run
the existing raw observation path with all qualification states unknown; it
cannot resurrect evidence from page JSON. Re-prepare explicitly through existing
read/analyze operations (re-verifying bytes/prepared input), then obtain a new
reference and supersede old rows under existing token checks. Runtime must not
loop on a ready full-asset cache entry whose evidence lease is unavailable:
perform at most one explicit preparation per missing asset/current batch,
joining its existing pending task and replacing only under its task token.
A second miss stays unknown; basic checks do not refine automatically. Scope/worker close
frees entries; a replacement generation rejects old references. Refinements have
distinct settings/identities and do not mutate a coarser live entry.

On an unavailable-handle fallback, `Binding` is the request's expected binding,
not an authenticated proof: mesh distance, both source bounds, contact and
containment are all unknown with `evidence-unavailable`. Other malformed source/
mesh/descriptor/row/cap assertions are errors. There is no transparent rebuild
inside a pair query and no repeat refinement loop hidden in basic checks.

The narrow backend API is implemented in `cad/monstertruck/src/qualification.rs`
as `boardstudio_cad::qualification`, without widening existing declarations.
Signatures (Rust-owned fields private):

```text
pub struct PreparedAnalysisMesh { /* private: owned final emission + sealed evidence */ }
pub fn prepare_vendor_analysis(&ImportedStep) -> Result<PreparedAnalysisMesh, Error>
pub fn prepare_generated_analysis(&ExactModel, Option<f64>)
    -> Result<PreparedAnalysisMesh, Error>

pub struct PreparedGeneratedFaceOccurrence {
    pub source_face_index:usize, pub solid_occurrence_index:usize
}
pub struct PreparedGeneratedView<'a> {
    pub emission:&'a GeneratedAnalysisMesh,
    pub face_occurrences:&'a [PreparedGeneratedFaceOccurrence]
}
pub struct PreparedVendorView<'a> {
    pub positions_mm_f64:&'a [[f64;3]],
    pub triangle_face_occurrence_indices:&'a [usize],
    pub source_faces:&'a [StepSourceFace],
    pub face_occurrences:&'a [StepFaceOccurrence]
}
pub enum PreparedAnalysisView<'a> {
    Generated(PreparedGeneratedView<'a>), Vendor(PreparedVendorView<'a>)
}
impl PreparedAnalysisMesh {
    pub fn export_view(&self) -> PreparedAnalysisView<'_>;
    pub fn analysis_mesh(&self) -> &AnalysisMesh;
    pub fn source_qualification(&self) -> &SourceQualification;
    pub fn retained_storage_bytes(&self) -> Option<usize>;
}

pub fn verify_analysis_pair(&PreparedAnalysisMesh, &PreparedAnalysisMesh,
    rows:[[f64;16];2], limits:Limits) -> QueryEvidence
```

`AnalysisMesh` and `SourceQualification` are typed Rust counterparts of the
shapes above, not JSON certificate inputs. `analysis_mesh()` includes the
sealed selected settings and effective source limits. `QueryEvidence` is the
typed numeric/source/contact/containment report; the provider adds the
worker-supplied `Binding`. Errors use the existing textual error boundary.

These are read-only declarations in `cad/monstertruck/src/qualification.rs`; no
existing member visibility changes. `GeneratedAnalysisMesh`,
`StepSourceFace` and `StepFaceOccurrence` are already public. A returned view
borrows the prepared owner immutably; it cannot outlive that owner. Constructing
or copying a view/report does not construct `PreparedAnalysisMesh` or sealed
evidence. There is no public mutator, deserializer, certificate constructor or
source AST/trace access. Preparation alone creates the opaque owner.

Generated preparation uses the shared final emitter **once**, proves its retained
trace before dropping it, and moves that SAME `GeneratedAnalysisMesh` into the
owner. The borrowed emission supplies f64 positions, tags, source faces, selected
deflection/angular settings, bounds, closure metadata and existing f32 display
buffers. Preparation also owns and seals the generated occurrence catalogue:
`source_face_index` resolves each source face and `solid_occurrence_index` is its
solid index, matching today's direct DTO. The provider serializes this borrowed
catalogue rather than reconstructing unchecked tags/catalogues. It never calls
`mesh_profile_for_analysis` or `generated_body_analysis_data` again for this
result. The legacy emission's optional bound is not the v2 authority: derive the
v2 `analysisErrorBoundMm` from `source_qualification()` exactly as specified below.

Vendor preparation copies the already FINAL `preview_positions_f64`,
`triangle_faces` and both catalogues from the supplied `ImportedStep` into its
sealed owner, preserving all f64 bits and index order; it does not reimport or
remesh. Its export view supplies exactly those retained analysis arrays and
catalogues. Keep the same `ImportedStep` alive while serializing existing display,
line, precision and repair metadata through its existing getters. The descriptor
binds that import's selected settings. Direct vendor `accuracy.analysisErrorBoundMm`
is likewise taken from the sealed source report; other repair fields retain
their separate existing meanings. Neither producer substitutes a display mesh.

In `cad/wasm`, borrow the prepared export view/descriptor/report to construct the
existing direct payload plus the v2 fields. Copy slices into independently
owned JS typed arrays (`Float64Array::from`/`Uint32Array::from`, checking each
usize-to-u32 conversion); flatten vendor xyz without changing f64 values. Copy
catalogues/settings/report into JS objects. Do not expose a view into WASM linear
memory. After these borrows end, move the SAME prepared owner into the new opaque
`AnalysisHandle`; no second retained mesh is created. Transfer/detachment affects
only JS output copies, not the owner's immutable allocation. A serialization
failure drops the staged owner and cannot publish an evidence reference.

`retained_storage_bytes()` is a conservative checked sum for the opaque owner's
entire retained allocation: vector/string **capacities**, catalogue/descriptor/
report storage, any retained exact-number/proof heap and the owner itself. Count
the full generated emission, including retained f32 buffers, not just f64 lengths.
Shared allocations must not be omitted (conservative double counting is allowed).
Overflow or unaccountable storage returns `None`, so the worker must not retain
it and later qualification is `evidence-unavailable`. Temporary JS transfer
copies are outside this retained cost; their separate existing transfer cap still
applies. The read-only WASM handle method
`AnalysisHandle.retainedStorageBytes(): number|null` exposes this cost internally
to the worker (checked u53 conversion, null on failure), without adding a direct
payload field or accepting a caller override. The worker calls it before cache
admission. This is a retention ceiling, not a peak-memory or timing guarantee.

The provider-owned `AnalysisHandle` wraps this type. WASM export
`query_qualified_collision_mesh_pair(buffers,metadata,context,leftHandle,rightHandle)`
borrows those opaque handles; strict `buffers` keys are `leftPositions`,
`leftTags`, `rightPositions`, `rightTags`, with the existing typed-array kinds.
`context` is `{snapshot,workerGeneration}` built by the worker. No JSON proof
constructor is exported. Keep the existing raw query export and observations.
Existing `ImportedStep` getters suffice; do not widen its fields, generated AST/
trace visibility, `ReadStepAsset::import_bytes`, `generated_body_analysis_data` or
`TriangleSoupMesh`. The existing importer facade and export surface are unchanged.

### Source accounting and final geometry

`directBoundMm` is a symmetric Hausdorff upper bound from the complete nominal
source surface set to FINAL exported f64 triangles/tags, after polygon fans,
source placement/mirroring and render filtering. Missing source faces/occurrences
or incomplete proof return unknown. That unknown is a qualification result, not
a display refusal: incomplete geometry stays visible with an explicit
diagnostic and may be queried approximately, but it cannot qualify the whole
source or establish whole-model clearance. The first vendor class is limited to
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
The generated cylinder candidate frozen at `5bc56d35bc77` is **unaccepted**:
f64 `edge.oriented_curve()` inversion before exact source extraction and discarded
cap-trim error invalidate trusting its output. Extract raw retained source knots,
weights and orientation before any f64 preprocessing; apply orientation in exact
arithmetic, and carry complete source/cap trim contributions into the symmetric
bound. The corrected private `b03cc0951dfe4c91f7c8fb395e8dc218a656d452` has root acceptance
for integration after its raw-knot/exact-reversal and cap accounting correction,
and that proof now backs the active producer for the supported classes. The
fixture's generated bounded examples are activation oracles for that corrected
class, never trusted output from `5bc56d35bc77`. A class the proof cannot
support stays unknown (`source-proof-unavailable`). Cones, nonplanar trims
and unproved/open seams remain unknown.

`sourcePrecisionMm` is the authenticated declared attachment allowance, zero for
generated inputs without a declaration. It does not describe manufacturing
accuracy. `totalBoundMm` outwardly encloses `directBoundMm+sourcePrecisionMm`.
On activation, existing `analysisErrorBoundMm` equals this total or null for
unknown. Charge it once per operand. A direct proof can absorb conversion,
repair and meshing error; never add those terms again. It does not separately
measure repair movement: unmeasured `maxRepairDisplacementMm` and per-face
`repairDisplacementMm` stay null. 007's actual-movement ledger and selected-budget
enforcement remain required separately; this contract does not settle 007.
Null measured repair is permitted for a bounded collision source report only
when its authenticated direct proof includes all actual conversion/repair
movement. Selected allowance and sampled support residuals cannot fill it in.
A staged proof missing an actual movement bound stays unknown. The accounting
literal above requires every source trim/cap contribution and actual final
numeric contribution, not an assertion that each contribution is zero.

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

### World accounting, effective caps and assessment examples

For `scaled-local-plus-row-rounding`, world direct and precision enclose their
asset values times `normUpper`; total encloses those two plus the independently
measured `transformRoundingMm`, and conservatively encloses asset `totalBoundMm`
times that norm plus rounding. All sums/products/conversions are outward. For `direct-world`, direct already encloses final
world geometry including row rounding; total adds only world precision, and the
rounding field is diagnostic. Norm encloses the actual affine operator norm;
zero is invalid for a nonsingular row. Contact inflation equals the two world
totals, not the local asset totals. No sampled residual or unknown repair is
substituted into either calculation.

Effective planar source defaults: 2 MiB source, 10,000 records, 80 decimal
characters, decimal exponents [-64,64], nonzero f64 magnitude [2^-128,2^128],
4,096 final triangles, direct admission cap 1e-8 mm and corner-construction
squared threshold 1e-16 mm². The corner threshold selects rational construction;
complete coverage and independently measured displacement authenticate the bound.
Generated corrected-source defaults are 10,000 final triangles and nonzero
retained scalar magnitude [1e-100,1e12]. This does not widen the query domain:
query defaults are 4,096 triangles per side, 100,000 total visited node pairs,
2,048 exact pairs (seed included), 8 contact-box pairs, exponents [-128,128],
and requested numerical width 1e-10 mm. Bounds/counters/settings are explicit;
no hidden limit, construction threshold or profile change can reuse an identity.
A different effective limit needs a matching request/response/cache identity.

The v2 assessment path consumes the validated `QueryQualification` and expected
request binding, replacing today's `query_numerical_error_bound_mm:Option<f64>`
and `containment_ruled_out:bool` arguments to
`assess_subject_collision_observation`. Neither caller-controlled argument can
activate v2. Host validates the entire qualifier against its request before
Runtime's post-await snapshot check and assessment; raw observations/provenance
remain available independently.

Runtime derives the source interval from `meshDistance` and world totals with
outward arithmetic, not by centering a guessed error on the raw Parry distance.
For an explicit accuracy a, require `(U-L)/2 + eLeft + eRight <= a` by exact/
outward comparison before declaring a supported band. With null accuracy there
is no implicit 0.01 mm demand. The physical threshold is tested separately:
upper < threshold supports a violation; lower >= threshold supports clear only
with authenticated containment ruled out. Otherwise relation stays uncertain.
A contained state supports material overlap, not a positive solid-clearance
claim. A surface-intersecting observation by itself cannot bypass source budget.
Broad contact boxes may accompany a scalar clear result, but never certify the
raw witness as a unique point within the scalar accuracy. Unknown contact state
keeps the overall fully qualified finding uncertain; preserve any computed
scalar interval as diagnostic. Unknown containment prevents clear, not an already
proved clearance violation. Parity/contained evidence remains unavailable until
its independent global correspondence/eligibility proof exists.

The fixture authors exact plane-plate pair and capacitor/mixed pair expectations,
plus the corrected generated R1 cylinder budget cases. A 0.01 mm selected
cylinder deflection with a roughly 0.015385 mm source bound cannot satisfy
0.01 mm pair accuracy even against an exact plane. Coarse roughly 0.058824 mm
and the corrected adversarial roughly 0.48205 mm bound must include full caps.
Finer targets authorize another on-demand proof, not an assumed smaller band;
return unknown until its actual result meets the full budget. The adversarial
cap case is a private retained-source/trace mutation, not a `PreparedCaseIR`
producer recipe: its immutable helper/test reference is pinned to corrected
`b03cc0951dfe4c91f7c8fb395e8dc218a656d452` in the same fixture. That helper retains
the actual 0.01 mm emission, changes a shared raw NURBS trim with the listed
asymmetric knots, and uses opposite raw orientations on cylinder/cap. Extract
raw knots/weights before exact reversal and include the complete cap error.
Producer acceptance uses the ordinary supported recipe; a consumer may test the
adversarial expected report shape, but cannot manufacture a trusted handle or
claim that this private mutation is a public producer input. The capacitor's
0.01 mm source-precision floor alone also prevents a 0.01 mm pair claim when
other contributions are nonzero. Complete retained but unsupported mixed
presentation geometry keeps source/repair qualification unknown. Unretained
presentation geometry (including styled trimmed curves outside a cuboid) likewise
keeps qualification unknown and never yields a bounded partial asset. Under the
current vendor-preview policy it must not by itself refuse the preview: ticket 016
requires safely usable surviving geometry with an explicit incomplete-model error.
No whole-source qualification or whole-model clearance follows from it. Provisional
`14cde` is not positive evidence. Corrected cuboid ledger
`05b00ce75d3ef9f688c219c92d9c1e227004c8cd` is accepted for its bounded exact-zero
movement class: its existing `maxRepairDisplacementMm:0` diagnostic is allowed,
while `analysisErrorBoundMm` remains null. A zero repair ledger is not a source/
mesh certificate. The fixture separates complete unsupported geometry with null
repair from incomplete presentation errors. These are repair-only ImportedStep
metadata checks separate from bounded v2 producer expectations;
this does not broaden 007 or v2.

### Validation and identity

Reject nonfinite/negative/reversed intervals, invalid indices, empty reasons or
regions, over-cap/truncated boxes, overlapping slab endpoints, invalid affine
rows, inconsistent contribution sums, stale revisions and mismatched mesh/row/
algorithm/settings identities. The shared examples and invalid mutations
pin these cases without changing v1 expected keys or copying its prepared input.

Certificate/cache identity binds source SHA or complete generated identity,
revision/snapshot, FINAL buffers/provenance, proof versions, physical rows,
selected repair/deflection settings, query algorithm and every limit above.
Assessment also binds requested accuracy and physical threshold. Use existing
cache/token machinery, not new hash manifests. Coarse output cannot satisfy a
tighter request; cancellation or changed revision prevents publication. Active
transfer limits and both producer/consumer tests remain required for the v2
wire. The private proof is exposed through the qualification path, not as a
frontend certificate schema.


### Executable shared activation fixture

`qualificationV2.activation` contains complete descriptor/ref/binding/qualifier
examples and producer key sets exercised by the producer and consumer checks.
Its direct-generated recipe copies `/directPayload` then replaces only the listed top-level fields;
it reuses `/preparedBody` and never duplicates that canonical input.
`analysisHandle` has
no JSON example: the real producer supplies it, the worker retains it, and only
its field name/type is asserted. Vendor examples use original asset bytes and
the existing catalogues, checking the four-solid/30-face census rather than
inventing a tessellation count. Array/face order remains free, but every actual
reported tag and catalogue entry is sealed in the handle. The `readOnlyExport`
recipe pins how both producers serialize the same owned emission before moving
it into the handle, including retained-storage refusal.

The canonical plane query names triangles `[2,0]`: after the exact fixture rows,
these are top Z=2 and bottom Z=3, with a feasible common-XY witness and 1 mm
distance. The old `[0,0]` names bottom Z=0 and bottom Z=3, whose exact distance
is 3 mm, so it cannot seed a 1 mm upper bound. `canonicalSeedCheck` pins an
independent geometry check: transform the actual named canonical vertices with
exact arithmetic, combine the parallel-plane lower bound with a feasible vertex
upper bound, then check raw witness membership. Actual producer tests resolve
indices from their own emitted arrays/provenance and independently validate the
seed; they never assume canonical triangle ordering.

Pair cases name descriptor/ref examples and exact rows; expected bounds are
independently authored conservative oracles, not certificate inputs. Consumer
shape tests may materialize this JSON; authentication positives must register
real provider-owned handles. Mutation paths apply to a named complete example;
`$nonFinite` is materialized as NaN only in a JS negative test. Detach/shared-buffer
and cancellation/eviction cases are runtime actions, not JSON-shaped certificates.
Both producer and consumer tests use the same examples, enforce exact keys and
check unknown/error outcomes as well as supported reports. No consumer test may
make a forged handle from the expected numeric source report.

Fixture generation tokens are illustrative labels. Actual producer/worker tests
bind each label to the real issued handle/reference, then enforce the same
relations and exact-key shapes; a hardcoded label can never mint evidence.
