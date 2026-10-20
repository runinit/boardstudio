# 002: One polygon containment and area module

Status: ready-for-agent
Type: build
Blocked by: None
Parent: [CAD deepening](../map.md)
Related: [ADR-0012](../../../adr/0012-bounded-outline-validation.md)

## What to build

Point-in-polygon and polygon area are reimplemented in roughly nine places across Core
(case preparation, mechanical checks, outlines, outline cleanup, artifact preview, host
connector modules), keycaps and the case viewer. A **Case exterior** can contain a point in
one place and not another. Audit each copy's on-edge, winding and hole behaviour, keep any
copy whose difference is deliberate (with a comment saying why), and move the rest onto one
module in contracts that follows ADR-0012's fabrication grid.

## Acceptance criteria

- [ ] An audit, appended under `## Comments`, lists each copy and its on-edge, winding and hole behaviour.
- [ ] Copies without a deliberate difference call the shared module; deliberate ones are commented.
- [ ] The shared module has tests for on-edge, vertex, hole and clockwise/counter-clockwise inputs.
- [ ] Each migrated caller keeps its verdicts; existing tests pass.
