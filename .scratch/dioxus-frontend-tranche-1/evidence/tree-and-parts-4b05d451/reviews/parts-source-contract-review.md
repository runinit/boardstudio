# Parts catalogue source-loading contract review

Source/design review, 2026-10-02. No application source mutation, builds, typed execution or browser acceptance performed. Read actual packaging, footprint_graphics precedent, Rust PartDefinition, imported catalogue, React catalogue composition, build/offline policy and component-onboarding requirements.

**Clear the bounded private loader design**, with the exact source and remaining proof obligations below. No wrapper export, public facade, dependency/configuration or provider change is needed.

## Exact module and deployment contract

Import `crate::runtime::resource_url("assets/layout-generators/src/index.js")`. `scripts/web/build-layout-generators.mjs:15–21` strips TypeScript while retaining the named `catalogue()` export from `ergogen/src/index.ts:34`. Its relative `../generated/catalogue.mjs` import resolves to `assets/layout-generators/generated/catalogue.mjs`. That generated module contains generator implementations, not ready PartDefinitions; do not import it as the catalogue API. The top-level `assets/layout-generators.js` wrapper exports only isErgogen/render.

Runtime resource_url yields origin + `/assets/...` at root and origin + `/boardstudio/assets/...` for the existing subpath. The current frontend helper packages both files before enumerating offline assets. Completed `web/target/builds/frontend-panels-a3a0224f-20261002/offline-manifest-{root,subpath}.json` includes both exact paths. service_worker.rs installs scope-prefixed assets and serves cached module requests. This establishes packaging/path compatibility; actual first Parts open after an offline reload at both prefixes remains required. No arbitrary deployment-prefix support is implied.

## Typed imported and generated data

The static JSON has formatVersion 1 and nine unique entries wrapped as `parts[].definition`. All observed definition field names and kinds map to current `core/src/model.rs:141–246`, including optional/null profiles and embedded KiCad source. Use private wrapper deserialization and the existing PartDefinition, preserving complete nested fields; do not fabricate defaults, strip rejected fields, or silently drop records. serde-wasm-bindgen 0.6.5 supports plain-object struct/map inputs and null/undefined optional values (`src/de.rs:495,552,567`), compatible with the catalogue's returned objects.

Full compatibility is not proven by this source inspection: require typed decode of all nine static definitions and the actual catalogue() result through the browser serde-wasm-bindgen path. Exercise/report import, callable, generation and decoding failures. Existing core tests decode selected imported definitions but do not establish this whole loader path. Cache immutable bundled results across document repaints; guard async lifetime and merge against fresh current definitions.

## Merge and provenance

Preserve JavaScript Map semantics: Ergogen then imported then project; later duplicate IDs replace values without moving their original position. New project-only IDs append. A sorted Rust map would change order.

This raw merge is source-loading clearance only. React Workbench.tsx:285–290 applies constructionDefinition to bundled entries before project overrides; projectConstruction.ts:9–13 adjusts reversible parameters and the Gateron solder-only variant. Preserve that behavior through its approved authority or keep it an explicit parity join; raw merging alone is insufficient.

Static JSON SHA-256: `000f4ba13114305c33e1378806c25840d903fa335da559b88c9d9404731acd7f`. Contrary to the suspected gap, completed a3a0224f and 2030c9a3 frontend provenance already records this exact path/hash via inherited source inventory. Root should verify retention for the new candidate; no duplicate configuration change is currently justified. Record the new private caller of the retained generator boundary in root-owned architecture/evidence documentation.

Existing RF-001/002 boundary observations remain applicable; no new structural takeaway observed. This approves implementation of the private loader, not Parts feature acceptance, geometry qualification or public API changes.
