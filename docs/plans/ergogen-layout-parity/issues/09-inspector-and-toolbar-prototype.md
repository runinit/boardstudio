# 09: Prototype the Inspector and toolbar for the new parameters

Status: ready-for-human
Type: prototype
Blocked by: [01](01-name-the-carried-parameters.md), [04](04-layout-kernel-order-of-operations.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Make a rough, clickable prototype of how users edit:

- column spread;
- row padding;
- Shift and Bend, at row and key level;
- independent offsets;
- the relative splay origin, its Reset button and the Ctrl swing
  ([ticket 06](06-origin-edits-hold-keys-still.md#answer));
- Column rotation, and how the Splay drag's "This column" choice looks
  ([ticket 05](05-this-column-splay-representation.md#answer));
- the matrix Anchor section, its canvas Pick, Follow X/Y/rotation, Detach, and Add anchored
  matrix ([ticket 07](07-matrix-anchors.md#answer)).

Cover the Inspector fields for Matrix, Row, Column and Key selections, and the toolbar tools and
drags, including a Bend gesture. Cover keyboard nudges and how a carried value
visibly moves the keys above it during preview. Existing tools stay: Stagger, Offset, Splay,
Origin and the Shift quick offset. React to the prototype with the user and record the chosen
layout and wording.
