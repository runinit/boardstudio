# Approved additive owner seam for 007 movement

Root approved these additive declarations after checkpoint `cde92885`; implementation follows this contract, not an implicit certificate. Collision v2 and
its fixture do not change. The existing 34-file preview route continues unchanged
until its complete movement route is proved; an unsupported report is never zero.

## Exact proposed declarations

In `monstertruck-io/src/step/load/convert/repair.rs`, re-exported by the existing
public `convert` module (all fields private, no `Clone`, serde or public constructors
other than the source parser):

```rust
pub struct StepRepairSource { /* original bytes, parsed numeric/domain authority */ }
pub struct StepRepairConversion { /* actual SourceBoundStepSolid and witnesses */ }
pub struct StepRepairOperation<'a> { /* borrowed immutable witness */ }
pub struct StepRepairCurveChange<'a> { /* private captured finite pair */ }
impl StepRepairSource {
    pub fn from_step_bytes(bytes: &[u8]) -> Result<Self, LoadError>;
}
impl Table {
    pub fn to_source_bound_solid_with_repair_checked(
        &self, solid_id: u64, representation: Option<u64>,
        original: &StepRepairSource,
    ) -> Result<StepRepairConversion, LoadError>;
}
impl StepRepairConversion {
    pub fn source(&self) -> &SourceBoundStepSolid;
    pub fn operations(&self, shell: usize)
        -> impl Iterator<Item = StepRepairOperation<'_>>;
    pub fn complete_bound(&self, shell: usize) -> Result<f64, &str>;
}
impl StepRepairOperation<'_> {
    pub fn stage(&self) -> &'static str;
    pub fn curve_change(&self) -> Option<StepRepairCurveChange<'_>>;
    pub fn source_ids(&self) -> (u64, Option<u64>, Option<u64>); // solid, face, edge
    pub fn bound(&self) -> Result<f64, &str>;
}
```

Root additionally approved the exact new `StepRepairCurveChange` view and getter
(no constructor, Clone, serde or existing visibility change):

```rust
impl StepRepairCurveChange<'_> {
    pub fn curves(&self) -> (&Curve3D, &Curve3D);
    pub fn domains(&self) -> ((f64, f64), (f64, f64));
}
```

The actual selected BEFORE interval is captured in the SAME conversion before
cut/relabel/master replacement. Directed domains retain sense through inversion.
Every actual mutation has its own immediate pair and immutable stage identity;
consecutive pairs must chain structurally, and the last AFTER must equal the
actual emitted owned carrier. An early BEFORE cannot be paired with a later final
AFTER as an individual mutation. Missing/ambiguous finite selection gives None and
Unsupported, never a full-support or endpoint reconstruction. Some is captured
data, not original selection proof. Numeric conversion and earlier master choice
remain separate. Kiwano's actual alternate-master route remains Unsupported.
Carrier/control/knot capacities are charged before witness clones; failure/caps
make the report incomplete without changing preview admission.

`representation: None` means the existing without-connectivity route, not automatic
precision discovery. IDs must resolve in the captured original. The table must be
identical to that source except authenticated conical angle-unit normalization;
that normalization retains both written units/angle and resulting carrier, and
numerical error is accounted separately. Any other mismatch makes the new report incomplete without changing preview admission.
Existing APIs and private module visibility do not change.

In `monstertruck-meshing/src/tessellation/repair.rs`, re-exported by the existing
public `tessellation` module (private fields, no public witness constructor):

```rust
pub struct ShellRepairMesh<'a, C, S> { /* borrowed actual input; owned final mesh/trace */ }
pub struct MeshRepairOperation<'a> { /* borrowed immutable witness */ }
pub fn robust_cshell_triangulation_with_repair_report<'a, C, S>(
    shell: &'a CompressedShell<Point3, C, S>, options: TessellationOptions,
    closed_axes: &[(usize, usize, (f64, f64), Vec<usize>)],
) -> ShellRepairMesh<'a, C, S>
where C: PolylineableCurve + ParameterBoundary2D<S>, S: RobustMeshableSurface;
impl<C, S> ShellRepairMesh<'_, C, S> {
    pub fn input(&self) -> &CompressedShell<Point3, C, S>;
    pub fn mesh(&self) -> &CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;
    pub fn fallback_faces(&self) -> &[usize];
    pub fn operations(&self) -> impl Iterator<Item = MeshRepairOperation<'_>>;
    pub fn complete_bound(&self) -> Result<f64, &str>;
}
impl MeshRepairOperation<'_> {
    pub fn stage(&self) -> &'static str;
    pub fn indices(&self) -> (Option<usize>, Option<usize>); // face, shared edge
    pub fn bound(&self) -> Result<f64, &str>;
}
```

Root also approved new opaque `MeshRepairDomainChange<'a>` through the existing
tessellation module (all fields private, no constructor, Clone or serde):

