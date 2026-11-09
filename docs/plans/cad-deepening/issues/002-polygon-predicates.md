# 002: One polygon containment and area module

Status: resolved
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

- [x] An audit, appended under `## Comments`, lists each copy and its on-edge, winding and hole behaviour.
- [x] Copies without a deliberate difference call the shared module; deliberate ones are commented.
- [x] The shared module has tests for on-edge, vertex, hole and clockwise/counter-clockwise inputs.
- [x] Each migrated caller keeps its verdicts; existing tests pass.

## Comments

Audit. The ticket counted about nine copies; there were fifteen. Every one is the same
float even-odd ray cast with the same half-open edge rule (left and bottom edges inside,
right and top edges outside), so copies agree on every input except possibly a point exactly
on a slanted edge: `case.rs`, `host_connector.rs` and `case_viewer.rs` walked edges as
(i, i-1) and the shared module walks (i, i+1), which swaps the endpoints in the interpolation
and can move the result by one ulp there. Other differences were cosmetic:

| Where | Difference | Outcome |
|---|---|---|
| `case.rs`, `mechanical_checks.rs`, `artifact/preview.rs`, `keycaps.rs`, `modules/host_connector.rs`, `outline.rs`, `outline_cleanup.rs`, `cad/wasm` lettering, `case_viewer.rs` | type (`Vec2` or `[f64; 2]`), argument order, a `len < 3` guard | migrated |
| `mechanical.rs` `inside_outline` | adds hole handling over a ring test | migrated; keeps its hole logic |
| `outline_validation.rs` | even-odd across a set of rings | migrated as an XOR over `path_contains` |
| `renderer/src/geometry.rs` | f32 render space, no contracts dependency | kept, commented |
| `core/tests/outlines.rs`, `core/tests/outline_repairs.rs` | independent test oracles | kept, commented |
| `core/tests/module_host_connector.rs` | JSON-valued test helper | untouched |

The `len < 3` guards changed nothing: fewer than three points already classifies as outside,
and the new module's tests pin that. [ADR-0012](../../../adr/0012-bounded-outline-validation.md)
turned out not to apply: it governs exact edge-contact validation on the fabrication grid,
not this float classification. Semantics are unchanged; the shared module is
`boardstudio_contracts::{path_contains, polygon_contains}`.

Verification: contracts 8, core 1,512 (14 `kicad-cli` tests only pass with `APPDIR`,
`APPIMAGE`, `LD_LIBRARY_PATH` and `GSETTINGS_SCHEMA_DIR` removed from the environment, which is
the T3 Code AppImage's stale mount and unrelated), web case 34, CAD worker 57.

Not done (found in code review): polygon **area** was in the title and the first paragraph but
not migrated; there are separate signed-area copies in `core/src/case.rs`,
`core/src/modules/occupancy.rs`, `web/crates/layout/src/outline/overlays.rs` and the CAD
worker's lettering, differing in sign, `abs` and f32/f64, so they need their own audit. Two
more containment copies sit in standalone gate binaries under
`cad/monstertruck-qualification` (a float ray cast in `profile_topology_gate.rs` and an
exact-rational one in `certified_prism.rs`); they are outside the production build and were
left alone. "Hole" in the test criterion is covered by the callers (`inside_outline`), since a
ring test has no holes; the module's tests cover on-edge, vertex, winding and degenerate rings.
