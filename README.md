# Board Studio v2

This is a new offline-first application. It reads and writes only
`boardstudio/v2` projects. There is no old-project importer or conversion path.
Earlier development shapes are unsupported; only the current generated contracts
are accepted. GitHub Pages builds this application.

## Package boundaries

| Package | Owns |
| --- | --- |
| `contracts` | Millimetre Y-up document and revisioned worker protocol |
| `core` | Rust document transactions, outline geometry, constraints, scripts, matrix groups, undo/redo and WASM boundary |
| `kicad` | KiCad 10 board and footprint serialization, footprint import and validation |
| `ergogen` | Bundled generator sources, model assets, provenance, and the trusted Ergogen runtime adapter |
| `cad` | Lazy OpenCascade case solids, mesh preview and STEP export |
| `app` | Keyboard workbench, browser persistence, workers and file packaging |

The app calls its worker, the worker calls the core, and exporters consume a
committed snapshot. Exporters must reject a stale revision. The same resolved
contours feed the 2D canvas, PCB edge, DXF and case construction.

## Ergogen library

Parts includes 37 active bundled Ergogen generators in `ergogen/library/`.
`pnpm --dir ergogen test` checks all 186 tracked library files against the
reviewed SHA-256 inventory and the generated runtime against the source.
`pnpm build` regenerates the trusted browser catalogue. User supplied JavaScript generators are not
executed.

Generator settings stay on part definitions. Net and anchor bindings can vary
per placed part. Utility generators that emit zones, routing, or text become
KiCad board objects when their parts are placed; they cannot be represented as
standalone `.kicad_mod` files. A footprint-library export lists these entries
in `BOARD-UTILITIES.txt`, while board export includes their output. Unsupported
or missing model paths fail export explicitly. Model previews load STEP/STP
files on demand; WRL is kept for KiCad export and STL sources are retained.

Project export offers **Embed used models**, enabled by default. It includes
only bundled models used by placed parts. Turning it off leaves bundled models
linked to the installed catalogue; imported local assets remain embedded.

## Run and validate

Install Node 24+, pnpm 11.26.0, Rust with `wasm32-unknown-unknown`,
`wasm-pack` 0.15.0, and KiCad CLI 10. Then run:

```sh
pnpm install --frozen-lockfile
pnpm dev
pnpm check
pnpm test:perf
```

`dev` builds the Rust WASM core and starts Vite. `check` runs repository hygiene, native core,
KiCad and CAD tests, app unit tests, generated-output and Rust-boundary checks,
type checking, the app build, and functional Chromium browser tests. Timing-only
scenarios run separately through `test:perf`. `precommit` prepares WASM and runs
app and CAD type checks; it is not a substitute for `check`.
`check:repo` checks authored module reachability, unused exports and runtime
dependencies, and local documentation links. Its entrypoint exceptions and
limitations are recorded in [`docs/repository-cleanup.md`](docs/repository-cleanup.md).
Install Playwright Chromium with `pnpm --dir app exec playwright install
chromium` if needed. `pnpm test:e2e:pages` checks the production build under
`/boardstudio/` (also included in `check`). Run the development-server CAD
regression with
`pnpm --dir app test:e2e:dev`. Set `BOARDSTUDIO_CHROMIUM=/usr/bin/chromium` to use a local
Chromium binary.

`test:perf` rebuilds WASM and the app before five serial, same-host
Chromium sessions, then runs pointer, matrix, and outline latency scenarios.
It compares the median session p95 against the captured mounted-workbench baseline. The fixture, limits, and results are in
[`docs/performance-baseline.md`](docs/performance-baseline.md).
It also records live case numeric edits, Undo, and mount/gasket releases as
diagnostics. Chrome versions are recorded without pinning a particular version.
See the [live-preview review and remaining performance work](docs/live-preview-ui-review.md)
for measurements and the exact-refinement target that remains unmet.

