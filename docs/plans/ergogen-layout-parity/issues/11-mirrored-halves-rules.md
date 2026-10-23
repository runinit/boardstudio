# 11: Decide mirrored-half rules for the new layout fields and anchors

Status: ready-for-human
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md), [05](05-this-column-splay-representation.md), [06](06-origin-edits-hold-keys-still.md), [07](07-matrix-anchors.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Linked halves (ADR-0008, ADR-0010) must reflect every new field deterministically. The kernel
fixes the reflection rule: values are copied unchanged, and the mirror negates the local heading
([ticket 04](04-layout-kernel-order-of-operations.md#answer), decision 5). For each new field,
decide how `layout::reflected`, `mirrored_half/inverse` and `mirrored_half/reconcile` treat it,
and which Local or Omitted exceptions apply. The fields are:

- the `columnLayout` records (spread, stagger, splay, Column rotation, splay origin, offset);
- the `rowLayout` records (padding, Shift, Bend, offset);
- cell Shift and Bend;
- the matrix anchor and `Layout.matrixIds`.

Also decide:

- how a hold-still origin edit on column 0 changes the follower's matrix Origin so both halves
  hold still ([ticket 06](06-origin-edits-hold-keys-still.md#answer), decision 2);
- how an anchor maps to the partner's matching key through the Matrix correspondence or the
  Layout mirror link, and how a Local exception on an anchored matrix behaves
  ([ticket 07](07-matrix-anchors.md#answer), decision 6);
- how an edit on the mirror half inverse-maps each field back to the source, including detach
  and keep-pose;
- which refusals linked halves need, such as the existing refusal of a local Y mirror.
