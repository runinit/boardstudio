# Generation optimization coverage

The original E0–E9 plan remains open. The first Cadrum/Manifold/Monstertruck
comparison was one bounded batch, not completion of that plan. Low bridge-history
or copy timings are prioritization evidence, not substitutes for the other tests.
The case design, production backend, and existing performance budgets are preserved.

## Coverage and remaining work

| Item | Evidence gathered | Still required |
| --- | --- | --- |
| E0: controls and attribution | Reproducible staged Cadrum, native/STEP material checks, browser bottom timings, Boolean/mesh attribution | Byte-identical unmodified production control and representative whole-assembly/cold/burst/Undo/export measurements; full operation/copy counts |
| E1: unused mesh output | Separate edge, face-ID, and combined suppression on four bottoms; no useful speed gain | Imported STEP and other affected body classes before any general change; no production adoption |
| E2: provenance/history | Native/STEP/browser checks and five-session history-suppression screening; less than 1% on the regression bottom | Wider topology/history-consumer coverage if pursued; not selected for speed |
| E3: Boolean options | Native/STEP/browser checks: dedicated cuts improve about 4%; OBB depends on orientation | Real rotated/contact/general-expression coverage, combinations, and full-app results for a finalist |
| E4: tabbed-plate meshing | Real plate has two orthogonal 28-vertex regions, 140 hole vertices and 390 grid cells each, below the existing 4,096-cell cap | Implement and test guarded outer-boundary classification; tabs/notches/winding/contact/collapse/fallback cases; closure/material/renderer and browser measurements |
| E5: Boolean planning | Native/STEP/browser checks and five-session screening: combined cuts/profile holes improve about 4–6% | Spatial grouping, stable ordering, primitive reuse, combinations and broader eligibility/fallback tests |
| E6: parametric reuse | Across six captured edits, 10/12 mounting inputs and 12/16 opening definitions remain unchanged; both outer contours change every time | Measure bounded primitive reuse including clone/integration costs; dependency changes, Undo, eviction, context, export and memory checks |
| E7: stepped profile construction | Extruded height bands preserve material after shared-coordinate repair, but are about 40% slower | This bounded implementation is rejected; direct face assembly was not tested and is not evidence of a speed gain |
| E8: ownership | All result copies retained; non-destructive mode is about 30% slower in browser screening | Copy removal stays conditional on meaningful cost and separate lifetime/drop-order/eviction/mesh/export proof |
| E9: secondary options | Final cleanup passes geometry but is slower in browser screening; current WASM memory is unshared | Release/LTO/wasm-opt comparisons and complete archive/thread/deployment feasibility audit |
| Preview alternative | Manifold bottom preview passes bounded checks at about 24 ms; Monstertruck probe fails/times out | Full workflow/all changing bodies, scheduling/fallback/readiness/export separation, startup, duplicate work and memory soak before integration |
| Final acceptance | Phase 2 evidence retained | Full correctness, frozen CAD budgets, five visible live sessions, interaction and 256-cycle soak for any promoted combination |

Round-two timing and decisions are recorded in the
[OCCT report](occt-optimization-round2.md). Every outstanding entry above remains pending;
no failed prototype or small isolated timing closes a different experiment.

## Execution order

1. The one-variable Cadrum/OCCT screening batch is complete. Retain its modest
   construction gains as candidates for later combination; none independently
   meets the 10% promotion threshold.
2. Test E4's real eligible tabbed plates and E6's confirmed unchanged inputs.
   These are concrete remaining opportunities, not speculative engine migration.
3. Complete remaining E5 grouping/preparation and E9 build comparisons, revisiting
   E8 only when its cost and ownership evidence justify it.
4. Combine only demonstrated improvements, then run full-app acceptance. Keep
   a Manifold preview integration experiment separate from exact-generation claims.

See [the original protocol](generation-optimization-experiments.md),
[first comparison results](gasket-comparison-results.md), and
[current plan](../PLAN.md). [TODO.md](../TODO.md) preserves the completed Phase 2 checklist and now tracks
the outstanding Phase 3 work above. Its active phase is 3; the completed Phase 2
items retain their original PLAN 2.x references and outcomes. This coverage
document remains the detailed record of partial evidence and acceptance gaps.
