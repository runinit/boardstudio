# F2.4 setup-guide readiness marker receipt

**Scope:** Existing F2.4 setup-guide criterion 28, implemented on private candidate base `047a33d8`.

**Pinned oracle:** React source `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/SetupGuide.tsx`, `app/src/ui/setupGuide.ts`, `app/src/ui/setup-guide.css`, and the five stage bodies in `app/src/ui/Workbench.tsx`.

**Source RED:** React explains “Step n of 5. Move between steps freely; your work is kept.” and marks each `stages[stage].ready` entry with an accessible “Ready” check. The Dioxus `ProjectSetupGuide` already had all five clickable stages and their current-stage ARIA state, but every marker was a number and the guide omitted that navigation explanation.

**Change:** The existing guide now renders the step explanation, an accessible check marker for each ready stage, and dynamic status detail. The existing ProjectDoc, accepted SceneDelta and current board wiring plan provide the readiness inputs; the guide owns only its display projection. Project, layout, wiring, optional case and review gates follow the pinned React stage rules, including board-scoped layout findings and the same generated-net/configuration comparison used by React’s `isWiringApplied`.

**Changed files:** `web/src/presentation/setup_guide.rs`, the narrow registration block in `web/src/presentation.rs`, scoped rules in `web/assets/m1.css`, fixture constructor updates in `web/src/presentation/setup_guide/tests.rs` and `web/src/presentation/objects/matrix_setup_focus_tests.rs`, and the existing criterion text in `.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`.

**Architecture:** No new RF finding. The readiness display remains a projection of accepted Session/Core data; it adds no second document authority or guide state owner.

**Checks and limits:** Ran Rust formatting for the guide module and its existing guide test fixture, plus `git diff --check`. No Cargo tests/build, browser journey, broad matrix or independent review was run, per the 2026-10-03 integrate-more/test-later direction. Root owns the combined candidate check and changed paired journey.
