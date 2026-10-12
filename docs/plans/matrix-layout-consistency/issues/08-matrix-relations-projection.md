# 08: Project matrix relationships for Relations

Status: resolved
Type: build
Blocked by: 03
Spec: [spec.md](../spec.md#relations) · Map: [map.md](../map.md)

## What to build

A pure projection in `web/crates/layout/src/matrix_relations.rs`. Its inputs are the
accepted `ProjectDoc`, its `MatrixScene` (including ticket 03's `companions`) and a
`TreeContext`. It returns a view model, not markup:

- `structure`: items with a label, a value and an optional navigation target
  (`TreeContext`). Each item also carries the part IDs to highlight (ticket 10).
- `departures`: items naming the parameter they override (row offset, column offset,
  key local offset or rotation, authored companion), plus their available actions
  (Reset; Convert for a column offset).
- `constraints`: kind, source and target references, and whether the constraint takes
  precedence over an attachment.

Fill these per selection exactly as the [spec's Relations table](../spec.md#relations)
says. Companion modes come from the scene, not from re-reading `layoutOverride` or
constraints. Column references are 1-based ("from Column N"), and the first column
reads "from the matrix origin". Only enabled, non-deleted cells count as keys.
Removed slots are counted separately. A context naming a removed row, column or
cell returns `None`. The projection has no state and makes no Core calls.

## Tests (tdd; native, scenes from `CoreEngine`)

1. A blank 5×7 matrix gives non-empty Structure for the matrix ("5 × 7, 35 keys",
   pitch, origin, independent), Column 3 ("0 mm from Column 2", inherited by Columns
   4–7) and a key. It has no departures and no constraints.
2. Staggers `[0, 2, −1]`: Column 2 is "2 mm from Column 1, cumulative 2 mm"; Column
   3 is "−1 mm from Column 2, cumulative 1 mm".
3. A column offset gives one departure, on that column only, with Reset and Convert.
4. A key with a following, an authored and a constrained companion shows each mode
   from the scene. The constrained companion names its source.
5. On a linked pair, each half names its partner.
6. A removed context gives `None`.

## Acceptance

`lint` and `test` pass. The panel renders the projection in 09.

## Files

The new `matrix_relations.rs` and its registration in the module tree. Can run in
parallel with 04–07.

## Outcome

Resolved 2026-10-09. `web/crates/layout/src/matrix_relations.rs` is a pure projection
(`matrix_relations(document, scene, context) -> Option<MatrixRelations>`), registered in `lib.rs`. It
returns Structure items (label, value, navigation target, highlight part IDs), Departures (parameter,
value, target, highlight, Reset and Convert actions) and Constraints (kind, source and target
references, whether the constraint takes precedence over an attachment) for Matrix, Row, Column, Key
and Matrix component selections, per the spec table. Companion modes come only from the scene's
`companions`; the module reads no overrides or constraints to decide precedence. Columns are 1-based
in text, the first reads "from the matrix origin", and a negative value uses U+2212. A context naming
another matrix, a removed key or a row or column past the matrix gives `None`.

Tests (`matrix_relations/tests.rs`, scenes from a real `CoreEngine`, all six ticket cases) pass:
blank 5 x 7 structure; staggers `[0, 2, -1]` with cumulative values; a column offset as one departure
with Reset and Convert; follows, authored and constrained companions (the constrained one names its
source and sets `takes_precedence`); linked pair partners; removed and foreign contexts. They were
written together with the module, not red first. Native layout suite: 75 passed.

Decisions and gaps for 09: "Inherited by" is one navigable item per later column rather than a
range; the Column "pivot" lists only the splay pivot kind (default or custom), not which columns it
pivots; Convert is listed for every column offset and its availability rule is ticket 07's; the key
row, column and cumulative-stagger items link to their objects. The projection is not rendered yet.

