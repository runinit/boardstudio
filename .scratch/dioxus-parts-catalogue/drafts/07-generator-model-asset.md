# Draft: Attach and remove a model-backed generator parameter

**Parent:** F4.3 — Generator forms, retained generator preview and Apply (`.scratch/dioxus-frontend-v1/issues/04-parts.md`).

**Source:** The pinned React `GeneratorFields.tsx` recognizes schema fields ending in `3dmodel_filename`, displays the generator parameter as read-only, accepts STEP/STP/STL/WRL files, and offers Remove model through the existing generator draft/Apply path. `createProjectActions.ts` stores imported bytes, creates Asset metadata, writes `boardstudio-asset:<assetId>` into the generator parameter and accepts the document edit.

**What to build:** Surface model-backed filename parameters in the Dioxus 3D model placement group. Reuse `presentation::model_asset_import::read_model_file` and `ImportedModelFile::store`; on import, persist verified bytes and submit one guarded accepted edit that appends Asset metadata and replaces only that generator parameter. Keep removal in the existing generator parameter draft and Apply flow.

**Boundaries:** No generator implementation, model conversion, model pose controls, Runtime/export changes, document schema changes, asset-byte deletion, or F4.4/F7.3 model-render acceptance. Keep file/read/store failures and accepted terminal feedback scoped to the selected Parts generator owner; stale selection or changed generator parameters cannot commit.

**Status:** source-implemented; combined candidate proof pending

- [x] Include supported `3dmodel_filename` parameters in the existing 3D model placement group and show their value read-only.
- [x] Attach or replace using the shared parser/store, checked SHA-256, MIME type, source filename and collision-checked document asset identity.
- [x] Commit the asset and `boardstudio-asset:<assetId>` generator parameter together through one normal accepted document edit after the asynchronous byte store.
- [x] Keep Remove model as a local empty-string generator parameter draft, retaining the existing preview and Apply semantics; leave content-addressed asset bytes/metadata available.
- [x] Retire stale async work and keep owner-specific errors/terminal results scoped to the current selected generator.
- [ ] Verify the changed representative import/replace/remove journey in a later combined browser candidate; no 3D rendering or physical-fit claim is included.
