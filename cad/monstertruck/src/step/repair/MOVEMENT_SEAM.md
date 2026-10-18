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

## Bounded implementation slice (2026-10-18)

The approved declarations above are implemented. Actual direct bounded-curve
conversion captures each selected finite cut/relabel and inversion, checks the
immediate structural chain and binds its last carrier to the actual owned output.
The importer has exact polynomial/rational whole-domain control proofs for these
captured Bspline/NURBS stages; the earlier original finite selection remains an
independent incomplete stage. Native LINE vertex-chord movement has a separate
original-decimal affine proof and exact world-mm budget comparison. Known stages
sum per occurrence/shell/edge and disjoint paths combine by maximum. A known
over-budget stage refuses even when unrelated authority is unsupported.

This is not complete 007. Projected LINE intervals, PCurve/selected-master and
alternate-master replacement, tiny-span raw spline normalization, and original
finite surface conversion still need their same-call original authority and
complete proofs. Mesher views capture actual pieces at branch/boundary/loop
changes, but concrete support composition, filled-region correspondence and
preceding true-curve approximation remain unproved. These reports stay incomplete;
the retained preview does not acquire a movement certificate. Kiwano's alternate
master remains Unsupported.

The initial IO source-storage estimate charges 1024 times original byte length
against the 256 MiB report cap before parsing/copying. Thus it can deliberately
leave reports incomplete well below the 32 MiB source cap; it does not restrict
preview admission. This conservative estimate and unsupported route coverage are
material limits of this slice, not evidence that admitted routes are complete.

The same-slice review correction retains the legacy tolerance precondition before
trace allocation, including empty inputs. A rejected aligned candidate rolls back
its own appended records before projection fallback; accepted earlier pieces and
conservative resource failures survive. Both finite certifiers share one private
outward-square-root conversion with the saved arithmetic order and diagnostics.
These corrections add no movement route coverage or preview tolerance policy.

## Projected native LINE: finite attachment operation

Root approved the existing curve view for the explicitly named
`native-line-attachment-projection-v1` stage. Its BEFORE is the actual finite
attachment `Line(p, q)` built from the SAME conversion's input vertices in source
sense order, with directed domain `[0, 1]`. It is NOT the original STEP LINE
support and NOT an inferred native scalar interval. AFTER is that call's actual
projected finite LINE over `[0, 1]`. A subsequent inversion is its own immediate
stage; structural chaining binds the last AFTER to the actual owned emitted edge.
The private capture retains original native support coefficients, the actual
binary native carrier and sense before projection, under the existing storage
charges. Source edge/use identities and immutable original source bind those data.
The stage name prevents generic consumers treating attachment-carrier data as
source-curve authority; native finite-selection and master stages remain Unsupported.

The importer independently derives the ideal projected attachment controls from
entity-local original decimal vertices/support, then bounds the complete affine
attachment-chord -> ideal-projected-chord operation in both directions. Numeric
ideal projection -> actual binary64 emitted controls is a separate retained term.
Unit and occurrence linear transforms apply once; common translation cancels.
The attachment correction enters the finite ledger once; the diagnostic owner
view does not charge it again. The opposite source-support -> vertex-chord branch
keeps its existing independent authority. An unproved native interval sense does
not invalidate the separately captured finite attachment operation or make that
native interval proved. Explicit budgets, exact equality and rounding boundaries
use exact rational squared comparisons against the selected binary64 allowance.

Root also approved refusal-only lower-bound enforcement when detailed traces are
capped. A private importer wrapper performs the actual conversion once while its
immutable input Table is still available. Per edge it authenticates original
support/vector/direction/vertex and point IDs, raw coefficients and sense against
that SAME input, binds the actual owned emitted LINE and both correct
VertexOnEdge attachments (edge, vertex, endpoint, indices and actual interval),
and checks exact ideal movement of those original attachment points. A strictly
over-budget lower bound refuses; equality does not. A non-refusal supplies NO
certificate, finite-selection proof or authorization. Resource failure/absent
domain captures remain incomplete; under-budget capped input keeps its existing
preview and Unknown asset bound. Unrelated padding, unsupported entities and
declaration ordering cannot erase a known over-budget operation. No cap, source,
allowance, constructor, public view or trait changes are needed.

