# Footprint generators in Rust

Planned: 2026-10-05. Inspected revision: `3cdeb2ac2`.
Status: steps 1–6 implemented. Native, WASM and targeted headless browser
verification is recorded below; a production UI/offline smoke test remains pending.

This plan covers items 1 and 2 of the
[TypeScript and Node removal assessment](typescript-node-removal.md): the
footprint generator provider and the PCB preview worker. Afterwards, footprint
generators exist only in Rust and no footprint generation path executes
JavaScript or requires Node. Decisions are recorded in
[ADR 0003](../adr/0003-rust-footprint-generators-in-core.md) and
[ADR 0004](../adr/0004-document-format-version-and-id-rename.md).

## Current implementation

| Piece | Size | Role |
| --- | --- | --- |
| 36 JavaScript generators: 24 from [ergogen/library](../../ergogen/library), 12 from its vendored infused-kim directory | ~9k lines | `params` defaults and a `body(p)` producing KiCad S-expression text |
| `defaultModels.mjs`, `generate.mjs` | ~150 lines | Bind default 3D models and model-selection rules; build `catalogue.mjs` |
| `ergogen/src/index.ts` | 411 lines | Parameter schema, catalogue, render context, form parsing, pad and courtyard geometry, definition normalization, terminal discovery, model IDs and bindings |
| `kicad/src/ergogen.ts` | 75 lines | Arc upgrade, model-path rewrite, footprint/object split |
| `preview-generator-worker.ts` | 196 lines | Envelope validation, net allocation, job execution |
| Two packaging scripts in `scripts/web` | ~90 lines | Strip types with Node and stage the modules under `web/assets` |

The page loads the generator module with dynamic `import()` in
[footprint_graphics.rs](../../web/crates/ui-shared/src/footprint_graphics.rs),
[parts/catalogue.rs](../../web/src/presentation/parts/catalogue.rs) and
[bundled_models.rs](../../web/crates/runtime/src/bundled_models.rs). Board preview, case
preview and KiCad export each run Core `PreparePreview` (or the export
equivalent), the JavaScript worker, then Core `FinishPreview`.

## Decisions

- **Rendering inside Core.** Board preview, case preview and KiCad export
  each become one Core request. Core renders jobs and allocates nets
  internally. `ErgogenJob`, `ErgogenJobResult`, `FinishPreview`, the worker,
  its envelope validation and page-side reply checks are removed. Page-side
  executor-epoch and currency checks remain.
- **A `footprints` crate.** `core` depends on it, and the page WASM calls it
  directly for drawings, parameter schemas and the catalogue. Measure the page
  WASM size change when the framework lands.
- **Mechanical port.** One module per generator emits the same S-expression
  text, which then passes through the existing parse step. Reuse Core's arc
  upgrade (`upgrade_legacy_arcs` in `core/src/artifact/source.rs`, currently
  private) instead of porting the TypeScript one. Step 2 makes it shared and must
  show its output matches the recorded export goldens, including six-decimal
  rounding; if it does not, port the TypeScript upgrade instead.
- **Declared metadata.** Each generator declares its kind, display name,
  matrix terminals, keycap-envelope parameters and explicit parameter types
  (number, boolean, string, net, array). No generator uses anchor or object
  parameters; the pose-derived render values (`p.point`, `p.at`, `p.xy`) are not
  parameters. Net parameters are declared, not inferred from a missing default.
  Remove the source-ID regex classification, the hardcoded switch list and the
  duplicate display-name table in the Parts catalogue. Declared kinds must
  reproduce the `catalogue` entries recorded in the golden fixtures; any
  difference is a listed deviation.
- **Side.** Every generator declares a `side` parameter with default `F`, so the
  schema is the only source of the layer a body renders on. Part-side mirroring of
  coordinates still follows the part. The 1→2 migration sets `side` to `B` on
  each back-side part that has a generator and no saved `side`, preserving
  today's output. Seven generators show `B` (or, for the keepout zone, `F&B`) as
  their schema default today, which the old renderer never used; this becomes `F`
  (see Deviations).
