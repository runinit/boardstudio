# 170: Bound WRL decoding and refuse unsupported polygons

Status: ready-for-agent
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Importing a deeply nested or heavily reused WRL model returns a clear unsupported
input error before exhausting the decoder's stack or work budget. The same shared
renderer instance remains usable for the next ordinary model. Keep valid bounded
node reuse, existing cycle refusal, and the current output-size protection.

A model outside the preview's supported convex polygon subset also receives a
clear unsupported-input error rather than a mesh that fills empty source space.
Use explicit refusal as the smallest complete fix; general concave polygon
triangulation is not required for this ticket.

The audit reproduced a packaged WASM trap with 4,096 nested groups in a
94,245-byte file; a box then trapped in that same instance. A separate 1,174-byte
reference graph took about 5.2 seconds despite adding no geometry. These are
historical reproduction measurements, not acceptance timing thresholds.

The audit also found that a valid planar U-shaped face declaring nonconvex
polygons produces a triangle fan covering its empty notch. VRML97 permits this
face; the preview must acknowledge its narrower supported subset.

## Acceptance criteria

- [ ] First reproduce the nesting and acyclic repeated-reference regressions
  through the real decoder, then demonstrate deterministic bounded refusal.
- [ ] Enforce a finite traversal depth before descending, and a cumulative
  expanded-occurrence/work budget shared across the whole decode. Empty groups,
  repeated references and separate roots consume work even without triangles.
- [ ] A limit breach returns an input-specific error without a WASM trap, accepted
  partial model, silent truncation or unbounded expansion. Cycle and output-size
  refusals remain effective; do not replace them with only the new limits.
- [ ] Boundary tests cover admitted versus refused depths/work, noncyclic shared
  nodes, multiple roots, cyclic references and a normal colored/transformed model.
  Choose and document finite limits against representative supported assets.
- [ ] First demonstrate the authored U-shaped face incorrectly covers a point
  inside its empty notch. After the fix the model is explicitly refused, with no
  accepted partial mesh or silent omission of the unsupported face.
- [ ] Conservatively refuse nontriangular faces declared nonconvex before triangle
  emission. Explain this supported-format limit in the error/documentation; a
  nonconvex declaration does not prove that every face is geometrically concave.
- [ ] Keep valid triangles and supported convex polygons working, including
  indexed per-vertex/per-face colors and transforms. Independently authored
  controls cover default-convex and explicitly convex polygons, a nonconvex-declared
  triangle and the nonconvex-declared U-shaped polygon.
- [ ] Fresh packaged WASM evidence uses one module instance: reject each hostile
  input, then successfully decode a normal box in that same instance. Verify the
  application model-loading path reports refusal without poisoning subsequent
  model use; creating a new decoder for the control is insufficient.
- [ ] The fresh packaged decoder probe also proves U-shaped refusal and supported
  polygon controls. The application model-loading path surfaces that refusal
  instead of showing a filled notch, and subsequent model use remains healthy.
- [ ] Run affected renderer checks. Resource assertions use bounded counters and
  behavior, not sleeps, weakened limits or a machine-specific speed threshold.

## Comments

Created from the user's audit findings after primary-source research. At the
user's request, merged the traversal and polygon-admission findings into this
single decoder slice; [172](172-refuse-unsupported-wrl-polygons.md) is retired as
merged, with all its acceptance requirements retained here.

This ticket adds bounded input handling, not a new WRL engine or renderer reset
fallback that conceals the original trap. It does not claim complete VRML97
support or broaden CAD analysis to use display meshes.
