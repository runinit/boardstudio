# 07: Define matrix anchors and several matrices per layout

Status: ready-for-human
Type: grilling
Blocked by: [02](02-ergogen-anchor-and-mirror-semantics.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

A layout can hold several matrices, and a matrix can anchor its frame to a key of another
matrix in the same layout, with shift, rotation and ergogen-style `affect` (ADR-0034). Decide:

- the stored anchor shape. How it composes is settled by the
  [kernel ticket](04-layout-kernel-order-of-operations.md#answer): world = anchor frame ·
  origin · rotation · mirror · local;
- the evaluation order across matrices, and refusal of cycles;
- what happens when the anchor key is disabled, deleted, or renumbered by row or column
  compaction (`matrix/deletion.rs::remove_axis`), or when its matrix is removed. For example:
  detach and keep the current pose, or refuse;
- linked halves: the anchor maps to the partner's matching key, and Local or Omitted exceptions
  on either end apply;
- a layout's ownership rules (`layout::validate` currently gives each layout exactly one
  matrix), board membership, and setup for an extra matrix (for example "Add thumb cluster");
- selection and context when several matrices are selected, and how anchored matrices appear in
  Objects;
- how explicit constraints and part-offset constraints that reference anchored keys are solved,
  in order.
