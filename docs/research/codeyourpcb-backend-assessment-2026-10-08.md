# CodeYourPCB as a BoardStudio generation and routing backend

Research date: 2026-10-08. Upstream source inspected at `main` commit
[`12018cb0c6d0d927e50f863ce744e985de6553f2`](https://github.com/szymontex/codeyourpcb/commit/12018cb0c6d0d927e50f863ce744e985de6553f2),
committed 2026-10-07 20:50:45 UTC. BoardStudio source checkout was at
`ef45d94d709bba59c96cb62a173664f7fdcb7947`; the working tree already contained
unrelated documentation changes. GitNexus was bound to the canonical
`boardstudio` repo path and reported its index three commits behind HEAD, so its
flow query was used as orientation only; source files were checked directly.
No builds or integration runs were done because another refactor is active.

## Recommendation

Run an isolated proof of concept for **routing only**. BoardStudio already
generates the PCB in Rust: Core renders generator jobs, allocates nets and
assembles KiCad artifacts in-process ([BoardStudio architecture](../architecture.md),
[KiCad generation](../../core/src/artifact/kicad/generation.rs)). CodeYourPCB's
unique near-term value is its in-house autorouter and perhaps its Rust DRC
rules. Replacing the generator with CodeYourPCB would also mean replacing or
adapting BoardStudio's keyboard matrix and circuit planning, module embedding,
outline semantics, board acceptance and artifact pipeline. Its `.cypcb` DSL is
an alternative board-authoring product, not a drop-in board-generation API.

Do not treat the router as production-ready from its benchmark claims. At the
pinned revision, its own routing record shows remaining shorts on several
right-side-up benchmark boards: 30 on `stm32_breakout`, 33 on `multi_ic`, 40 on
`qfp_fanout`, and 22 on its native `esp32_starter` example after follow-up
corrections. Its README says the autoroute toolbar button is hidden while route
quality is being worked on. These are useful research and regression fixtures,
not evidence that it can safely route BoardStudio designs ([routing record,
measured 2026-09-25 and corrected 2026-09-25](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/docs/routing.md),
[README](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/README.md)).

## What the repository provides

There are two packages with “router” in their role, and they should not be
confused:

| Package | What it does | Browser / external dependency implication |
|---|---|---|
| `cypcb-autoroute` | In-house Rust routing algorithms (`PathFinder` negotiated congestion and `ImprovedAStar`), exposed by `route_board(&mut BoardWorld, &FootprintLibrary, &dyn RoutingRuleSet, &AutorouteConfig)`. It produces routes/vias and reports complete, partial or failed results. | This is the candidate for a Rust/WASM routing backend. `cypcb-render` depends on it and exposes `auto_route` and parameterized/variant methods through `wasm-bindgen`. Its router dependency closure includes CodeYourPCB's world, DRC/rules and router crates, Bevy ECS, `pathfinding`, `smallvec`, serde and tracing. The autoroute manifest has WASM-specific `getrandom` features. Rust implementation and WASM-aware manifests are evidence of a plausible browser build, not a verified build or proof that every transitive dependency is pure Rust. |
| `cypcb-router` | Applies routes to `BoardWorld` and bridges Specctra DSN/SES to FreeRouting. Its `FreeRoutingRunner` launches a `freerouting.jar`. | Do not select this package if the goal is to avoid an external router. The in-house engine is a separate crate. |

Sources: [`cypcb-autoroute` API and dispatch](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-autoroute/src/lib.rs),
[`cypcb-autoroute` manifest](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-autoroute/Cargo.toml),
[`cypcb-router` API](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-router/src/lib.rs),
[FreeRouting process wrapper](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-router/src/freerouting.rs),
[`cypcb-render` manifest](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-render/Cargo.toml),
[WASM route entry points](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-render/src/lib.rs),
[workspace dependencies](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/Cargo.toml).

CodeYourPCB describes itself as code-first PCB design. The `.cypcb` source
expresses board size and layer count (with an optional outline), placed
components, named nets, and editable `trace` path blocks. Its documented
autorouter consumes that model; it is not a circuit-to-placement optimizer.
Schematic generation is listed as planned, and its source structure has a
separate Bevy ECS `BoardWorld` that the DRC, renderer and router use. The
repository's viewer is TypeScript/Vite/Canvas 2D/Three.js, even though its board
engine and routing code are Rust/WASM. Adopting the DSL or app wholesale would
therefore not meet a pure-Rust application goal; using the routing crate alone
does not require adopting that viewer. This overlaps with BoardStudio's
footprints, nets and routes, but it does not encode BoardStudio's keyboard
matrix decisions or accepted project/session lifecycle ([CodeYourPCB README](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/README.md),
[world crate](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-world/src/lib.rs)).


CodeYourPCB also has a generation/export surface: `cypcb-kicad` reads and
writes KiCad boards, while `cypcb-export` writes Gerber, Excellon drill, BOM and
component-placement (pick-and-place) outputs. The workspace declares
`MIT OR Apache-2.0`, which its crates inherit. That is permissive at the
workspace level; individual third-party dependencies still need their own
review. These crates make CodeYourPCB a possible manufacturing-export backend
if replacing that function is a goal, but the path still requires translating
BoardStudio data into `BoardWorld` and qualifying the resulting files. The
pinned parser maps unsupported KiCad pad shapes such as trapezoid/custom to
rectangles and records the approximation, while the board writer emits format
version token `20221018`. The README says free silkscreen text and 3D models are
not written. Those are specific comparison points for generated boards and
export completeness ([workspace manifest](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/Cargo.toml),
[`cypcb-kicad` parser](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-kicad/src/pcb_parser.rs),
[`cypcb-kicad` writer](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-kicad/src/board_writer.rs),
[`cypcb-export` API and manifest](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-export/src/lib.rs),
[README export coverage and omissions](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/README.md)).

The route engine has meaningful engineering work behind it: grid A*, negotiated
congestion, repair and smoothing; test files cover repeatability, vias, plated
through-hole pads, non-plated mounting holes, hand-drawn copper, net completeness
and board fixtures. Its benchmark harness grades multiple KiCad benchmark
boards and checks completion and DRC metrics. However, the routing record
explicitly distinguishes DRC “after” from “introduced” violations and still
reports many shorts. Therefore “complete” means all pins were connected; it
does not mean electrically clean ([autoroute benchmark test](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-autoroute/tests/benchmark_validation.rs),
[routing metrics and benchmark table](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/docs/routing.md)).

## Fit with BoardStudio

BoardStudio already has a Rust board generator/export path. Its Core export
planning validates and prepares accepted project data; its generator renderer
uses footprint generators and `NetAllocator` before assembling KiCad output.
Electrical planning separately resolves keyboard matrix rows/columns, direct
GPIO assignments, controller pins and peripheral nets. Those product decisions
should remain BoardStudio-owned if experimenting with a router
([Core generation](../../core/src/artifact/kicad/generation.rs),
[electrical plan](../../core/src/electrical.rs),
[architecture and ownership](../architecture.md)).

The likely adapter cost is manageable enough to test but not free. The public
autoroute function requires CodeYourPCB's `BoardWorld`, footprint library,
rule-set trait and routing configuration. BoardStudio has its own document,
electrical and footprint types. A direct library integration needs translation
for board size/stackup, placed footprints and pad identities, net membership,
existing copper, holes/vias, keepouts/zones, outline and board/fabrication rules.
The existing DRC assessment already found `cypcb-drc` operates on `BoardWorld`
and is not a drop-in checker ([dependency and CAD research, DRC section](dependency-and-cad-options-2026-10-08.md#rust-pcb-drc-exists-but-no-reviewed-crate-is-a-drop-in-kicad-drc-replacement)).

For a first experiment, avoid changing BoardStudio's generator. Feed its existing
generated `.kicad_pcb` into CodeYourPCB's parser and autorouter, then insert only
the new route segments/vias into a copy of the original BoardStudio board file.
This tests routing behavior while retaining original footprint source, model
paths, identities and unrelated KiCad data. CodeYourPCB has a KiCad parser and
route command, but its CLI routes by importing into `BoardWorld`; the current
BoardStudio implementation emits complete KiCad files from accepted export
inputs ([CodeYourPCB parser and route CLI](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-cli/src/commands/route.rs),
[BoardStudio artifact generation](../../core/src/artifact/kicad/generation.rs)).
If that seam proves worthwhile, a later experiment can compare a direct
BoardStudio-to-`BoardWorld` adapter against parse-and-patch.

Important product-specific cases need explicit gates:

- **Keyboard matrix boards:** Preserve the Core-assigned electrical nets,
  diode orientation, controller connections, split-board identity and all
  fabricated route widths/clearances. A router must not infer or rewrite those
  semantics.
- **Imported module boards:** Keep existing copper and module footprints
  electrically intact. A route operation should add copper while preserving
  imported board/source data and manual routes.
- **Irregular keyboard outlines and cutouts:** CodeYourPCB's world model stores
  a polygon `BoardOutline`, but the autoroute grid constructor gets its extent
  from rectangular `BoardSize`, then populates pads, zones, locked traces and
  vias. The grid source does not consult `BoardOutline`. On a concave or split
  keyboard outline, cells in the rectangle but outside the actual board can
  therefore be searchable; the current grid setup is not enough evidence that
  routes stay inside the fab outline. This is a specific blocker until a
  polygon-aware obstacle representation is implemented or the POC proves the
  case safe ([grid construction](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-autoroute/src/grid.rs),
  [`BoardOutline`](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-world/src/components/board.rs)).
- **Footprint geometry:** The importer reports unsupported pad approximations
  and refused tracks, zones and stackup data. A POC must treat any approximation
  or refusal affecting route obstacles as a hard failure, not route with a
  silently simplified board. The parser source documents these reports
  ([KiCad parser metadata and import flow](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-kicad/src/pcb_parser.rs),
  [pad conversion and approximation reporting](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-kicad/src/pcb_parser.rs)).

## Isolated proof-of-concept gates

After the concurrent refactor is settled, create a disposable branch or worktree
and evaluate a pinned CodeYourPCB revision without changing BoardStudio's
tracked lockfile or dependency configuration. The POC should use actual
BoardStudio output fixtures: a representative matrix/diode board, a board with
an imported routed module, and a board with the most difficult supported
outline/cutout geometry. Include both through-hole and SMD pads and board vias.

Pass only if all of these hold:

1. The adapter or parser round-trip preserves every input footprint, pad/net
   mapping, board stackup, hole, existing manual trace/via, zone/keepout and
   fabrication constraint needed by routing. Any parser refusal or geometry
   approximation is reported and stops routing.
2. Every fixture routes with **zero unrouted connections and zero newly
   introduced shorts**. Run the same KiCad DRC gate used by BoardStudio's
   integration checks; the route may not add DRC findings relative to the
   BoardStudio-generated source board. No routed copper may cross the real
   outline, enter a cutout, or violate hole/pad clearance.
3. The result is deterministic for a fixed board and config, and route
   extraction/patching changes only segment/via records in the copied source.
4. The library path builds for the repository's `wasm32-unknown-unknown` target
   without invoking KiCad, Java/FreeRouting or another runtime executable.
   Run routing off the UI thread. While the longest representative route runs,
   pointer/zoom and progress/cancel controls must remain responsive, with input
   acknowledgement under 100 ms on the declared target browser and test
   hardware. Set a route-completion budget before running the POC and require
   every representative fixture to meet it. Record WASM size, cold start and
   peak memory separately; the CAD worker is not a routing-performance baseline.
5. BoardStudio's accepted snapshot/revision and export qualification checks
   remain the authority for applying or saving results; stale route replies may
   not replace a newer board. These requirements follow the current worker and
   accepted-state ownership model ([architecture](../architecture.md)).

A failed fit or quality gate should end the adoption experiment and leave
CodeYourPCB as a reference for algorithm and DRC test ideas. Passing the POC
would establish only a routing integration seam; it would not establish that
CodeYourPCB should own BoardStudio board generation, nor KiCad-equivalent DRC.

## Primary sources

- [Pinned CodeYourPCB commit](https://github.com/szymontex/codeyourpcb/commit/12018cb0c6d0d927e50f863ce744e985de6553f2)
- [CodeYourPCB README at pinned commit](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/README.md)
- [CodeYourPCB routing record at pinned commit](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/docs/routing.md)
- [`cypcb-autoroute` manifest and API](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-autoroute/Cargo.toml)
- [`cypcb-render` WASM bridge](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-render/src/lib.rs)
- [`cypcb-router` FreeRouting bridge](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-router/src/freerouting.rs)
- [`cypcb-kicad` parser](https://github.com/szymontex/codeyourpcb/blob/12018cb0c6d0d927e50f863ce744e985de6553f2/crates/cypcb-kicad/src/pcb_parser.rs)
- [BoardStudio architecture](../architecture.md), [Core KiCad generation](../../core/src/artifact/kicad/generation.rs), [Core electrical planner](../../core/src/electrical.rs), and [prior dependency/DRC assessment](dependency-and-cad-options-2026-10-08.md)
