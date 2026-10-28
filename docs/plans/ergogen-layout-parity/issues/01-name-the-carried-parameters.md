# 01: Name the carried parameters

Status: resolved
Type: grilling
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

The new row-level and key-level transforms carry to every key above them in the column.
Ergogen calls them `shift` and `rotate`. BoardStudio already uses **Key rotation** for the
independent turn of one key (ergogen `adjust.rotate`) and **Offset** for independent moves.

Decide:

- the canonical domain terms for the carried shift and the carried rotation, at row level and at
  key level, and their `_Avoid_` lists;
- whether **Key rotation** and **Key local offset** are renamed to sit clearly beside them;
- the user-facing labels: Inspector field names, toolbar tool names, Relations wording.

Record the terms in `GLOSSARY.md` (Matrix layout). Ergogen-familiar names are preferred where
the concept is identical. A name must never mean "inherited" in one place and "independent" in
another.

## Answer

Decided with the user on 2026-10-22. The terms are recorded in
[GLOSSARY.md](../../../../GLOSSARY.md#matrix-layout).

- **Shift** (row shift, key shift) is the carried move, along the key's heading. It equals
  ergogen `shift`.
- **Bend** (row bend, key bend) is the carried turn: the later rows swing about the key, so the
  column curves. It equals ergogen `rotate`. It is not called "Rotate" because of the existing
  Key rotation and matrix Rotation.
- **Independent terms are unchanged:** Row offset, Column offset, Key local offset ("Local X/Y")
  and Key rotation. The glossary reads in pairs: Shift / Offset and Bend / Key rotation.
- **Hints say "later rows", never "above".** For example, "Moves this row and the rows after
  it", like Stagger's "Moves this column and the columns after it".
- **Tools take their parameter's name** (Shift, Bend, and Spread and Padding if they become
  tools). Whether each tool exists is decided by the
  [prototype ticket](09-inspector-and-toolbar-prototype.md).
- **Relations reuses the stagger pattern:** "Row 2 shift: 2 mm", "Inherited by Row 3",
  "Cumulative shift" and "Cumulative bend".
- **Build item:** Relations says "origin" instead of "pivot" (for example "custom pivot" becomes
  "custom origin"), matching the Inspector and toolbar.
- **No ergogen names in the UI.** The name mapping lives in the parity spec and the fixtures.
