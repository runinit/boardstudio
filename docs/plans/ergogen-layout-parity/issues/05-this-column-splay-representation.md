# 05: Represent "This column" splay as its own intent

Status: ready-for-human
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

"This column" splay (`MatrixSplayAffect::Column`) turns one column only. Today it stores a
counter-angle on the next column plus compensating offsets on every later column
(`core/src/matrix/splay.rs::update`), so other columns' fields show values the user never typed.

Decide how the intent is stored and evaluated in the new kernel. One option is a non-inherited
column turn about the column's splay origin; there may be others. Also decide:

- how the Inspector and toolbar show it next to an inherited splay;
- what switching a column between the two modes does to its values;
- how it reflects on a linked half.
