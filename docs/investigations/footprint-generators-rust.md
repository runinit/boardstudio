# Footprint generators in Rust

Planned: 2026-10-05. Inspected revision: `3cdeb2ac2`.
Status: agreed plan; implementation has not started.

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
| [defaultModels.mjs](../../ergogen/library/src/defaultModels.mjs), [generate.mjs](../../ergogen/scripts/generate.mjs) | ~150 lines | Bind default 3D models and model-selection rules; build `catalogue.mjs` |
| [ergogen/src/index.ts](../../ergogen/src/index.ts) | 411 lines | Parameter schema, catalogue, render context, form parsing, pad and courtyard geometry, definition normalization, terminal discovery, model IDs and bindings |
| [kicad/src/ergogen.ts](../../kicad/src/ergogen.ts) | 75 lines | Arc upgrade, model-path rewrite, footprint/object split |
| [preview-generator-worker.ts](../../scripts/web/preview-generator-worker.ts) | 196 lines | Envelope validation, net allocation, job execution |
| Two packaging scripts in [scripts/web](../../scripts/web) | ~90 lines | Strip types with Node and stage the modules under `web/assets` |

The page loads the generator module with dynamic `import()` in
[footprint_graphics.rs](../../web/src/presentation/footprint_graphics.rs),
[parts/catalogue.rs](../../web/src/presentation/parts/catalogue.rs) and
[bundled_models.rs](../../web/src/bundled_models.rs). Board preview, case
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
  text, which then passes through the existing parse step. Reuse Core's
  `upgrade_legacy_arcs` instead of porting the TypeScript arc upgrade.
- **Declared metadata.** Each generator declares its kind, display name,
  matrix terminals, keycap-envelope parameters and explicit parameter types
  (number, boolean, string, net, anchor, list, object). Remove the source-ID
  regex classification, the hardcoded switch list and the duplicate display-name
  table in the Parts catalogue.
- **Fixed quirks.** JavaScript coercion and formatting quirks are corrected,
  not reproduced. Each correction is a listed deviation. Rendering rejects values
  that do not match the declared parameter type. No correction may change the
  pad count or order of an existing definition without its own migration, because
  saved pad IDs and nets are matched by index. Generator version `bundled-1` is
  retained.
- **Typed generator errors.** Validation failures name the generator and
  parameter and carry a user-facing message; current message text is the
  baseline wording.
- **Licensing.** Each ported module keeps the SPDX identifier (MIT or
  CC-BY-NC-SA-4.0) and author attribution of its source. The crate documents its
  mixed licensing.
- **Identifier rename.** After cutover, `ergogen:<namespace>/<name>`
  definition IDs become `generator:<namespace>/<name>` and `ergogen:model:<path>`
  asset IDs become `bundled-model:<path>`. Generator source IDs such as
  `ceoloide/switch_mx` and `bundled-1` are unchanged.

## Sequence

1. **Golden baseline.** While Node is available, a temporary harness records,
   for every generator: `parameters`, `catalogue`, render output, geometry,
   normalized definitions, terminals, model bindings, KiCad export forms and
   worker replies for existing fixtures. Inputs are defaults, each boolean
   flipped, sides F and B, rotations 0, 90 and 37 degrees, marker nets, anchors,
   and every generator configuration in the bundled demo projects. Commit the
   fixtures; they cannot be regenerated after deletion.
2. **Provider framework.** Implement form handling, the render context,
   geometry and courtyard joining, normalization, terminal discovery, model IDs
   and bindings, model-path rewriting, net allocation, the generator registry and
   typed errors. Verify against golden render text, independently of generator
   ports.
3. **Generator ports.** Port in batches: utilities and mounting holes; diode,
   LED, reset and power switches; MX, Choc and KS27/KS33 switches; controllers,
   displays, encoder and connectors; infused-kim. Each batch must match goldens
   as parsed form trees with numbers compared as exact text and whitespace
   ignored, apart from listed deviations. Port the model-binding assertions from
   the footprint-library Node tests. Batches merge unused by production.
4. **Cutover.** Switch every page call site to the crate, change
   `footprint_forms.rs` to typed forms, and replace the Prepare/worker/Finish
   paths with single Core requests. Run browser checks for generator edits,
   Parts previews, PCB and KiCad export, fresh-project save and reopen, bundled
   examples and offline behavior.
5. **Format version and rename.** Add a document format version. Documents
   without it are version 1. A 1→2 migration at Core's load boundary renames
   identifiers and converts saved numeric strings for number parameters; it
   covers browser storage, archive import and bundled examples. Migrated projects
   are written back immediately. Rewrite committed catalogue data with a one-off
   script after confirming no recorded hash covers the identifiers, then rewrite
   golden fixtures mechanically.
6. **Deletion.** Once item 3 replaces bundled-project preparation, delete the
   generator JavaScript, `defaultModels.mjs`, `generate.mjs`, `catalogue.mjs`,
   `ergogen/src`, `kicad/src/ergogen.ts`, the worker, both packaging scripts,
   staged `layout-generators` and `preview-generator` assets, their references in
   build, dev and WASM-test scripts, the transport test, `patches.json` and the
   upstream refresh scripts. Retain vendored models and source and licence
   manifests.

Until step 6, bundled demos are still prepared by the JavaScript provider.
Their stored pads reflect the old behavior until renormalized by Rust.

## Deviations

Record each corrected quirk here with the generator, inputs, previous output
and corrected output.

| Generator | Deviation | Previous | Corrected |
| --- | --- | --- | --- |
| `ceoloide/mounting_hole_npth` | `hole_size` and `hole_drill` typed as numbers, not strings | String parameters | Number parameters; saved strings migrated |

## Completion criteria

- [ ] All 36 generators render from Rust and match goldens apart from listed deviations.
- [ ] No page code imports generator JavaScript; no preview worker exists.
- [ ] Board preview, case preview and KiCad export each use one Core request.
- [ ] Version 1 documents from storage, archives and bundled examples migrate to version 2 without loss.
- [ ] No persisted identifier or catalogue entry contains `ergogen`.
- [ ] Ported modules carry their source licence and attribution.
- [ ] Generator JavaScript and its Node packaging, tests and refresh scripts are deleted.
