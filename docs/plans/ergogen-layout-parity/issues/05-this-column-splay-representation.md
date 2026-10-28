# 05: Represent "This column" splay as its own intent

Status: resolved
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

## Answer

Decided with the user on 2026-10-22; every recommendation was accepted. The term is recorded in
[GLOSSARY.md](../../../../GLOSSARY.md#matrix-layout).

Today "This column" is not stored anywhere. It is a session-only choice (the `splay_affect`
signal, shown as the Inspector's "Splay affects" select). That choice makes
`splay.rs::update` write a counter-splay on the next column and compensating offsets on every
later column.

1. **Its own non-inherited angle.** Each column record gains `rotation`. It turns that column's
   keys, as a rigid body, about the column's splay origin. Rows, Shift, Bend and offsets all
   turn with it. In the kernel ([ticket 04](04-layout-kernel-order-of-operations.md#answer)),
   column c starts with splays 0..c plus its own rotation about the same origin. Column c+1's
   base and heading ignore that rotation, so every other column stays still by construction. A
   "This column" edit writes only this field: no counter-splay, no compensating offsets. The
   value must be finite.
2. **The name is Column rotation.** It pairs with Splay (carried) the way Key rotation pairs
   with Bend, and Offset with Shift.
3. **The Inspector shows two fields** for a Column selection: Splay ("Turns this column and the
   columns after it") and Column rotation ("Turns this column only"). The "Splay affects"
   select is removed.
4. **The toolbar's Splay drag** keeps a "This column / This and following" choice. The choice
   now means which field the drag writes: "This and following" (the default) writes Splay,
   and "This column" writes Column rotation. The
   [prototype ticket](09-inspector-and-toolbar-prototype.md) decides how the choice looks.
5. **Switching modes never changes stored values**, and there is no conversion action. Turning
   a Column rotation into Splay would move later columns, unless later columns got
   compensation, which is the hidden state this ticket removes.
6. **Linked halves copy Column rotation unchanged**, as they do Splay. The mirror negates the
   local heading. Column values have no Local exception.
7. **One origin for both turns.** Splay and Column rotation turn about the same splay origin,
   and the Origin handle shows whenever either is nonzero. The
   [origin-edit ticket](06-origin-edits-hold-keys-still.md) must also hold keys still under
   Column rotation.
8. **Relations and the scene.** Relations lists Column rotation separately ("Column 2
   rotation: 5°, this column only"), outside "Cumulative splay" and "Inherited by".
   `MatrixColumnBasis` adds `rotation`. Column c's axes and frames include it; column c+1's do
   not.
