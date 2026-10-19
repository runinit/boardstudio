# 172: Refuse WRL polygons the preview cannot triangulate correctly

Status: ready-for-agent
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

A WRL model outside the preview's supported convex polygon subset receives a
clear unsupported-input error rather than a mesh that fills empty source space.
Use explicit refusal as the smallest complete fix; general concave polygon
triangulation is not required for this ticket.

The audit's valid planar U-shaped face declares nonconvex polygons. A triangle
fan currently covers its empty notch. The VRML97 format permits that face, so
the preview must either support it correctly or acknowledge its narrower subset.

## Acceptance criteria

- [ ] First demonstrate the authored U-shaped face incorrectly covers a point
  inside its empty notch. After the fix the model is explicitly refused, with no
  accepted partial mesh or silent omission of the unsupported face.
- [ ] Conservatively refuse nontriangular faces declared nonconvex before triangle
  emission. Explain this supported-format limit in the error/documentation; a
  nonconvex declaration does not prove that every face is geometrically concave.
- [ ] Keep valid triangles and the existing supported convex polygon controls
  working, including indexed per-vertex/per-face colors and transforms. Rejecting
  a model must not alter the next decoded model in the same instance.
- [ ] Regression inputs use independently authored expected geometry/admission,
  not only a snapshot of the existing triangle fan. Include a default-convex
  polygon, explicit convex declaration, a nonconvex-declared triangle and the
  nonconvex-declared U-shaped polygon.
- [ ] Run the affected renderer checks and a fresh packaged WASM decoder probe
  proving the U-shaped refusal and supported controls. Confirm the model-loading
  path surfaces the unsupported-input error instead of showing a filled notch.

## Comments

Created from the user's audit findings after primary-source research. Independent
of 170's traversal budgets; coordinate overlapping decoder edits. Does not claim
complete VRML97 support or broaden CAD analysis to use display meshes.
