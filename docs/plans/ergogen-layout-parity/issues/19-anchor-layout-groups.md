# 19: Evaluate multi-matrix Layouts and maintain single-key anchors

Status: ready-for-agent
Type: build
Blocked by: [18](18-core-placement-edits.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns the Core edit for aligning through anchors; the Setup, Align and selection consumer wiring that calls it is left to [22](22-update-layout-consumers.md).

## Outcome

Complete anchor graph evaluation and Core-owned create/change/detach behavior. A
Layout is one board-local group, not an array-order parent tree. No aggregate arm.

## First RED / GREEN slice

Fail an actual Core edit creating a 1×3 anchored matrix in its selected key's Layout;
move that key and assert dependent member poses. Add only the graph/evaluation needed
to pass, then test reordered matrix lists through the same public route.

## Subsequent acceptance

- Evaluate dependency order, actual final key frames, Origin/Rotation then Follow;
  cover every subset, zeroing, rotated frames and constrained matrix drag directions.
- Add/change/detach are one typed Core transaction/Undo, not several UI requests.
  Default Add is Origin `(0,-pitch.y)`, all Follow components, 1×3 and same Layout.
- Refuse self/cycles, cross-Layout/board targets, missing targets and constraint-driven
  anchor keys; refuse constraining an anchor key. Solve constraints after anchors.
- Logical disabled/deleted keys and Omitted matrices/keys keep evaluating before
  visible output filtering. Do not materialize missing geometry for anchors.
- Compaction remaps physical references; actual shrink/removal detaches dependants
  and keeps frame, never targets a new key at an old coordinate. Deliver the group-level form of the
  confirmed asymmetric linked policy here (20–21 consume it later), not a silent fallback.
- Extend placement/Align through actual anchor frames and Follow representability;
  exact impossible edits refuse; constrained drags/Rotate availability remain truthful.

## Checks

Source seams: Core matrix layout/validation/deletion and constraint preparation,
public Open/ResolveEdit/scene interfaces and Session preview/history. Cover valid and
refused graphs with actual materialized poses and cancellation, not graph helper
success alone. Run focused Core/Session tests and quick checks for changed crates.
