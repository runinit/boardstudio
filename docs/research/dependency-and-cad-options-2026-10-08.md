# Dependency and CAD options for BoardStudio

Researched 2026-10-08 against workspace HEAD `ef45d94d7` and the working tree as
observed today. A large refactor is active in another tool, so this is a
read-only snapshot and may not describe its in-progress state. The GitNexus
index was three commits behind HEAD; its query/context results were used only
to locate concepts, then checked against source. No index, build, or test state
was changed. No candidate kernel was compiled or newly tested for this note.

## Findings

### KiCad is a validation dependency, not an app runtime dependency

The browser application emits and consumes KiCad board text from Rust. It does
not need to launch KiCad for ordinary editing or export. KiCad CLI 10 is listed
as a development prerequisite because integration tests ask KiCad to parse
generated boards; the default test step runs those Rust tests. KiCad also
remains the intended downstream tool for routing and fabrication. The official
format is a documented s-expression format, while the CLI additionally runs
DRC and exports manufacturing outputs such as Gerbers, drill files, and BREP.
Replacing the format parser therefore would not replace those engineering
functions. [Project README](../../README.md), [KiCad board format](https://dev-docs.kicad.org/en/file-formats/sexpr-pcb/),
[KiCad CLI 10](https://docs.kicad.org/10.0/en/cli/cli.html).

The lowest-risk way to remove KiCad from the standard developer install is to
make the KiCad-backed integration tests opt-in locally and keep them in a
dedicated KiCad 10 CI lane. That preserves the external compatibility oracle.
Bundling KiCad CLI is possible for native tooling but adds a large cross-platform
package, library/data discovery, updates, and redistribution review; it cannot
be launched by a static browser app. Eliminating the CLI entirely would mean
owning equivalent DRC and CAM behavior or accepting that the app only validates
its own serialization. A parser alone is not an obvious win: the app already
has a Rust s-expression parser seam, and parser coverage does not provide DRC
or fabrication validation. Rust DRC and design-readiness projects do exist
(see below), so a future CLI replacement is not limited to writing every rule
from scratch; each candidate has important scope and integration limits.
KiCad is GPL-3.0-or-later; any binary bundling
proposal needs review of the exact distributed components and their licenses.
[KiCad license](https://gitlab.com/kicad/code/kicad/-/blob/10.0.0/LICENSE.GPLv3),
[current Rust KiCad model project](https://github.com/adom-inc/kicad_lib),
[Rust parser crate](https://crates.io/crates/kicad-parser).

### Rust PCB DRC exists, but no reviewed crate is a drop-in KiCad DRC replacement

There are Rust projects that perform board checks. They differ in whether they
check authoring geometry or manufacturing outputs, what board model they
accept, and how much evidence exists for production use. A pure-Rust
implementation can remove a KiCad CLI requirement for some checks, but it does
not automatically provide KiCad's complete rule coverage, zone-fill behavior,
fabrication exports, or compatibility oracle.

| Candidate | What its source actually checks / accepts | License and execution fit | Assessment for BoardStudio |
| --- | --- | --- | --- |
| [HyperDRC](https://github.com/timschmidt/hyperdrc) | Rust library and CLI for design-readiness/manufacturing-package checks. Accepts Gerber layers/directories and package archives, plus KiCad PCB and sidecars (Excellon, IPC-356); checks are divided into layer, drill, board, assembly, and other readiness families. | MIT. Its manifest includes Rust geometry crates, Arrow/Parquet, bundled SQLite, compression/archive, XML and `libc`; the project documents a native CLI. WASM/browser support is not established. | Best candidate for a bounded evaluation as a supplementary manufacturing/readiness validator, especially if BoardStudio can supply the package artifacts. It is a young project and its broader readiness checks should not be mistaken for complete KiCad DRC parity. |
| [cypcb-drc](https://github.com/szymontex/codeyourpcb/tree/main/crates/cypcb-drc) | Substantial Rust rule engine: clearances, drill/trace widths, unconnected and unrouted pins, shorts/net splits, keepouts, edge/hole/courtyard/solder-mask/silk checks, trace current/impedance, stackup, flex rules and more. It runs against CodeYourPCB's `BoardWorld` ECS model and design-rule presets, not arbitrary KiCad files. | Workspace license is dual MIT/Apache-2.0; dependencies are Rust crates including Bevy ECS and spatial indexing, with optional Rayon. The source has a `wasm32` timing branch, showing WASM awareness, but that alone is not proof the crate's full dependency graph and APIs build and work in BoardStudio's browser. | Useful reference for rule coverage and test cases; not a drop-in library for BoardStudio without translating its board into `BoardWorld` or adopting its model. Inspect how rule semantics compare before borrowing results. |
| [DesignGuard](https://docs.rs/designguard) | KiCad file validation with schematic and PCB checks such as decoupling, pull-ups, trace widths, EMI and IPC-2221-oriented checks. Its library API accepts file paths and parses KiCad designs; its published description emphasizes design-intent/preflight findings as well as geometry checks. | MIT, version 0.1.1. Manifest includes Tokio, Reqwest, Petgraph and UUID among other Rust dependencies; published docs target Linux. No WASM support is documented. | Relevant as an existing Rust KiCad-file validation library, but early version and advertised rule scope do not establish general geometric DRC parity. Worth a small fixture-based comparison if schematic-aware checks are wanted; less compelling for a browser-side pure-Rust backend. |
| [gdscheck](https://github.com/aesc-silicon/gdscheck) | A DRC engine for GDSII integrated-circuit layouts with YAML rules and PDK decks. It reads GDSII, not PCB/KiCad board geometry. | AGPL-3.0-or-later; Rust CLI, Rayon and geometry dependencies. It explicitly calls itself experimental and not yet qualified for tape-out. | Not a PCB candidate. It is a useful example that Rust DRC engines exist in adjacent electronic-design domains, but its input format, rules and licensing do not fit this use. |

The strongest near-term path is to define BoardStudio's own minimum DRC contract
and compare it against KiCad on representative generated boards, using KiCad as
the reference oracle. For an external Rust candidate, start with HyperDRC on
manufacturing artifacts and DesignGuard only if schematic/design-intent checks
are in scope. Treat cypcb-drc as a source of rule ideas and fixture scenarios,
not a reusable backend unless a model adapter proves small and trustworthy.
Keep any alternative validator's coverage explicit; a clean report from a
partial checker must not be presented as “DRC passed” without naming its rules.

Sources checked in first-party manifests and library/check code, rather than
README feature lists alone: [HyperDRC Cargo manifest](https://raw.githubusercontent.com/timschmidt/hyperdrc/main/Cargo.toml),
[HyperDRC library layout and input model](https://raw.githubusercontent.com/timschmidt/hyperdrc/main/src/lib.rs),
[HyperDRC check families](https://raw.githubusercontent.com/timschmidt/hyperdrc/main/src/checks/mod.rs),
[cypcb-drc manifest](https://raw.githubusercontent.com/szymontex/codeyourpcb/main/crates/cypcb-drc/Cargo.toml),
[cypcb-drc rule registration](https://raw.githubusercontent.com/szymontex/codeyourpcb/main/crates/cypcb-drc/src/lib.rs),
[cypcb-rules manifest](https://raw.githubusercontent.com/szymontex/codeyourpcb/main/crates/cypcb-rules/Cargo.toml),
[DesignGuard published manifest](https://docs.rs/crate/designguard/0.1.1/source/Cargo.toml.orig),
[DesignGuard API](https://docs.rs/designguard),
[gdscheck manifest](https://github.com/aesc-silicon/gdscheck/blob/main/Cargo.toml),
[gdscheck project and maturity statement](https://github.com/aesc-silicon/gdscheck).

### CAD kernel replacement is feasible as an experiment, with a major import gap

The current geometric kernel is already a B-rep engine: OpenCascade (OCCT)
represents geometry with boundary-representation solids. Cadrum provides the
Rust/WASM integration, while the kernel itself is C++. So the requested change
is specifically **replace the OCCT B-rep kernel with a Rust-native B-rep
kernel**, not add B-rep geometry to a non-B-rep backend. Cadrum already avoids a
user-installed CAD runtime. The remaining build complexity is concentrated in
the pinned WASI/C++ toolchain, container, cached kernel archive, and native OCCT
oracle. Existing Cadrum research records current-backend validation; those
results do not validate replacement candidates. [CAD architecture](../architecture.md),
[Cadrum assessment](cad/cadrum-assessment.md),
[CAD build setup](../../cad/README.md).

| Option | Current evidence and capability | License and maturity | Main BoardStudio risk |
| --- | --- | --- | --- |
| Cadrum 0.8.20 + OCCT 8.0.1 | Production Rust/WASM adapter; case solids, STEP import/export and tessellation are validated in the existing worker | Cadrum MIT; OCCT LGPL with exception notices | Keeps a C++ kernel and its build pipeline; this is the known-good baseline |
| Monstertruck 0.4.1 / current upstream | Rust B-rep, booleans, STEP I/O, tessellation, healing and fillet/chamfer APIs; WASM bindings exist. Prior pinned version already passed load/validation for 53 generated STEP exports | Apache-2.0; active fork, but geometry/API maturity still needs application proof | Touching/coplanar boolean regressions; arbitrary vendor STEP failures described below; current browser build/parity not established |
| Truck | Rust B-rep with STEP I/O, booleans, meshing and fillets | Apache-2.0; active project | Boolean scope excludes tangent-face intersections; fillets are narrowly constrained |
| brepkit | Rust/WASM B-rep with STEP, booleans, tessellation, fillet/chamfer and assembly APIs | Current line AGPL-3.0-only or commercial; older permissive line does not imply current feature parity | Difficult booleans may fall back to meshes, losing analytic surfaces or watertightness |
| Fornjot | Rust B-rep research implementation | Development shut down 2026-06-19 | Not a sensible new dependency absent a maintained successor |

These upstream feature descriptions are source evidence, not quality scores.
Sources: [Cadrum/OCCT pinned integration](cad/cadrum-assessment.md),
[Monstertruck](https://github.com/virtualritz/monstertruck),
[Truck shape operations](https://github.com/ricosjp/truck/blob/master/truck-shapeops/src/lib.rs),
[brepkit](https://github.com/andymai/brepkit/blob/main/README.md) and its
[stability policy](https://github.com/andymai/brepkit/blob/main/STABILITY.md),
[Fornjot shutdown announcement](https://www.fornjot.app/blog/shutting-down-fornjot/).

There is already a useful kernel seam in
[kernel.rs](../../cad/wasm/src/model/kernel.rs): `CadKernel` abstracts case
build, STEP import/export, and meshing, and the fixture gate is generic over
implementations. The same file documents that preview/export caches and
keycap generation still hold `cadrum::Solid` directly, so the seam does not
cover all production operations. It gives a starting point for a staged
experiment, but a full cutover also needs to address those direct dependencies.

The strongest Rust-native prototype candidate is currently Monstertruck, the
Truck fork. Its upstream project documents B-rep modeling, solid operations,
healing, fillets/chamfers, STEP I/O, meshing, and a WASM binding crate under
Apache-2.0. Its README also records a boolean rewrite being reverted after
regressions on punched and adjacent cubes, directly relevant to touching and
coplanar case geometry. Feature lists are not application validation. [Monstertruck upstream](https://github.com/virtualritz/monstertruck).

Crucially, Monstertruck is not an untested idea here. The resolved 2026-10-05
spike tested 0.4.1 against 53 Cadrum-generated STEP exports and a 33-file vendor
corpus. All 53 generated exports loaded and validated; 47/53 initially matched
the comparison measurements. The remaining six were later traced to a
serde_json float-rounding difference in the spike harness, and the corrected
inputs matched under the OCCT oracle. That is not evidence that Monstertruck
independently matched all 53 baselines. The vendor corpus
results exposed real limits: only 16/33 files passed the spike's validity and
measurement checks, several had substantial tessellation openings, three had
placement mismatches, and the slowest import took 129.5 seconds. OCCT itself
also rejected 17 vendor files, with some disagreements between kernels. The
spike therefore supports a narrow conclusion: Monstertruck is a credible
reader/oracle for BoardStudio-generated geometry, but not a drop-in arbitrary
vendor STEP importer. It did not establish production browser kernel parity.
See the resolved spike record in commit
[`767abb194`](https://github.com/runinit/boardstudio/commit/767abb194); its
measurements are historical and tied to its pinned 0.4.1 implementation.

Truck upstream has explicit tangent-face boolean limits and a constrained
single-edge fillet. Brepkit documents broader STEP, boolean, fillet, tessellation
and assembly support, but difficult booleans may fall back to meshes and lose
analytic surfaces or watertightness. Its current license is AGPL-3.0-only or
commercial; older permissive versions should not be assumed to contain current
features. [Truck shape operations](https://github.com/ricosjp/truck/blob/master/truck-shapeops/src/lib.rs),
[brepkit README](https://github.com/andymai/brepkit/blob/main/README.md),
[brepkit stability policy](https://github.com/andymai/brepkit/blob/main/STABILITY.md).

The practical investigation should separate two targets. First test whether a
Rust kernel can generate our own case/keycap solids and produce preview meshes
and STEP that round-trips through OCCT. The generated geometry is controlled
and the Monstertruck oracle spike already provides substantial starting
evidence. Separately test arbitrary vendor STEP import, with declared units,
assembly placement, curved surfaces, bounds, and timing; the spike shows this
is the harder compatibility problem. Do not repeat the 0.4.1 corpus spike from
zero. Refresh the exact current revision and only investigate deltas that could
change its prior failures.

Before a production cutover, require an isolated worker proof of concept to
pass the existing CAD fixtures (holed and concave contours, tray/lid, mounts,
gasket, multi-body, and the 63-solid transformed component), plus touching or
near-tangent faces, thin walls, and curved components. Compare solid count,
closed-shell status, volume, bounds, STEP reimport and mesh correctness against
the current OCCT path. Measure release WASM size, cold initialization, peak and
repeated-operation memory, and fixture latency. Reject silent mesh fallback for
manufacturing STEP output. Keep the current worker contract and OCCT differential
oracle during the experiment. The prior spike's failures mean a generated-case
cutover and an arbitrary-import cutover need independent acceptance decisions.

### Other overhead: distinguish user runtime, build tools, and checks

| Dependency or layer | Where it applies | Assessment |
| --- | --- | --- |
| Python 3.11 | Build/content/check scripts; not app runtime ([check runner](../../scripts/check.py), [CAD build helper](../../cad/scripts/cadrum_build.py)) | Useful orchestration glue, but still a developer prerequisite. Low priority unless reducing onboarding prerequisites is the objective; replacing scripts with Rust would mainly reduce toolchain count. |
| Dioxus CLI, wasm-pack, wasm-bindgen, Rust WASM target | Build and packaging | Required by the Rust browser app. wasm-bindgen generates JavaScript glue; a Rust source policy cannot remove the browser's JavaScript execution environment. Simplify only with evidence that a tool duplicates another. |
| Chrome/Chromium and ChromeDriver | Mounted WASM/browser tests | Verification-only, not shipped to users. Keep a browser CI lane; developers can run native checks without it. |
| KiCad CLI | Native integration tests and optional manufacturing validation | Compatibility/validation oracle, not browser runtime. Split its test lane if local setup friction matters. |
| WASI SDK, C/C++ compiler, CMake/Ninja, container runtime, native OCCT | CAD kernel build and oracle ([toolchain container](../../cad/wasm/Containerfile), [kernel boundary](../../cad/wasm/src/model/kernel.rs)) | Highest-confidence complexity candidate if the goal is pure Rust and easier CAD builds. A Rust B-rep experiment targets this cluster but introduces geometric compatibility risk. |
| Generated WASM glue and bundled models/fonts/catalogue data | Shipped assets | Asset size and update responsibility, not developer dependencies. Measure before pruning; bundled data enables offline operation. |
| Rust workspace crate count and worker boundaries | Compile architecture and runtime isolation | Crate count alone is not runtime overhead. Keep the feature crates unless compile-graph measurements or ownership evidence identify a real cost. |
| Rhai scripting ([core manifest](../../core/Cargo.toml), [script engine](../../core/src/script.rs)) | User-authored geometry scripts, compiled for native and WASM | Only a candidate if the product removes user scripting. Its current presence is purposeful; measure binary impact before treating it as avoidable. |
| `kiutils_sexpr` ([core manifest](../../core/Cargo.toml)) | KiCad s-expression parsing in Rust | Narrow parser dependency; not the KiCad application or a replacement for DRC/CAM. Low priority. |
| `three-d` renderer ([renderer manifest](../../renderer/Cargo.toml)) | Rust/WASM scene drawing | Current renderer, distinct from retired Three.js; replacement is a product/runtime decision, not leftover TypeScript overhead. |
| Asset delivery and worker serialization | Models, fonts, catalogue data, WASM startup and worker request/reply paths | Plausible performance/complexity area, not a measured finding in this research. Measure download/parse costs, cold startup, repeated scene payload sizes and clone/transfer costs before pruning offline assets or changing boundaries. |

The check runner confirms these categories: default checks include Python
tooling, Rust tests/lint, web build, browser tests and the CAD tests; the web
build and browser checks use distinct paths. CI installs KiCad for the native
test job, while the browser job installs Chromium and uses the prebuilt CAD
toolchain artifact. This is why “pure Rust” should be defined by runtime and
distribution goals, not by counting Python scripts or build-only tools.
[Check commands](../../scripts/check.py),
[CI checks](../../.github/workflows/check.yaml),
[web build](../../scripts/build-web.py).

Ranked follow-up candidates for question 3 are therefore: (1) reduce the CAD
toolchain burden if the pure-Rust goal is firm; (2) measure asset delivery and
worker serialization to test whether payloads or repeated copies create real
cost; (3) reduce Python only if onboarding is a priority; (4) consider Rhai only
if user scripting is no longer a product requirement. The last three are
hypotheses to investigate, not established overhead findings. A narrower
geometry route is also possible: Rust 2D region preparation, extrusion, and
tessellation may cover controlled generated cases with fewer general 3D
booleans. It does not solve arbitrary STEP input or produce equivalent curved
analytic STEP output by itself.

## Suggested order

1. If the immediate goal is easier onboarding, isolate KiCad-dependent tests
   and keep a required dedicated CI compatibility lane. No app code migration
   is needed to remove KiCad from the baseline local setup.
2. For CAD, run a focused current-Monstertruck delta check on generated cases
   and browser/WASM support; use the existing 0.4.1 vendor-corpus results as
   known limitations. Do not make arbitrary STEP import a prerequisite for the
   first production-kernel experiment unless it is explicitly in scope.
3. Only if the CAD proof of concept meets geometry, STEP, browser, size,
   performance and license gates should the team schedule a backend migration.
4. Audit the Python/build scripts and WASM asset budget separately. Remove
   maintenance machinery only when its operation has a clear replacement and
   the relevant build/release checks remain reproducible.

No code, dependency, lockfile, Git index, or build output was changed for this
research. External project claims above point to first-party sources; upstream
capability statements are not independent validation. The in-progress refactor
may alter the local inventory after this snapshot.
