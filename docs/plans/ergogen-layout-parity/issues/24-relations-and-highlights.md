# 24: Deliver Core-backed placement explanations and Relations highlighting

Status: claimed
Type: build
Blocked by: [23](23-inspector-toolbar-and-handles.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Finish typed Core/scene contribution facts and present one How it is placed chain,
Independent adjustments and Authored constraints. Do not build a web-side geometry
oracle or move the panel's strings/tree into Core.

## First RED / GREEN slice

Fail a mounted key explanation for Shift (10,0), Bend 90°, Shift (10,0): show cumulative
Shift (10,10) in matrix axes from real Core facts, not raw (20,0). Add its parameter
navigation without document/history change, then take the next relation case.

## Subsequent acceptance

- Trace actual column/row/key contributions, independent turns/offsets, defaults and
  cumulative Bend versus other headings. Mixed values say Varies; cross-matrix
  selections do not acquire a pretend common frame. Use later rows and origin.
- Separate Inherited by from direct Follows/Followed by and Follow subset; explain
  actual Local anchors, Local pose pinning, Omitted logical poses and independent
  adjustments whose anchors can still move another matrix.
- Source references highlight contributors; receiver references highlight their named
  receivers, violet dashed, with keyboard-focus parity. Keep amber for actual preview.
- Clear references on leave/blur/unmount/context change/removal; reject stale reused
  identities. Hover changes no selection/camera/visibility/document/history.
- Explain Disabled/Deleted/Omitted/Hidden sources without resurrecting geometry or
  layers. Logical inspection does not bypass Omitted ownership or restore parts.
- Parameter activation selects/opens its owner/tool; reference activation navigates.
  Reset/Convert remain explicit separate writes, with accessible blocked reasons and
  Core-owned companion/constraint precedence.

## Checks

Sources: `web/crates/layout/src/matrix_relations.rs`,
`web/src/presentation/layout_matrix_relations.rs`, UI-model highlight state and
`web/src/presentation/layout_relation_highlight_tests.rs`. Add real Core fact coverage
and mounted panel/page pointer+focus lifecycle tests, with affected browser subsets
and quick checks. Selected state, visibility and pending edits must remain independent.
