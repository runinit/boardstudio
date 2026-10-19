# 04: Fix WRL primitive tessellation, per-face color indexing, and box normal winding

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** Decoding VRML97 (`WRL`) models in the renderer produces valid finite triangles for `Sphere` primitives, outward-facing normals for `Box` primitives, and per-face colors indexed by `face_index` when `IndexedFaceSet` specifies `colorPerVertex FALSE` without an explicit `colorIndex` list.

## Acceptance criteria

- [ ] Decoding a WRL `Sphere { radius 1 }` succeeds with finite triangle positions and outward-pointing unit normals instead of failing with `WRL model has no finite triangles` from degenerate polar cap triangles.
- [ ] Decoding a WRL `IndexedFaceSet` with `colorPerVertex FALSE` and no `colorIndex` assigns `Color.color[face_index]` to each face rather than painting every face with the last color in `Color.color`.
- [ ] Decoding a WRL `Box` emits triangles wound counterclockwise from the outside so all 12 triangle normals point outward (`dot(centroid, normal) > 0`).
- [ ] Native renderer tests cover `Sphere` decoding, `colorPerVertex FALSE` per-face coloring without `colorIndex`, and outward `Box` normals.
