# 01: Name the carried parameters

Status: ready-for-human
Type: grilling
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

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
