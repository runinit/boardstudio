# 12: Decide how Relations explains carried chains and anchors

Status: ready-for-human
Type: grilling
Blocked by: [01](01-name-the-carried-parameters.md), [04](04-layout-kernel-order-of-operations.md), [05](05-this-column-splay-representation.md), [07](07-matrix-anchors.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Relations (`web/crates/layout/src/matrix_relations.rs`, `web/src/presentation/layout_matrix_relations.rs`)
and the canvas highlight currently explain cumulative stagger, splay, pitch, gap and companion
modes for one matrix. Decide how they explain:

- carried chains: which later rows and keys inherit a Shift or Bend, and which columns inherit
  spread, stagger and splay;
- Column rotation, which applies to one column only
  ([ticket 05](05-this-column-splay-representation.md#answer), decision 8);
- padding and offsets, which do not carry;
- the Shift and Bend that disabled and deleted keys still carry
  ([ticket 04](04-layout-kernel-order-of-operations.md#answer), decision 7);
- dependencies between matrices: what a matrix follows, which matrices follow a key, and the
  Follow subset ([ticket 07](07-matrix-anchors.md#answer));
- the entries and wording, what hover or focus highlights on the canvas, and which tool each
  entry opens.

Relations follows the stagger pattern and says "origin", not "pivot"
([ticket 01](01-name-the-carried-parameters.md#answer)).