```rust
impl<C, S> ShellRepairMesh<'_, C, S> {
    pub fn domain_changes(&self) -> impl Iterator<Item = MeshRepairDomainChange<'_>>;
}
impl MeshRepairDomainChange<'_> {
    pub fn stage(&self) -> &'static str;
    pub fn indices(&self) -> (Option<usize>, Option<usize>);
    pub fn uv_controls(&self) -> (&[Point2], &[Point2]);
    pub fn spatial_controls(&self) -> (&[Point3], &[Point3]);
}
```

These are full actual finite pieces before/after the SAME mutation, with stage,
direction and source association. Missing/empty/ambiguous or one-to-many
correspondence is Unsupported. Preserve piece boundaries; never zip unrelated
resampled controls. Native carriers/surfaces come only from the immutable actual
input. The actual final mesh is owned by the same trace. A preceding true curved
trim to polyline is a separate approximation term, Unknown without proof.

## Witness meaning and capture points

Bounds are symmetric whole-finite-domain movement in input source units (IO) and
the actual supplied shell coordinates (mesher). A witness owns the before/after
carriers and closed domain plus a proved covering correspondence in both directions;
equal endpoint residuals, identical samples and coincident supports are insufficient.
Original decimal-to-binary arithmetic is a separate numerical error, never a repair
and never silently ignored by a complete original-to-output claim.

Capture inside `EdgeCurve::parse_curve3d_with_bound` before LINE projection/chord,
`try_normalized_edge_bspline` before relabelling, `bounded_trim` before restriction,
and selected-master/PCurve trimming before a master or native interval is replaced.
Capture inside `face_surface_or_report` before `Surface::try_from`, retaining raw
surface coefficients and the finite support domain induced by original trims.
The conversion sink receives the same result that `shell_edges`/`shell_faces` emit;
the opaque conversion owner owns that result. No second conversion is evidence.

Capture the original chart pieces before `reconcile_closed_piece_branches`, UV
resampling, canonical-pole bridging and boundary-loop remapping; retain the actual
chosen piece/domain and final mesh in the same meshing call. Approximation from
ordinary curve/surface tessellation, fallback triangulation and f32 projection is
recorded separately, with Unknown approximation where unsupported. A change of the
represented trim region or carrier is repair even if implemented inside the mesher.
Changing chart labels is zero only with proved equality of the complete physical
domains; singular/periodic labels alone do not establish equality.

Each owner uses a closed private stage enum and append-only stage sequence. The
stage string is its fixed versioned diagnostic name. Each admitted route must
terminate every expected stage exactly once as unchanged, bounded or Unsupported;
an omitted stage poisons completeness. Original finite trim ambiguity is Unsupported.
The importer binds the borrowed mesher input to the actual normalized conversion
shell by exact structural equality, face/edge identities and orientations. It then
binds the *same* emitted mesh to the retained final arrays; a stale/alternate result
cannot lend a witness. Source mismatch, omitted stage and changed output controls
must fail. There is no AST accessor, caller-supplied certificate or shadow mesher.

Sum consecutive movements along each carrier/domain path; combine disjoint regions
by maximum. Face support/trim dependencies carry their preceding stage sums. Apply
authenticated source-unit and occurrence linear transforms once per occurrence,
excluding the common translation. Take the maximum over occurrences, not their sum.
Compare each complete occurrence path against its selected world-mm budget, or
`max(2 * authenticated placed precision, 0.02 mm)` by default. Numeric conversion
and mesh approximation remain separate terms; direct collision proof is not charged
again. Unsupported geometry may retain preview admission but cannot claim complete
repair certification or use a partial report to authorize an unmeasured change.

## Lifetime, cost and controls

Owners keep immutable witnesses until import validation finishes; borrowed views
cannot outlive them. Original bytes are copied once into `StepRepairSource`; reuse
the captured original source rather than parsing again per occurrence. Trace cost is
linear in source carriers plus actual changed domains, shared edge uses stored once.
Reject report construction before allocation beyond 32 MiB source bytes, 1,000,000
operation records, or 256 MiB retained witness storage; cap refusal cannot become a
certificate. These approved report-resource caps are not new preview admission limits. Check conservative owned allocation capacities before trace growth; cap/construction failure preserves the actual admitted preview and makes complete_bound Err. Never return a guessed bound. The mesher input lifetime is tied only to shell; options and closed_axes are not borrowed by its result. No sampling
or default tolerance increase may rescue an unsupported proof.

Use one literal finite LINE/plane STEP fixture in
`cad/monstertruck/tests/fixtures/repair-finite-domain.step` (now pinned: tetrahedron with original X LINE at Y=1/32 mm and emitted chord Y=0), read as the same bytes by
converter-owner tests and importer/actual Node-provider tests. Independently authored
finite carrier/domain controls must prove nonzero movement in both directions;
default, explicit, equality/rounding, metre, rotated/scaled occurrence and immutable
byte/provenance controls use it. Mutations omit a stage, change source identity, move
an interior control while preserving endpoints, and alter the actual emitted result.
Before activation replay the unchanged authoritative approved34 fixture, including
all positive source faces, occurrences/lines, existing bounds oracles and Panasonic
155-face surface-only Unknown. A compatibility failure is reported, not hidden by
raising a budget, dropping geometry or relabelling an unmeasured repair as zero.
