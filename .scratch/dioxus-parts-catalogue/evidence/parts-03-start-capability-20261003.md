# Issue 03 selected-definition preview start capability

**Inspected candidate:** `d7742d46ad0e4dc05d5eb229c7c7de50e6f7945c`, served
at `http://127.0.0.1:34757/`.

## Callable inputs already present

- `PartsPreviewWorkspace` resolves the current accepted catalogue entry from
  the Parts selection, and mounts `PartsMechanicalProfileWorkspace` in the
  central Parts canvas. The accepted selected `PartDefinition`, accepted
  `Scope`, snapshot token, generator draft, and per-selection generation reach
  the preview consumer. The parts-private 2D projection already uses the
  accepted source definition and the existing footprint projector.
- The page binary has a private `RendererPageHost` mount and a single
  `shared_viewer::CaseSharedViewer` consumer used by Case, Layout, Keymap, and
  Keycaps. The host includes scene submission, camera and display controls,
  lifecycle status, scoped event delivery, unmount cleanup, and existing model
  asset decode ports.
- The F7.1 private contract decision is recorded in
  `.scratch/dioxus-shared-viewer/issues/01-common-viewer-contract.md` and the
  reviewed dispatch evidence under
  `.scratch/dioxus-shared-viewer/evidence/source-contract-reviewed-20261002/`.
  This is a consumer start proof only; it does not assert completion of F7.1 or
  F7.3.

## Start boundary

The standalone selected-definition route can build an ephemeral sample
`ProjectDoc` from the exact accepted definition, copy the source project's
asset descriptors, clear every sample terminal net, and run the existing
Core → preview-generator worker → Core pipeline. A Parts owner must retain the
active accepted scope/token/document identity plus the selected definition and
selection generation; the sample's fixed IDs and copied revision are not
currentness keys. The Parts consumer must reject work when the source owner
changes or the preview unmounts and must never map sample picks into live parts.

The current `CaseSharedViewer` and `native_case_preview` are correctly scoped
to the active Case document. They cannot be passed off as a library sample.
This implementation therefore owns a Parts-private sample producer and a
narrow private Parts source variant in the existing shared viewer/model path.
No renderer copy, shared WASM/service-worker input, Rust engine rewrite, public
API, schema, or active-project edit authority is needed. F7.3, INT.2, full
F4.1/F4.4, assembly/companion parity, and their acceptance joins remain open.

This is the capability-level operational start recorded on Issue 03. It does
not change F4.4's canonical graph row, blocker history, full acceptance
criteria, or any of the 62 frontend parent records.
