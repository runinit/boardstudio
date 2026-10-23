# 06: Decide how origin edits hold keys still

Status: ready-for-human
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Moving a column's splay origin keeps every key still by default, as it does today. A modifier
swings the keys about the new origin instead, as ergogen does. Today the compensation is written
as column offsets on this column and every later column (`splay.rs::preserve_columns`).

With the origin relative to the column base, decide:

- where the hold-still compensation is stored. For example, a translation folded into this
  column's spread and stagger moves later columns and their origins rigidly, so only one column's
  values change;
- which fields visibly change when the origin moves, and how that reads in the Inspector;
- the modifier, and the Undo granularity;
- the Origin tool's "Reset to column base" behaviour.

The same origin also turns the column's **Column rotation**
([ticket 05](05-this-column-splay-representation.md#answer)), so holding keys still must cover it
too.
