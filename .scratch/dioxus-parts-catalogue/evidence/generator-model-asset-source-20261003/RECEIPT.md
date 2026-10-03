# Generator model asset source receipt

- Isolated authoring branch: `codex/parts-component-model-editor-20261003`.
- Shared adapter available from `20595f43557a5e591a02ee2deadf78c2ff0be79f`; integrated parent source supplied it as `fcaedd4d`.
- Changed source: `web/src/presentation/parts/generator_settings.rs`.
- React reference: pinned `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/GeneratorFields.tsx` and `app/src/createProjectActions.ts`.

The Dioxus form now includes schema parameters ending in `3dmodel_filename` in its collapsed “3D model placement” group. Their values are read-only. Import and replacement reuse `presentation::model_asset_import::read_model_file` and `ImportedModelFile::store`; after async storage, the handler checks the selected generator owner again, builds from the latest accepted definition, adds collision-checked Asset metadata and updates only the selected generator parameter to `boardstudio-asset:<id>` in one ordinary accepted edit. The model value becomes available to the existing generator preview and accepted definition. Remove model changes only the local parameter draft to empty and uses the existing preview/Apply flow; content-addressed bytes and old asset metadata remain available.

Read, storage, owner-change and terminal errors are scoped to the selected generator; detached or superseded results cannot update a replacement selection. No Runtime, schema, generator implementation or renderer path changed.

`rustfmt --check` and `git diff --check` pass. No tests, build, browser journey or review were run per coordinator instruction. The receipt does not claim F4.3 acceptance, model-render acceptance, or physical-fit readiness.
