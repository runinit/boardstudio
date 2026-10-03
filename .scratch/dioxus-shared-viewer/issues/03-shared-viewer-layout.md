# 03: Canonical Layout shared viewer

**Parent:** F7.3 — Single shared assembly viewer and model-preview adapter.

**What to build:** canonical Layout board/model/contour and current-document conditional Case overlays; map picks to its current parts, modules and supported overlays; expose source-backed per-component model visibility and an accessible Layout-specific viewer context.

**Blocked by:** F7.1 is the only canonical start prerequisite. The Case model-delivery owner lands and reviews the common private model bridge first as implementation coordination; this is not a new canonical dependency or change to F7.3 acceptance joins.

**Acceptance joins (not start blockers):** Preserve F7.3's canonical acceptance joins INT.2 and BND.1. F7.8 remains the later cross-workflow join.

**Author-owned boundary:** Own only this consumer's projection, local state/pick mapping, and workflow integration. Root owns page-binary module registration, shared mount/composition and shared CSS; these edits land serially. Reuse the Case child's reviewed private viewer/model bridge. Do not clone the viewer or change Core, renderer exports, schemas, public APIs, or visibility.

**Status:** ready-for-agent; consumer implementation and public verification remain open.

- [ ] Build the current canonical document/selected-board projection for Layout, retaining renderer `PcbModel` transforms and the enabled `BoardReference` pose/elevation. Preserve the existing same-document-only Case overlay condition; never attach overlays from a separate physical Case document.
- [ ] Keep Layout selection and edit ownership in its current workflow. Map picks only through current producer identities and scoped current-document data; unknown/stale picks are empty. Preserve the exact separate module/keycap producer identities where applicable.
- [ ] Preserve source visibility rules including generated-keycap model suppression when generated keycaps exist (where applicable); do not create placeholder meshes or synthetic IDs.
- [ ] In the canonical Layout consumer, derive component layer rows from the captured Layout preview's ordered `PcbModel` rows, not from the physical Case/Native preview slot. Keep each renderer model ID as the visibility key and preserve the source reference and filename label. Join availability only against delivery rows for the same current Layout source owner/lease: delivered rows are available, pending rows are disabled/loading, and missing or failed rows are disabled/unavailable. Toggling a row hides or shows only that model ID through the existing display state and shared renderer; retain the existing global Models visibility control.
- [ ] Give the Layout canvas a Layout/PCB-assembly-specific accessible name and describe the supported current Layout-part pick behavior. Do not announce it as a Case preview. Preserve the existing context name/behavior for other viewer consumers.
- [ ] Demonstrate the existing 2D and 3D routes, real model delivery through the shared adapter, camera/display state, mapped and empty picks, and scope/projection supersession against pinned React. Cover normal, empty/missing/error/retry states relevant to this consumer.
- [ ] Through the production Layout Runtime route, prove an accepted component model reaches the viewer as a decoded mesh and that a corresponding available row hides and restores only that model. Retain model-row/count evidence for the selected fixture; adapter-only stub bytes or a renderer screenshot without its source-owner join do not replace the current readiness and count gates.
- [ ] Verify this consumer through the integrated public Dioxus application after root's serial mount/CSS integration. Retain fixture, paired actions, source/build identity, applicable keyboard/focus/accessibility evidence, and lifecycle cleanup.
- [ ] State explicitly that this child does not close F7.3 or change its INT.2/BND.1 acceptance joins; record RF handoff evidence or “No new refactoring takeaway observed.”
