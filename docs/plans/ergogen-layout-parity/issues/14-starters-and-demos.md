# 14: Decide how starters and demos are rebuilt

Status: ready-for-human
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md), [07](07-matrix-anchors.md), [08](08-uniform-pitch-consumers.md), [09](09-inspector-and-toolbar-prototype.md), [11](11-mirrored-halves-rules.md), [12](12-relations-and-highlight.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Graduated from the map's Starters fog now that layout, anchors, editing and explanation
semantics are settled. ADR-0034 requires rebuilding starters and fixtures under the new
format rather than migrating them. Decide:

- which current bundled examples, starter/demo paths and fixture families must be rebuilt;
- which preserve their demonstrated geometry and which intentionally demonstrate Spread,
  Padding, Shift/Bend, independent rotations or matrix anchors;
- whether any split example's thumb cluster becomes a separate anchored matrix and which
  linked-half/exception behaviour it demonstrates;
- default dimensions, pitch and parameter choices, and the expected visible layout;
- the golden/regression checks, provenance and attribution needed for replacement data.

First inventory the actual current examples and fixtures rather than relying on retired
starter names. Keep geometry-only examples labelled as such; do not introduce electrical
or fabrication-readiness claims. This ticket chooses replacement data and acceptance,
not the implementation or a new hardware recipe.
