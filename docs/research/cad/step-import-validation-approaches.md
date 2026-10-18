# STEP import validation and repair: simpler approaches

Date: 2026-10-18. Research only; no production code changed.

BoardStudio (Rust CAD, Monstertruck kernel, no OCCT) currently certifies STEP
import through a six-package per-route accounting program (007): resource
accounting, original selection/master choice, raw spline normalization, surface
construction, mesher, terminal policy. Each new importer repair route adds
instrumentation and a proof. This note surveys what the ISO 10303 standard and
other STEP implementations actually do, and proposes simpler certification
routes that do not require per-route accounting.

## 1. What ISO 10303 defines as validation

ISO 10303 (STEP) is a data model, not a validation algorithm. Geometric
validity is defined by the application interpreted constructs (AICs):

- **AIC 513 / AM 1329 "Elementary boundary representation"** (ISO 10303-513,
  ISO/TS 10303-1329): a `manifold_solid_brep` composed of elementary geometry
  (lines, conics, planes, cylinders, cones, spheres, tori, B-splines) with
  **complete explicit topology**. "Manifold" means every edge is shared by
  exactly two faces and every vertex link is a cycle. This is the geometric
  validity condition the standard actually imposes: a closed, oriented, manifold
  shell with consistent geometry/topology incidence.
- **AIC 514 "Advanced BREP"** (ISO 10303-514) extends this to advanced
  (NURBS) surfaces and curves with the same manifold/topology requirements.
- **Geometric validation properties** are defined by **ISO/TS 10303-1039**
  ("Geometric validation property representation") and operationalised by the
  **CAx-IF Recommended Practices for Geometric and Assembly Validation
  Properties, v4.2** (2014). These are the *standard* validation mechanism:
  the exporter computes volume, surface area and centroid (solids) or area and
  centroid (surfaces) and centroid/length (curves) and stores them in the STEP
  file attached to the product/shape; the importer recomputes the same
  quantities from the imported geometry and compares.

  CAx-IF evaluation thresholds (section 4.11 of the RP):
  - Volume / area / curve length: green < 1% deviation, yellow 1–10%, red > 10%.
  - Centroid: green < 0.1% of model size (space diagonal of the bounding box),
    yellow 0.1–1%, red > 1%; absolute fallback (< 1 mm green, 1–5 mm yellow,
    > 5 mm red) for models whose space diagonal is below ~20 mm.
  - Assembly validation properties: number of children per node, and a notional
    solid centroid position to verify instance placement without reprocessing
    geometry.
- **Cloud of Points (COPS)** (section 6 of the RP): per-face sampling points
  stored in the file, recomputed and compared on import. This is the heavy,
  certified-delivery (LOTAR) option, not a default.

Key point: the standard's own validation is **post-hoc comparison of aggregate
invariants** (volume, area, centroid, counts), not per-conversion-route
accounting. The exporter and importer never agree on internal conversion
routes; they agree on the resulting geometry's bulk properties.

## 2. How other STEP libraries handle import validation

### Monstertruck / ruststep / step-p21 (Rust)

- **step-p21** (fork of ruststep) is a Part 21 *syntax* parser only. It
  deliberately provides no geometric semantics; validation is the consumer's
  concern. It exposes the entity table and typed hydration, nothing more.
- **monstertruck-io** (the crate BoardStudio vendors) already ships a typed
  loss-accounting layer that is exactly the "simpler" post-import check:
  - `Table::entity_report` (`EntityLoadReport`): counts records swallowed during
    table building, split into `refused` (typed Deserialize failure) and
    `shape_unexpected` (arity-guarded arm with no else). `require_empty()`
    turns any swallow into `LoadError::PartialLoss`.
  - `to_compressed_shell_reported` / `to_compressed_solid_reported` return a
    `ShellLoadReport` tallying kept vs. listed entities per `LossCategory`
    (Face, Wire, EdgeUse, Edge, Vertex, Representation, Solid) with a
    `LossReason` per loss (e.g. `FaceUnresolved`, `SurfaceRefused`,
    `DegenerateVertexLoop`, `EdgeCurveRefused`). `require_lossless()` turns loss
    into an error. Reports are `merge()`-able across shells.
  - `Table::solids_via_shape_relationship` exposes the part→solid hop losses.
  - The un-suffixed methods still exist and discard the report, so strictness
    is opt-in; in-gate fixtures are lossy by design (e.g. `DegenerateVertexLoop`
    is the correct outcome for a point boundary).
  - This is the idiomatic monstertruck validation: **one aggregate report per
    conversion, opt-in strictness**, not per-route instrumentation.

