# 01: Prove the private shared viewer contract and scope inputs

**Parent:** F7.1 — Common viewer contract and renderer feasibility.

**What to build:** Establish one source-backed private contract for the shared 3D viewer so Layout, Parts, Keymap, Keycaps and Case can use the same scene, camera and picking behavior. The handoff identifies callable renderer operations, required scene/state inputs, lifecycle/error handling and the distinction between canonical-board scenes and Case's selected physical-instance scene. It chooses a reachable private wrapper placement in the page binary or an already sufficient public facade, with no API or visibility expansion.

**Blocked by:** None (F7.1 has no canonical start blockers).

**Status:** ready-for-agent

- [ ] Compare the pinned React viewer behavior with the current Dioxus renderer host and the existing Rust WASM renderer exports for scene replacement, display state, camera presets/orbit/zoom/fit, handles, picking, model decoding, lifecycle, context loss and error recovery. Record each supported operation and any adapter-only gap; identify any true contract gap precisely.
- [ ] Demonstrate the actual crate call path. The page binary and `boardstudio_web` library are separate crates, so `pub(crate)` library methods are unreachable from the binary. Record the selected same-crate private wrapper location or cite the sufficient existing public call path; do not widen visibility.
- [ ] Define the minimum private input/output contract for all five consumers, including canonical-board inputs for Layout/Keymap/Keycaps and the selected physical-instance projection for Case. Keep CAD generation, workspace feature behavior and cloned viewer implementations out of this slice.
- [ ] Publish a concrete decision and acceptance evidence for F7.3 and consumer integration. Reuse existing sufficient evidence and identify exactly what must be exercised by those later tasks; do not claim the shared viewer itself is implemented.
- [ ] Record “No new refactoring takeaway observed” for the inspected renderer/host boundary, or add source-backed evidence to existing RF-002/RF-012 without proposing an unapproved API change.

**Parent graph:** Canonical F7.1 `Start after: none`; `Acceptance joins: none`. F7.3 starts after F7.1. Its canonical `Acceptance joins` are INT.2 and BND.1; they do not block fixture-backed F7.3 implementation. All other F7 joins remain unchanged.

**Suggested routing:** Luna High author and verifier; dedicated Astra review because the task establishes a renderer boundary used by several workflows.