- **Generator lookup.** A definition without a `generator` passes through
  normalization untouched. A definition whose generator source is not in the
  registry, or whose version is not `bundled-1`, is a typed `UnknownGenerator`
  error in parameters, render, normalization and the catalogue.
- **Fixed quirks.** JavaScript coercion and formatting quirks are corrected,
  not reproduced. Each correction is a listed deviation. Rendering rejects values
  that do not match the declared parameter type. No correction may change the
  pad count or order of an existing definition without its own migration, because
  saved pad IDs and nets are matched by index. Generator version `bundled-1` is
  retained.
- **Typed generator errors.** Errors are typed with generator and parameter
  fields. The user-facing message keeps the current text as its baseline, even
  where that text does not name the generator or parameter. Messages produced by
  the JavaScript engine (such as `utility_router`'s JSON parse failure) are not a
  baseline.
- **Licensing.** Each ported module keeps the SPDX identifier (MIT or
  CC-BY-NC-SA-4.0) and author attribution of its source. The crate documents its
  mixed licensing.
- **Identifier rename.** After cutover, `ergogen:<namespace>/<name>`
  definition IDs become `generator:<namespace>/<name>` and `ergogen:model:<path>`
  asset IDs become `bundled-model:<path>`. Generator source IDs such as
  `ceoloide/switch_mx` and `bundled-1` are unchanged, with one exception:
  `ceoloide/utility_ergogen_logo` becomes `ceoloide/utility_logo` (display name
  and footprint name likewise), so that no persisted identifier contains
  `ergogen`. The 1→2 migration rewrites it in saved definitions and parts.

## Sequence

1. **Golden baseline.** Complete (`80b8e460e`, `footprints/tests/golden`; see its
   README). A temporary harness recorded, for every generator: `parameters`,
   `catalogue`, render output, geometry, normalized definitions, terminals,
   model bindings, KiCad export forms and net snapshots, plus provider and
   preview-worker scenarios. Inputs were defaults, each boolean flipped, sides F
   and B, rotations 0, 90 and 37 degrees, marker nets, numeric-string coercion,
   validation-error cases and every generator configuration in the bundled demo
   projects: 820 generator cases and 28 worker scenarios. Anchors are covered
   only through the part pose, because no generator declares an anchor
   parameter. The fixtures cannot be regenerated after deletion; step 5 rewrites
   them mechanically for the renames.
2. **Provider framework.** Complete. The `footprints` crate implements form
   handling, the render context, geometry and courtyard joining, normalization,
   terminal discovery, model IDs and bindings, model-path rewriting, net
   allocation, the generator registry and typed errors. It is verified without
   any generator port: the recorded render text runs through parsing, geometry,
   model references and export and matches the goldens (803 geometry cases, 783
   export cases), the render context matches 40 recorded cases, and JavaScript
   number formatting, `Math.sin`/`cos`/`hypot` and parsing match V8 bit for bit
   (`libm`; `serde_json` needs `float_roundtrip`). Of the 28 worker goldens,
   those for net allocation, repeated reserved names and the 32-bit net limit
   carry over; about 20 envelope-validation scenarios retire with the worker.
   Findings:
   - The crate cannot depend on `core` (which depends on it), so it defines the
     wire types it needs (`Pad`, `ModelBinding`, `PartKind`, ...) with the same
     serialized form as `core::model`. At cutover core re-exports them.
   - Arcs stay in Core: `upgrade_legacy_arcs` reproduces the old worker's arcs
     on all 20 recorded cases (`core/src/artifact/source.rs` test), so the crate
     does not upgrade arcs and Core applies its function to exported text.
   - Commit `3cdeb2ac2` deleted `cad/bench/fixtures`, which four core tests still
     read, so `cargo test` for `core` did not compile. The four
     `internal-gasket-v1` fixtures now live in `core/tests/fixtures`.
   - Trigonometry and number formatting also match V8 when built for
     `wasm32-unknown-unknown` (153 checks run in Node).
   - The page WASM size change is not measured yet; the crate is not linked into
     production. Measure it at step 4.
3. **Generator ports.** Complete. Port in batches: utilities and mounting holes;
   diode, LED, reset and power switches; MX, Choc and KS27/KS33 switches;
   controllers, displays, encoder and connectors; infused-kim. All 36 generators
   match all 820 golden cases (render text and nets, geometry, normalization,
   model references, export, schema and catalogue) as parsed form trees with
   numbers compared as exact text and whitespace ignored, apart from the listed
   deviations (`footprints/tests/generators.rs`). The model-binding assertions of
   the footprint-library Node tests are ported to
   `footprints/tests/library_assertions.rs`; the tests for the three vendored
   generators the catalogue excludes (`choc`, `diode`, `nice_nano_pretty`) and for
   the upstream refresh and inventory scripts retire with that code in step 6.
   Findings:
   - The survey of string-typed numeric parameters found only
     `mounting_hole_npth` (`hole_size`, `hole_drill`) and the 3D-model transform
     vectors: every `*_3dmodel_xyz_offset|rotation|scale` parameter that defaulted
     to `''` meant "automatic" and held a three-number list when set. They are
     typed as arrays; an empty list is automatic. The page saves `''` for every
     unset schema entry, so existing documents carry `''` for these, which the
     1→2 migration must drop, and **steps 4 and 5 must ship together** or the Rust
     renderer would reject those documents.
   - The generator bodies are mechanical ports of the JavaScript. Kept as they
     were, to be decided separately: `utility_filled_zone` tests `p.prority`, so
     `priority` is never written; the LED footprint name reads
     `…(per-keysingle-side)`; `utility_router` prints `NaN` for unparsable
     coordinates; `p.drill_y == 0` falls back to `drill` in both mounting holes.
   - Local nets are allocated for every row of the controllers and displays
     whether or not they are reversible, because the JavaScript evaluated those
     templates unconditionally; the goldens pin this, so the Rust bodies do too.
   - Linking all 36 generators into a WASM module (release, `opt-level = "s"`,
     LTO) costs about 690 KB before `wasm-opt` and compression, including
     `serde_json`; measure the page itself at step 4.
   - The crate is now mixed-licence (13 MIT, 23 CC-BY-NC-SA-4.0); see
     `footprints/NOTICE.md`. A test keeps each module's declared licence in step
     with its header. Batches merge unused by production.
4. **Cutover.** Ships together with step 5, because saved documents carry
   values the typed renderer rejects until the migration runs. Switch every page call site to the crate, change
   `footprint_forms.rs` to typed forms, and replace the Prepare/worker/Finish
   paths with single Core requests. Run browser checks for generator edits,
   Parts previews, PCB and KiCad export, fresh-project save and reopen, bundled
   examples and offline behavior.
5. **Format version and rename.** Add a document format version. Documents
   without it are version 1. A 1→2 migration at Core's load boundary renames
   identifiers (including the logo source ID), sets `side` on back-side parts and
   converts saved numeric strings for every number parameter, not only
   `mounting_hole_npth`; it covers browser storage, archive import and bundled
   examples. The native demo builder now emits version 2 documents; recorded version 1
   archives still migrate at load. Migrated projects
   are written back immediately. Rewrite committed catalogue data with a one-off
   script after confirming no recorded hash covers the identifiers, then rewrite
   golden fixtures mechanically: definition IDs, asset IDs, the `ergogen_model_…`
   export paths, and the logo source ID, footprint name and display name. The
   `${EG_INFUSED_KIM_3D_MODELS}` KiCad path variable is not an identifier and is
   unchanged.
6. **Deletion.** Complete after the native builder replaced item 3 of the
   [removal assessment](typescript-node-removal.md): `tooling/demo-projects` and
   `scripts/prepare-demo-projects.mjs` import the JavaScript provider, so nothing
   it imports can be deleted until a Rust builder replaces them. Steps 1–5 do not
   depend on it. Once item 3 replaces bundled-project preparation, delete the
   generator JavaScript, `defaultModels.mjs`, `generate.mjs`, `catalogue.mjs`,
   `ergogen/src`, `kicad/src/ergogen.ts`, the worker, both packaging scripts,
   staged `layout-generators` and `preview-generator` assets, their references in
   build, dev and WASM-test scripts, the transport test, `patches.json` and the
   upstream refresh scripts. Retain vendored models and source and licence
   manifests.

Bundled demos are now prepared by the native Rust builder. Generator-backed
definitions are normalized by Rust before Core opens the projects.

## Deviations

Record each corrected quirk here with the generator, inputs, previous output
and corrected output.

| Generator | Deviation | Previous | Corrected |
| --- | --- | --- | --- |
| `ceoloide/mounting_hole_npth` | `hole_size` and `hole_drill` typed as numbers, not strings | String parameters | Number parameters; saved strings migrated |
| all | Number parameters reject non-numeric values; saved numeric strings migrated | Numeric strings accepted by most generators; `ceoloide/mounting_hole_plated`, `infused-kim/mounting_hole`, `infused-kim/trackpoint_mount` and `ceoloide/switch_gateron_ks27_ks33` throw "Invalid numeric Ergogen geometry" in geometry extraction, and `ceoloide/rotary_encoder_ec11_ec12` throws its hole-size error | Typed error naming the parameter |
| all | `side` declared on every generator, default `F` | `side` read from render inputs, unset values follow the part side; schema default ignored | Declared parameter; migration sets `side` on back-side parts |
| `ceoloide/diode_tht_sod123`, `ceoloide/led_sk6812mini-e`, `ceoloide/switch_choc_v1_v2`, `ceoloide/switch_gateron_ks27_ks33`, `ceoloide/switch_mx`, `infused-kim/trackpoint_mount`, `ceoloide/utility_keepout_zone` | Schema default for `side` | `B` (`F&B` for the keepout zone) | `F` |
| every `*_3dmodel_xyz_offset`, `_rotation`, `_scale` that defaulted to `''` | Typed as arrays; an empty list is automatic; saved `''` removed by migration; a list that is not three numbers is rejected | Text `''` meaning automatic; a malformed list printed `undefined` | Array parameter, typed error naming the parameter |
| `ceoloide/utility_point_debugger`, `infused-kim/point_debugger` | Crosshair label | The text `undefined` (a point name that never existed) | An empty label |
| `ceoloide/utility_router` | Position syntax errors | V8's JSON parser text | A typed rejection naming the route; entries of `routes` must be text |
| `ceoloide/utility_filled_zone`, `ceoloide/utility_keepout_zone` | `points` entries must be `[x, y]` number pairs | Any value printed as text | Typed error naming the parameter |
| unknown generator source | Normalization returns the definition unchanged | Silent pass-through | Typed `UnknownGenerator` error (definitions without a `generator` still pass through) |
| `ceoloide/utility_ergogen_logo` | Renamed `ceoloide/utility_logo`, with footprint and display names | `ceoloide:utility_ergogen_logo` | `ceoloide:utility_logo`; saved source IDs migrated |

## Completion criteria

- [x] All 36 generators render from Rust and match goldens apart from listed deviations.
- [x] No page code imports generator JavaScript; no preview worker exists (the demo builders are gated on item 3).
- [x] Board preview, case preview and KiCad export each use one Core request.
- [x] Version 1 documents from storage, archives and bundled examples migrate to version 2 without loss.
- [x] No persisted identifier or catalogue entry contains `ergogen`; licence and attribution text is exempt.
- [x] Ported modules carry their source licence and attribution.
- [x] Generator JavaScript and its Node packaging, tests and refresh scripts are deleted (gated on item 3).


## Cutover and deletion verification (2026-10-05)

- Core, footprint golden/licence tests, native page tests, WASM page compilation,
  release page compilation, generated contracts, repository checks and tooling
  checks pass. KiCad's independent CLI checks are retained: 23 Node integration
  checks and the native generator export checks cover source artwork, front/back
  placement, connectivity, model paths and legacy arcs.
- The former page/worker/finish chain now uses `PreviewPcb` and `ExportPcb`.
  Targeted headless browser checks cover the catalogue, previews, generator edits,
  handoff orchestration and scope/epoch protection. Four catalogue checks had
  incorrectly used plain `#[test]` inside WASM-only presentation; they now execute
  as browser tests, and the test-reachability check passes.
- Six IndexedDB browser checks pass, including version 1 migration, immediate
  write-back, preservation of stored model bytes and fresh-project save/reopen.
  Missing format versions are legacy; an explicitly null version is malformed
  and rejected, with a regression test. The real REVIUNG41 archive migration
  renders every generated part and opens through Core.
- `cargo run --manifest-path core/Cargo.toml --locked --release --example
  prepare_demo_projects -- web/assets/fixtures` replaces Vite/Node preparation.
  All 20 examples match the recorded Node baseline for placements, references,
  matrix recipes, electrical assignments, mechanical settings, keymaps, modules,
  generator parameter configuration, pad identities and selected model hashes.
  The acceptance test also unpacks every archive and checks embedded bytes.
  Measured source layouts and the review fixture's authored keymap/assumptions
  now live in `content/layouts/`; source hashes remain in preparation provenance.
- Generator JavaScript, the generated catalogue, worker, packaging/transport
  scripts, temporary recorders, generator refresh/tests and the TypeScript demo
  builder are deleted. Vite and the retired workspace packages are removed from
  the lockfile. Vendored models, licences, source manifests and Python model
  maintenance scripts remain. Layout extraction's standalone KiCad form reader
  is retained as `scripts/kicad_forms.py`, without a generator dependency.
- Release packaging excludes obsolete generator assets left by an earlier build;
  a regression test confirms this for both root and subpath sites. This avoids
  returning the deleted worker to the offline inventory through cached outputs.
- A production UI/offline smoke test remains unverified: the browser skill's
  runtime reports no available browser. Native offline-policy tests and release
  packaging tests pass, but do not establish a deployed offline browser reload.
  No broad claim of a Node-free repository is made: CAD and other retained
  maintenance/check tooling remain outside this footprint-generator slice.

- Broader native integration testing (`cargo test --manifest-path web/Cargo.toml`)
  is blocked by existing harness compilation errors in unchanged tests such as
  `outline_camera`, `board_reference_effect` and `outline_lifecycle`: they import
  WASM-only test/browser modules into native fixtures. The documented native
  `--lib --bin boardstudio-web` check passes; those unrelated harness failures
  have not been hidden, deleted or addressed with configuration changes.

- Matched page-size comparison against `9d2656d1f` (before the Core/page cutover),
  with Rust 1.98, the same Cargo release flags (`--no-default-features --features
  page --bin boardstudio-web`) and wasm-bindgen 0.2.129: the bindgen WASM grows
  from 36,293,592 to 36,489,667 bytes (+196,075 bytes, 0.54%). Deterministic gzip
  grows from 7,460,875 to 7,507,383 bytes (+46,508 bytes, 0.62%). This measurement
  is before Dioxus/wasm-opt deployment optimization and excludes the deleted
  JavaScript assets. Artifacts are under `web/target/size-comparison/`.
- The mounted generator editor's two old-side-default assertions were updated
  to the agreed `F` default, including real pad-position assertions. All four
  mounted edit/supersession/provider-failure tests pass; the combined targeted
  page-browser results cover 47 distinct passing tests, plus six storage tests.

- Root and `/boardstudio/` production packaging pass with freshly built page,
  Core worker and native fixtures, reusing the unchanged CAD/renderer providers.
  HTTP checks verify the app, worker entrypoint, bundled archive and offline
  bootstrap with isolation headers on both routes. Each site serves 20 archives;
  each offline inventory lists 189 assets and excludes the retired generators.
  This is static packaging evidence; the UI/offline-reload smoke gate remains.