### OpenCASCADE (C++)

OCCT's STEP translator (`STEPControl_Reader`) follows a check-then-transfer
model:

1. `ReadFile` loads the model into memory (no translation).
2. `PrintCheckLoad(failsonly, mode)` reports invalid/incomplete entities by
   type or by message.
3. `TransferRoots` / `TransferOne` / `TransferList` perform translation, with
   `PrintCheckTransfer` reporting transfer failures per entity.
4. Translation parameters bound repair: `read.precision.mode` (file/user),
   `read.precision.val` (basic tolerance), `read.maxprecision.val` /
   `read.maxprecision.mode` (hard cap, preferred/forced), `read.stdsameparameter.mode`,
   `read.surfacecurve.mode`, `read.encoderegularity.angle`.
5. **ShapeHealing** (`ShapeFix`, resource-file sequences) runs after translation
   as a configurable, user-tunable pass.
6. **Validation properties** (area, volume, centroid) are imported and compared
   per the CAx-IF RP; Creo/PTC-style "Import Validation Report" computes mass
   properties and reports green/yellow/red.

OCCT does not do per-conversion-route accounting. Its validity story is:
entity-level load/transfer reports + tolerance-bounded healing + GVP comparison.
This is the industry norm.

### IfcOpenShell

IfcOpenShell validates at the **schema/instance level** (`ifcopenshell.validate`):
entity not abstract, attribute types correct, cardinalities correct, select/
enumeration conformance. Geometric validity is delegated to OCCT. There is no
per-route geometric accounting; validity is structural + delegated.

## 3. Validation approaches beyond per-route accounting

| Approach | Cost | What it catches | Standard? |
|---|---|---|---|
| **Aggregate loss report** (monstertruck `require_lossless`/`require_empty`) | O(entities) | Dropped faces/wires/edges/vertices, swallowed records, unresolved surfaces | monstertruck-native |
| **GVP comparison** (volume/area/centroid) | O(geometry), once | Geometry lost or distorted during import; standard thresholds | ISO/TS 10303-1039, CAx-IF RP 4.2 |
| **COPS** (per-face sample points) | O(faces × samples) | Per-face deviation/misplacement | CAx-IF RP 6 (LOTAR/certified) |
| **Tolerance-bounded repair + refusal** | O(geometry) | Over-budget repair, non-convergent healing | OCCT `read.maxprecision.*` |
| **Source-byte retention + SHA** | O(bytes) | Tampering/corruption in transit | BoardStudio already |
| **Post-import invariants** (face/edge/vertex counts vs source census, mesh nonempty, bounds sanity, watertightness) | O(mesh) | Incomplete conversion, empty mesh, absurd bounds | BoardStudio already (partial) |
| **Round-trip hash / geometry equality** | O(export) | Import+export not identity | Useful for export path (008), not preview |
| **Mesh coverage / deflection** | O(mesh) | Tessellation gaps | BoardStudio mesh bounds |

The per-route accounting in 007 is essentially a *refinement* of the aggregate
loss report: instead of "did the shell lose a face?", it asks "did this specific
conversion stage move geometry by how much?". That is a legitimate but much
more expensive question, and it is only needed for the collision-accuracy
certificate (014), not for import admission.

## 4. Simpler certification approaches

Recommendations for BoardStudio's stated goal ("import vendor STEP for
display/alignment/collision; preserve source bytes/geometry; apply bounded
repairs; reject over-budget/incomplete imports"):

1. **Use the aggregate reports that already exist.** Replace the six-package
   per-route instrumentation with:
   - `EntityLoadReport::require_empty()` after `Table::from_step_bytes`.
   - `ShellLoadReport::require_lossless()` after each `to_*_reported` conversion.
   - Refuse if either fails. This is one call per conversion, O(entities),
     and monstertruck already computes it.
2. **Add GVP comparison when the source file carries validation properties.**
   Most vendor STEP files from major CAD (SolidWorks, Creo, CATIA, Fusion)
   include them (CAx-IF RP mandates part-level attachment). Recompute volume,
   area, centroid on the imported mesh/B-rep and compare against the embedded
   values using the RP thresholds (1% / 0.1%). This is the standard, vendor-
   agreed acceptance test and requires no per-route instrumentation. Refuse or
   mark "yellow" on red.
3. **Keep tolerance-bounded repair with a single declared budget.** BoardStudio
   already uses `max(2 × declared precision, 0.02 mm)`. Keep that as the only
   repair gate; reject imports whose measured repair exceeds it. Do not try to
   attribute the repair to a specific route.
