status: accepted
---

# Validate outlines on the fabrication grid with a per-path budget

Outline edge contacts are judged on the existing 0.001 mm geometry grid. Exact
integer predicates reject every nonadjacent contact or crossing; consecutive
edges may meet only at their shared endpoint, so backtracking overlaps are invalid.
The checker orders edge boxes by x and tests only boxes that can overlap. Its
candidate scan can still be quadratic when many boxes overlap, so each persisted
path that reaches it is limited to 1024 points: authored polygons, protected gaps
and component courtyards. Generated intermediate paths use the same checker
without this persisted-input limit.

An over-budget persisted path remains saved and editable and reports an outline
finding that blocks dependent output, consistent with
[ADR-0001](0001-fixed-edited-outlines.md). The budget is per path, not per project.