The shared literal `tests/fixtures/repair-projected-line.step` extends the existing
finite-domain tetrahedron with authenticated precision and an exact 1/64 mm LINE
offset, selecting the real projected branch. The original approved fixture and
corpus are unchanged. Native and actual Node REDs preserve the genuine default
budget bypass at raw offset `0.020000000000000001`; its parsed attachment guard
accepts `.02`, while ideal geometric attachment movement exceeds that selected
allowance. Source numeric rounding remains separate. Full original trim/master,
PCurve, tiny-span raw normalization, surface and mesher-region coverage remain
pending; this finite attachment feature does not close 007 or clear 011 timings.

## Same-master planar spline-PCurve seam slice (root authorized after 4de2f735)

No new public declaration or existing visibility widening. The shared literal
`tests/fixtures/repair-planar-seam.step` uses the existing preview-tolerance rational
planar circle, with only its shell/model wrapper made a solid for the traced facade.
The first qualified class keeps the selected planar spline PCurve; alternate
masters, original finite selection and original surface conversion stay Unsupported.

In `try_trim_to_points`, mark each attempted candidate; rejected candidates truncate
only their own records, never conservative resource failures or earlier records.
`preview_seam_extended` retains the immediate original finite carrier `(s,e)`, the
ACTUAL shifted spline after `knot_translate(e-s)`, and ACTUAL concatenated carrier
`(s,e+(e-s))` in that invocation. The immutable
`same-master-planar-pcurve-seam-extension-v1` pair covers two separate pieces:
the original and its translated copy. Private records retain the join and shifted
net. A single affine map over the doubled domain is forbidden. Whole knot and
homogeneous-control algebra must cover both pieces, their one-sided seam limits
and the actual value selected at the seam; endpoint closeness is not periodicity.

Exact copying/translation/clamping preserves the geometric image. Its independently
derived ideal operation repair is zero only after full finite-piece coverage;
actual binary64 knot/clamping deviation is a separate measured numerical term,
never ordinary tessellation approximation. Missing/ambiguous maps or nonpositive
rational denominators are Unsupported. Plane coefficients must be authenticated
from the actual retained concrete plane, not sampled evaluations. Source plane
conversion and evaluation/render quantization remain separate and unproved.

`same-master-planar-pcurve-trim-v1` retains the resolved directed finite interval
immediately BEFORE active-trim installation and the SAME interval/carrier AFTER.
That view operation does not prove the preceding selection of the finite region.
The seam pair's full-domain AFTER may restrict to this actual interval only at
this named stage, with structural carrier equality and checked inclusion; no
general chain weakening. Inversion has its own immediate carrier/domain pair.
The last pair must bind the actual emitted edge, original IDs and unchanged master.

Recursive curve/surface storage is charged with checked capacities and bounded
traversal before retaining clones, including the shifted copy. Caps preserve the
legacy preview with incomplete coverage. Consecutive stages sum, disjoint seam
pieces max; source units/occurrence maps apply once. Numeric terms never authorize
unknown source/master coverage. The existing census's two TrackPoint alternates
are exactly `TP_Cap_Green_T430.step`; none of the twelve alternates gains proof.

The concrete extension proof requires clamped source endpoints: the active domain
must equal the outer knot extent. `try_concat::clamp` uses OUTER knot ends, so an
unclamped carrier cannot borrow this domain-preservation claim. The added actual
unclamped control selects the legacy alternate and stays Unsupported. No converter
change or preview refusal is introduced. Native/Node import controls close the
original one-edge fixture with an explicit chord and opposite face; the original
open-boundary import refusal is preserved. Plane coefficients are used only when
existing constructor equality authenticates recovery of the actual raw affine
points and identity plane transform; other plane records remain Unsupported.
