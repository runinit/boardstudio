# Component model editor source receipt

- Base: `f653b8f9f53c306509863d1464ac94fd58aa8b8b` (the isolated branch also contains its shared-adapter commit and the module-profile compiler repair).
- Shared adapter commit: `20595f43557a5e591a02ee2deadf78c2ff0be79f`.
- Component UI source: pending final source commit.
- Reference: pinned React source `5a472a9426e6e38993361da402cd4ec730feb369`, `PartsInspectorPanel.tsx` and `createProjectActions.ts`.

The shared private adapter `presentation::model_asset_import::read_model_file(file).await` accepts STEP/STP/STL/WRL browser files, rejects empty and over-32-MiB inputs, computes SHA-256 and returns the original filename and model MIME type. `ImportedModelFile::store(&runtime.store).await` persists verified bytes through the existing `BrowserStore`. The adapter deliberately leaves document asset identity and metadata to its caller.

The selected non-generator component Inspector now has a local file-import flow, first-model-binding alignment controls for offset/rotation/scale, and detach. Asset bytes are stored before an accepted document edit. After asynchronous persistence the edit is reconstructed from the latest accepted snapshot, retains later model bindings and unrelated current definition fields, and replaces only the first binding as the React action does. A detach leaves content-addressed bytes and asset metadata intact. Read, persistence, owner, cancellation and accepted-terminal feedback are scoped to the current Parts selection/session/scope.

No tests, build, browser journey or review were run for this source packet by instruction; the root coordinator will join source and run the combined candidate checks. The existing viewer/model-projection and physical-fit evidence remains separate; this receipt does not claim model rendering acceptance.