The browser benchmark lives at `/bench.html`. It measures a worker preview
request through a painted 2D outline frame for 100 and 200 key fixtures. It
warms ten samples and reports the next hundred. The browser test gates the
95th percentile at 100 ms and 200 ms respectively.

## Automatic outlines

New boards follow the current keycap and component envelopes with a 4 mm margin,
2 mm fillets, and 10 mm bridges between disconnected groups on the same board.
The **Outline** button opens margin, sharp/fillet/chamfer, and bridge settings.
Tight corners are fitted locally; findings report the requested and applied sizes.
Existing procedural envelopes now follow the layout, retaining their margins and
membership; envelopes saved without finishing settings retain sharp corners.
Explicit rectangles and polygons remain authored geometry.

Switch definitions and instances can supply `keycap: {x, y}` dimensions. Built-in
switches default to 18 × 18 mm; custom switches without dimensions use their
courtyards and show a finding. The part inspector can override keycap dimensions,
exclude a part, or give it a separate nonnegative edge margin. A zero margin
supports edge-mounted parts; exclusion does not validate pad support.

**Draw addition** and **Draw cutout** create optional board features. Click snapped
points (Alt bypasses snapping), then Enter or double-click to close; Escape cancels.
Automatic interior voids are filled before authored cutouts are applied. Invalid
polygons block outline exports. All consumers use the committed board contours,
including PCB Edge.Cuts, SVG/DXF and case/plate construction.

## Design workbench

**Add** offers a matrix setup flow and a searchable component catalog. Choose a
matrix’s rows, columns, and key assembly before placing it; choosing a component
starts a standalone cursor preview.
Click to place on the active board using the selected snap increment; Alt bypasses
snapping. Arrow keys move the preview and Enter places it. Escape cancels without
changing the project. Detailed footprint editing and models live in **Parts**;
**Apply to selected key** adds a variant or companion to a selected matrix cell.

The tree defaults to Board → Matrix → Columns → Keys → Components. Its local
Columns/Rows preference changes presentation only, and summaries count enabled
keys. Empty slots remain selectable for restoration. The inspector follows the
selected matrix, row, column, key, or component. Rename boards and matrices in
their inspectors. **Delete matrix** removes the container and its members in one
undoable edit, including an empty matrix. Hover and selection outlines follow the
current scope. Canvas scope and Snap controls stay fixed during pan and zoom.

In **Parts**, the left library groups searchable **Key Assemblies** and
**Components** by category. The right inspector edits the selected definition.
The center switches between compiled **2D footprint** geometry and an attached
**3D model**; missing models have an explicit import prompt. Interactive preview
uses STEP; WRL attachments remain available for export and show that limitation.
Bundled model previews report missing files or download failures with a retry
action. Imported generator settings are grouped into dimensions, footprint
options, identification, connections, model placement, and advanced parameters.

The single-row header provides direct Layout, PCB, Case, and Parts navigation;
at 820px and below these use the native Workspace selector. Project menu →
Setup guide opens the guide, and Project menu → Workspace settings owns
appearance and restoring the navigator and inspector panel layout. Both pages
provide an explicit Back/Close path. Export remains a dedicated header action;
Parts and Export provide explicit returns to the prior design view.
Keycap overlays follow the current saved member poses while disabled slots
retain their parametric frame. Starter metadata uses the same negative-Y row
direction as its existing switches. Editing a matrix retains its switch
identities and net references.

Case starts with **Live preview** enabled. Valid edits refine automatically while
the last usable geometry stays visible. Pause uses **Update preview** for manual
updates; **Cancel** pauses an active generation. Gasket and case mount drags show
a draft, save once on release, and cancel with Escape. Drafts cannot be exported;
manufacturing export still requires current committed geometry and validation.

## Current handoff