4. **Post-import invariant suite (cheap, deterministic):**
   - face/edge/vertex counts in the imported shell equal the source census
     (already tracked).
   - every source face has positive f64 mesh coverage (already).
   - bounds are finite and within a sane multiple of the declared precision.
   - mesh is nonempty; no NaN/Inf vertices.
   - original bytes hash matches (already).
5. **Reserve per-route accounting for the collision certificate (014) only.**
   The movement bounds needed for collision (max repair displacement) are a
   stricter, separate question. Keep the per-route work there, scoped to the
   routes that actually feed collision (not display/alignment).
6. **Defer COPS.** Per-face sampling points are only required for LOTAR/
   certified long-term archiving. For preview/alignment/collision they add
   O(faces × samples) cost for marginal benefit. Adopt only if a certification
   contract demands it.

This collapses six packages of per-route work into: one aggregate report call,
one optional GVP comparison, one budget check, and a small invariant suite. The
per-route accounting that remains is confined to the collision certificate,
which is the only consumer that needs it.

## 5. ISO 10303 on geometric validity and repair

- Validity is **manifold B-rep** (AIC 513/514): closed, oriented, consistent
  topology with geometry incidence. The standard does not require a specific
  repair; it requires the result to satisfy the manifold conditions.
- Repair is **not** standardised. OCCT's `ShapeFix`/`ShapeHealing` and the
  `read.maxprecision.*` parameters are implementation-defined; the standard
  only requires that the exchanged geometry's validation properties (GVP) match
  within agreed tolerances after exchange.
- The standard's acceptance criterion is therefore **GVP agreement**, not
  internal repair attribution. This is the strongest argument for replacing
  per-route accounting with post-import GVP comparison: it is what the standard
  itself prescribes as the acceptance test.

## Sources

- ISO 10303-42:2025 (geometric and topological representation), ISO 10303-513
  (AIC elementary B-rep), ISO/TS 10303-1329 (AM elementary B-rep),
  ISO 10303-514 (advanced B-rep), ISO/TS 10303-1039 (geometric validation
  property representation). https://www.iso.org/standard/91386.html
- CAx-IF Recommended Practices for Geometric and Assembly Validation Properties
  v4.2 (2014). https://www.mbx-if.org/home/wp-content/uploads/2024/05/rec_prac_gvp_v42.pdf
- NIST STEP File Analyzer User's Guide (validation properties overview).
  https://nvlpubs.nist.gov/nistpubs/ir/2016/NIST.IR.8122.pdf
- OpenCASCADE STEP Translator documentation (load/check/transfer, tolerances,
  ShapeHealing, validation properties). https://occt3d.com/dev/doc/overview/html/occt_user_guides__step.html
- monstertruck-io 0.4.1 source: `src/step/load/report/mod.rs`
  (`ShellLoadReport`, `EntityLoadReport`, `require_lossless`, `require_empty`,
  `LossCategory`, `LossReason`); `src/step/load/convert/mod.rs`
  (`to_compressed_shell_reported`, `to_compressed_solid_reported`).
- step-p21 / ruststep README (scope: Part 21 syntax only).
  ~/.cargo/registry/src/.../step-p21-0.1.1/README.md
- BoardStudio 007 ticket and repair-accounting notes:
  docs/plans/cad-backend-replacement/issues/007-strict-step-import.md
- BoardStudio monstertruck STEP research:
  docs/research/cad/monstertruck-step-support-2026-10-16.md,
  docs/research/cad/monstertruck-step-bounds-2026-10-17.md
- IfcOpenShell validation (schema/instance level, geometry delegated to OCCT):
  https://docs.ifcopenshell.org/autoapi/ifcopenshell/validate/index.html
- Creo Import Validation Report (mass-property-based green/yellow/red):
  https://support.ptc.com/help/creo/creo_pma/r12/usascii/data_exchange/interface/About_Resolving_Import_Validation_Failures.html
- CADIQ / LOTAR STEP validation context:
  https://www.cadinterop.com/en/your-needs/cad-qualification-and-validation/data-stability-testing.html
- Springer "Boundary Representation Models: Validity and Rectification"
  (manifold B-rep validity, repair complexity). https://link.springer.com/chapter/10.1007/978-1-4471-0495-7_23
- PDES Inc. STEP Application Handbook (validation properties, recommended
  practices). https://pdesinc.org/wp-content/uploads/2020/04/STEP-application-handbook-63006-BF.pdf
