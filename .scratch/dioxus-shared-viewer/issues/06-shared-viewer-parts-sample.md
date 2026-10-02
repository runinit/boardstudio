# 06: Isolated Parts sample shared viewer

**Parent:** F7.3 — Single shared assembly viewer and model-preview adapter.

**What to build:** isolated `sampleAssembly` document, never the live project document; advance private projection generation when definition, companion set/order, placements, side, or rotation changes even when sample IDs or copied revision are reused

**Blocked by:** F7.1 is the only canonical start prerequisite. The Case model-delivery owner lands and reviews the common private model bridge first as implementation coordination; this is not a new canonical dependency or change to F7.3 acceptance joins.

**Acceptance joins (not start blockers):** Preserve F7.3's canonical acceptance joins INT.2 and BND.1. F7.8 remains the later cross-workflow join.

**Author-owned boundary:** Own only this consumer's projection, local state/pick mapping, and workflow integration. Root owns page-binary module registration, shared mount/composition and shared CSS; these edits land serially. Reuse the Case child's reviewed private viewer/model bridge. Do not clone the viewer or change Core, renderer exports, schemas, public APIs, or visibility.

**Status:** ready-for-agent; consumer implementation and public verification remain open.

- [ ] Build the isolated `sampleAssembly` document and sample-board projection for Parts, retaining renderer `PcbModel` transforms and any enabled `BoardReference` pose/elevation associated with that sample. Do not attach unrelated live-project Case overlays.
- [ ] Keep Parts selection and edit ownership in its current workflow. Keep picks sample-local; never map sample IDs to or edit live-project parts. Unknown/stale picks are empty. Preserve the exact separate module/keycap producer identities where applicable.
- [ ] Preserve source visibility rules including generated-keycap model suppression when generated keycaps exist (where applicable); do not create placeholder meshes or synthetic IDs.
- [ ] Demonstrate the existing 2D and 3D routes, real model delivery through the shared adapter, camera/display state, mapped and empty picks, and scope/projection supersession against pinned React. Cover normal, empty/missing/error/retry states relevant to this consumer.
- [ ] Verify this consumer through the integrated public Dioxus application after root's serial mount/CSS integration. Retain fixture, paired actions, source/build identity, applicable keyboard/focus/accessibility evidence, and lifecycle cleanup.
- [ ] State explicitly that this child does not close F7.3 or change its INT.2/BND.1 acceptance joins; record RF handoff evidence or “No new refactoring takeaway observed.”