The app can create a v2 project with multiple boards and place matrices with
user-selected dimensions. Each cell gets a switch and diode; MX/Choc, solder,
hotswap, and RGB presets change the assembly. Presets can update a matrix or
create a separate design. A matrix can map its diode row-to-column or
column-to-row. The CAD tree selects matrices, rows, columns, keys,
and components; dragging rows/columns sets independent offsets. The column
inspector also exposes cumulative **Stagger** (mm) and **Splay** (degrees): stagger
shifts this and following columns; splay rotates them around this column’s
nominal first-key anchor, carrying later column pivots. Matrix mirror and
rotation apply afterward. These are stored as optional `columnStaggers` and
`columnSplays` arrays, defaulting to zero for existing projects.
Deleted keys disappear from the canvas; restore them by selecting their empty
slot in the tree and checking **Enabled** in the key inspector. The editor has fractional
pitch snapping, Alt bypass, wheel zoom, Space-pan, and fit-to-design. The
Parts workspace previews compiled 2D footprints in the workspace as
generator settings change and loads attached 3D
models on demand. It can also place and group parts,
author component pads, link placements with offset or mirror constraints,
edit outlines, map pads to nets, bind local STEP or WRL models, build
plate/tray/lid case bodies, and save a project ZIP with hashed assets. It
supports keyboard movement of focused parts at 0.1 mm or 1 mm steps, with
one undo step per keypress. Project ZIP imports bound their expanded size.
It exports board outlines as SVG/DXF, placed KiCad boards and a KiCad footprint
library with relative model paths, and analytic case STEP. KiCad
board export includes placed parts and any tracks, vias, zones, and keepouts
emitted by bundled Ergogen utilities. Remaining routing is done in KiCad.
Browser storage and the app shell work offline after the first
load. The CAD kernel loads when a case preview, imported STEP component mesh,
or case STEP is requested.

See [current architecture and validation](docs/architecture.md) for module ownership,
compatibility boundaries, and maintained check commands.

Validation covers deterministic core transactions, KiCad 10 parsing and DRC,
OpenCascade STEP reimport, browser editing/export, and outline preview latency.
The 6 × 5 MX/Choc solder/hotswap assembly geometry passes KiCad DRC without
nets. Assigned but unrouted matrix nets still require routing and DRC in KiCad.
Fabrication readiness still requires review of the routed KiCad design and
mechanical fit. The 3D component view is visual; it does not prove clearances.
The KiCad footprint importer supports a documented subset, and mirror
constraints reflect placement without changing asymmetric footprint geometry.
The v2 CI workflow runs on pull requests and does not deploy.

### Inspector organization

Parts inspectors focus on placement and part options. Canonical bundled generator
assembly choices and keycap dimensions are visible; connection
bindings, model placement and advanced footprint parameters are disclosed separately.

The catalogue owns footprint import and **New custom component**. Raw courtyard
and pad authoring is available only through **Edit footprint** for non-generator
custom/imported definitions. The selected catalogue item is the sole editing target;
there are no separate definition or model-target selectors. Key assemblies show
only their assembly description and placement action. Geometry scripts live under
**Project → Geometry scripts**.

Design inspectors lead with position and layout. Outline overrides, optional
constraints, assembly settings and matrix actions have named disclosure sections.
Electrical connections stay in PCB. Case inspectors show wall dimensions only for
trays/lids and disclose mounting and gasket details. These changes reduce routine
configuration without changing project formats, footprint identities or exports.

## Repository maintenance

The former v2 packages are now the root workspaces. Package names and the
project/storage formats are unchanged. See the
[repository cleanup inventory](docs/repository-cleanup.md) for retained
coverage decisions and historical documentation dispositions.

The trusted library is a provenance snapshot, including its original maintenance
scripts. Its nested package is not a workspace: those historical scripts are
not supported root commands. Run `pnpm --dir ergogen test` for current validation.
When deliberately updating a bundled source, review the upstream change and
licenses, update the relevant source provenance, and update the corresponding
entry in `ergogen/library-integrity.json`. Never refresh hashes to hide drift.

GitHub Pages builds `app/dist` on main pushes or manual dispatch. Relative asset
URLs support the repository subpath; the build requires Rust/WASM and wasm-pack
in addition to Node and pnpm. Deployment is performed only by the Pages workflow.
