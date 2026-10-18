# 02: Add GVP comparison

Status: ready
Type: build

## What to build

Add Geometric Validation Property comparison to the STEP import path.
When a source STEP file carries validation properties (volume, area, centroid),
recompute them on the imported geometry and compare against the embedded values
using CAx-IF RP 4.2 thresholds:
- Volume / area / curve length: green < 1%, yellow 1–10%, red > 10%.
- Centroid: green < 0.1% of model size (space diagonal of bounding box),
  yellow 0.1–1%, red > 1%; absolute fallback for small models.

## Acceptance criteria

- [ ] Parse GVP from STEP file when present (CAx-IF RP section 4.11).
- [ ] Compute volume, surface area, centroid from imported B-rep/mesh.
- [ ] Compare and classify as green/yellow/red.
- [ ] Refuse import or mark yellow on red (configurable).
- [ ] Add tests with a STEP file that carries GVP.
- [ ] Document which vendor STEP files in the corpus carry GVP.

## Notes

- Most vendor STEP files from major CAD (SolidWorks, Creo, CATIA, Fusion)
  include GVP per CAx-IF RP.
- This is the standard's own acceptance test. It requires no per-route
  instrumentation.
- If GVP is absent, skip comparison (do not fail). Record "no GVP" in metadata.

## CURRENT

(none)
