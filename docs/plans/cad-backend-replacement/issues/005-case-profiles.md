# 005: Build production case profiles with Monstertruck

Status: resolved
Owner: /root/luna_production_profiles_005
Type: build
Blocked by: None

## What to build

Build prepared polygon regions, holes and plate/tray/lid extrusion as exact Monstertruck solids and meshes in the production CAD source, keeping the working application intact during migration.

## Acceptance criteria

- [x] Prepared-region input reaches the production Monstertruck builder and mesh output; reuse intended case elevations, dimensions and disconnected-region behavior.
- [x] Preserve concave profiles, inner holes and winding behavior; reject invalid, nonfinite and self-crossing contours instead of emitting partial geometry.
- [x] Verify bounds, closed oriented topology and independent analytic volume/material expectations for representative region fixtures.
- [x] Provide a small exact-model interface for dependent features/export, with native and WASM checks plus independent Standards/Spec review; preview meshes never replace exact solids.

## Comments

2026-10-17: Root integrated the reviewed helper reuse, Core-compatible case schema relocation, core WASM override and render-precision correction. Fresh actual combined source passes10 CAD tests, WASM library/test compilation, Core check and the old-path serde regression; formatting passes. All10 CAD tests also pass after the reviewed Circle2D period delta. Independent bounds/material/closed-topology checks and both review axes clear this bounded builder prerequisite. See integration evidence (`../../../../.evidence/production-profiles-005/root-combined-v1/REPORT.md`). Application/cache/worker migration remains010; resolving005 does not establish complete case features or full backend acceptance.

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.
