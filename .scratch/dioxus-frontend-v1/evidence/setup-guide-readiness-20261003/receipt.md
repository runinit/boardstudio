# F2.4 setup-guide readiness marker receipt

**Scope:** Existing F2.4 setup-guide criterion 28, implemented on private candidate base `047a33d8`.

**Pinned oracle:** React source `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/SetupGuide.tsx`, `app/src/ui/setupGuide.ts`, `app/src/ui/setup-guide.css`, and the five stage bodies in `app/src/ui/Workbench.tsx`.

**Source RED:** React explains “Step n of 5. Move between steps freely; your work is kept.” and marks each `stages[stage].ready` entry with an accessible “Ready” check. The Dioxus `ProjectSetupGuide` already had all five clickable stages and their current-stage ARIA state, but every marker was a number and the guide omitted that navigation explanation.

**Change:** The existing guide now renders the step explanation, an accessible check marker for each ready stage, and dynamic status detail. The existing ProjectDoc, accepted SceneDelta and current board wiring plan provide the readiness inputs; the guide owns only its display projection. Project, layout, wiring, optional case and review gates follow the pinned React stage rules, including board-scoped layout findings and the same generated-net/configuration comparison used by React’s `isWiringApplied`.

**Changed files:** `web/src/presentation/setup_guide.rs`, the narrow registration block in `web/src/presentation.rs`, scoped rules in `web/assets/m1.css`, fixture constructor updates in `web/src/presentation/setup_guide/tests.rs` and `web/src/presentation/objects/matrix_setup_focus_tests.rs`, and the existing criterion text in `.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`.

**Architecture:** No new RF finding. The readiness display remains a projection of accepted Session/Core data; it adds no second document authority or guide state owner.

**Checks and limits:** Ran Rust formatting for the guide module and its existing guide test fixture, plus `git diff --check`. No Cargo tests/build, browser journey, broad matrix or independent review was run, per the 2026-10-03 integrate-more/test-later direction. Root owns the combined candidate check and changed paired journey.

## F2.4-C01/C02 paired saved-fixture journey

**Reference:** TS app `http://127.0.0.1:5175/`; named sessions `f24guide-ts-20261004` (1280×577) and `f24guide-dioxus-20261004` (1280×577). **Candidate:** frozen Dioxus app `http://127.0.0.1:34804/`. Each side used Start Sofle v2, then exercised the same saved Sofle project. Browser sessions were closed after the paired checks.

Both implementations exposed five freely selectable stages in the order Project & hardware, Layout & assemblies, Controller & wiring, Case (optional), Review & export. The same Sofle fixture marked stages 1, 2, 3, and 5 Ready, while the optional Case stage retained its numbered incomplete marker. Accessibility snapshots exposed the Ready label on ready stages and the numbered label on the incomplete stage. Every stage displayed its matching detail; the visible “Step n of 5. Move between steps freely; your work is kept.” explanation remained present. Previous from Review returned to Case. Back to objects dismissed the guide without changing the saved current stage, and reopening Setup guide restored that stage.

For the per-project preference check, each side left the Sofle guide open on Case, created a new project, and then reopened the saved Sofle project from the project menu. TS stored Sofle `1df91167-b0e4-45bb-93ee-16f0a0c53def` as `open=true,currentStep=case`; the new TS project `48593832-c976-4d8d-b350-cfa2b9587a6c` had its own `open=true,currentStep=project` state. Reopening Sofle restored its open Case guide. Dioxus likewise stored Sofle `ed599cf0-6616-4b28-945a-70a0fb8204e3` as `open=true,currentStep=case`; the new project `9688fbe0-3d1a-4889-a4b3-f9bc99d70a73` had independent `open=true,currentStep=project` state. Reopening Sofle restored its open Case guide. Back to objects had persisted `open=false,currentStep=case` before the guide was reopened on each saved project.

**F2.4-C01 verdict: PASS for the paired journey.** Five stages, readiness marker/accessible name distinction, stage details, Previous, and Back to objects were exercised on both. **F2.4-C02 verdict: PASS for the paired journey.** Open and current stage remained scoped to each project across new-project creation and reopening the saved Sofle project. No source or tracker changes were made for this qualification.
