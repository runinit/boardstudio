# GVP necessity and Rust availability for STEP import validation

Date: 2026-10-18. Research only; no production code changed.

Question: is Geometric Validation Property (GVP) comparison actually needed for
STEP import validation in BoardStudio, or can we rely on aggregate reports +
invariants + budget gate? And if GVP is needed, can existing Rust libraries
provide it?

## Summary / recommendation

**Drop the GVP comparison.** Concretely: remove the naive `extract_gvp_volume_mm3`
heuristic, the `mesh_volume_mm3` helper, and the `gvp_*` fields on `StepAccuracy`.
Rely on the aggregate loss reports (`require_empty` / `require_lossless`), the
post-import invariant suite, and the single repair budget gate, which is what
BoardStudio already ships and what actually catches real import failures.

Revisit GVP only if a certification contract (LOTAR / COPS) explicitly demands
it; in that case implement it properly (real STEP GVP parsing with unit
conversion, area + centroid, not just volume) and treat it as an advisory report
(green/yellow/red), never as an import gate.

## 1. Does ISO 10303 require GVP for import acceptance?

No. ISO 10303 defines *geometric validity* as manifold B-rep — AIC 513
(elementary B-rep) / AIC 514 (advanced B-rep): a closed, oriented, manifold
shell with consistent geometry/topology incidence. That is the validity condition
the standard actually imposes.

GVP is defined separately by **ISO/TS 10303-1039** ("Geometric validation
property representation") and operationalised by the **CAx-IF Recommended
Practices for Geometric and Assembly Validation Properties v4.2**. It is a
*validation mechanism*, not a validity condition: the exporter computes
volume / surface area / centroid (solids) or area + centroid (surfaces) and
centroid + length (curves) and stores them attached to the product/shape; the
importer recomputes and compares. CAx-IF thresholds: volume/area/length green
< 1%, yellow 1–10%, red > 10%; centroid green < 0.1% of model size (space
diagonal), yellow 0.1–1%, red > 1%, with an absolute fallback for small models.

The CAx-IF conformance *tests* use GVP agreement as their acceptance criterion
when a file carries GVP, but the standard does not require a file to carry GVP,
and a file without GVP is not invalid. So GVP is "validate if present", never
"required for admission".

## 2. Do real vendor STEP files carry GVP?

In BoardStudio's actual input population: **effectively none.**

Survey of the repository's STEP corpus (ergogen vendor models, ABC dataset
fixtures, OCCT fixtures, choc fixtures, monstertruck fixtures) found zero files
carrying geometric validation properties (no `volume`/`area`/`centroid` value
representation items attached as GVP). Related but distinct property types do
appear:

- material properties (`density of part`, `material name`) in many ergogen
  models — not GVP.
- PMI validation properties (`number of annotations`, `number of views`) in
  some Siemens NX files — annotation VP, not geometric VP.

Conclusion: in BoardStudio's real inputs, GVP is absent, so a GVP comparison
almost never runs. The one implemented today (`extract_gvp_volume_mm3`) is dead
code in practice, and its heuristic ("VOLUME" substring + first number) is a
false-positive risk even when it does fire.

## 3. Are there Rust crates for STEP GVP parsing?

No. Nothing in the Rust ecosystem parses STEP geometric validation properties:

- `step-p21` / `ruststep` (and BoardStudio's `monstertruck-io` fork) are
  Part 21 *syntax* parsers only; geometric semantics are the consumer's concern.
- `monstertruck-io` ships aggregate loss reports (`EntityLoadReport`,
  `ShellLoadReport`) but no GVP parsing and no volume/area/centroid from B-rep.
- `brepkit` computes volume/area/centroid for its own solids, but is a
  different kernel and does not parse STEP GVP.
- No crate on crates.io mentions GVP / geometric validation property parsing.

Implementing real GVP parsing is nontrivial: walk
`property_definition` → `property_definition_representation` →
`shape_representation` (or `representation` with a `value_representation_item`
carrying `volume_measure_with_unit` / `area_measure_with_unit` / a point for the
centroid), resolve the unit conversion (STEP length unit may be inch/mm/m), and
handle per-solid / per-shell / part-level / instance-level and assembly-level
variants. None of that exists in Rust today; BoardStudio would own it.

## 4. What does OCCT actually do with GVP?

Advisory, not a hard gate:

- The basic `STEPControl_Reader` does not compute or compare GVP at all. GVP is
  read only through the XDE path (`STEPCAFControl_Reader`) and only when
  `ReadProps` is enabled — an opt-in.
- In CATIA/Creo-style import, the GVP check is reported as green/yellow/red in
  an *import validation report*; the import still succeeds. Status is a quality
  indicator for the user, not an admission gate.
- OCCT's actual import gates are: `PrintCheckLoad` / `PrintCheckTransfer`
  entity reports, tolerance-bounded repair (`read.maxprecision.*`), and
  `ShapeFix` healing — i.e. load/transfer reports + healing, plus optional GVP
  reporting. This matches the "aggregate report + budget + invariants" shape
  BoardStudio already uses.

## 5. Can we compute volume/area/centroid from monstertruck B-rep?

Only via the tessellation, not analytically:

- `monstertruck-meshing` provides `CalcVolume` for `PolygonMesh` and for
  `Solid<Point3, PolylineCurve, PolygonMesh>` — both operate on tessellated
  (polygon) surfaces. There is no analytic volume integration for NURBS /
  BSpline / cylinder / cone / sphere surfaces in monstertruck.
- BoardStudio's retained geometry is a `CompressedShell<Point3, …, Option<PolygonMesh>>`
  (the preview mesh), so any volume computation is mesh-based by construction.
  Computing "from B-rep" would still mean tessellating first.

## 6. Is the mesh volume comparison meaningful given tessellation error?

Marginally, and not as a gate:

- The mesh is tessellated at a deflection tolerance (default 0.1 mm). Volume
  error of a piecewise-linear approximation is second-order in deflection, so
  relative error is typically small (≪ 1%) for parts of ~100 mm. But it grows
  for coarse tessellation, small features, high curvature, and thin walls, and
  can exceed the 1% green threshold on legitimate imports.
- The comparison also needs unit conversion (STEP length unit → mm) and
  centroid/area, none of which the current code does; it only grabs the first
  positive number after "VOLUME".
- Because the embedded value is exact (exporter's native B-rep) and the
  computed value is tessellated, a "red" result can be a tessellation artefact
  rather than a real import defect. Gating import on it would create false
  rejections; reporting it would create noise. Neither is worth the cost given
  the corpus carries no GVP.

## 7. Keep / simplify / drop

**Drop.** Remove:

- `extract_gvp_volume_mm3` (naive text heuristic)
- `mesh_volume_mm3` (mesh tetrahedral volume, only used by GVP)
- `gvp_embedded_volume_mm3`, `gvp_computed_volume_mm3`, `gvp_volume_status`
  from `StepAccuracy`

Keep the aggregate reports, invariant suite, and budget gate as the import
admission criteria. If a future certification contract requires GVP, implement
it as a proper, unit-aware, multi-property (volume + area + centroid) parser and
report it as green/yellow/red metadata — advisory, never a gate — and consider
COPS only for LOTAR-class delivery.
