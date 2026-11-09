# 04: Fix WRL primitive tessellation, per-face color indexing, and box normal winding

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Decoding VRML97 (`WRL`) models in the renderer produces valid finite triangles for `Sphere` primitives, outward-facing normals for `Box` primitives, and per-face colors indexed by `face_index` when `IndexedFaceSet` specifies `colorPerVertex FALSE` without an explicit `colorIndex` list.

## Acceptance criteria

- [x] Decoding a WRL `Sphere { radius 1 }` succeeds with finite triangle positions and outward-pointing unit normals instead of failing with `WRL model has no finite triangles` from degenerate polar cap triangles.
- [x] Decoding a WRL `IndexedFaceSet` with `colorPerVertex FALSE` and no `colorIndex` assigns `Color.color[face_index]` to each face rather than painting every face with the last color in `Color.color`.
- [x] Decoding a WRL `Box` emits triangles wound counterclockwise from the outside so all 12 triangle normals point outward (`dot(centroid, normal) > 0`).
- [x] Native renderer tests cover `Sphere` decoding, `colorPerVertex FALSE` per-face coloring without `colorIndex`, and outward `Box` normals.

## Answer

Resolved at the `renderer::decode_wrl` seam (`renderer/src/wrl.rs`, `renderer/src/lib.rs`):
- `append_sphere` skips degenerate polar cap triangles (`latitude + 1 < LATITUDES` for the lower-pole triangle, `latitude > 0` for the upper-pole triangle) and winds triangles counterclockwise from outside so all normals are finite and point outward.
- `IndexedFaceSet` with `colorPerVertex FALSE` and no `colorIndex` falls back to `face_index` rather than `colors.len().checked_sub(1)`.
- `append_box` winds all six faces counterclockwise from outside so all 12 triangles have outward-pointing normals (`dot(centroid, normal) > 0`).
- Verified with `cargo test -p boardstudio-renderer --locked` (36 passed).
