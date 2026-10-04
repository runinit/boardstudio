# Case-local Export geometry source investigation

Candidate `frontend-parts-library-repairs-20261004` is source `be570f3b` at `34822`. The fresh Sofle fixture reaches “Exact case geometry ready” after adding an authored Plate and configuring the mechanical stack, but CasePanel has no local export action. The pinned reference mounts `Export geometry` after mechanical-stack configuration; its edited fixture correctly disables the button while findings remain. `app/src/ui/MechanicalAssemblyPanel.tsx` passes `onExport` and readiness into `CaseGenerationControls` (lines 546/568); the latter renders the named button and disables it for unready/busy geometry.

## Existing route and ownership

`web/src/runtime.rs::Runtime::export_mechanical` is the existing generated-mechanical export route. It admits only the current physical scope, accepted board/revision, captured mechanical configuration, matching board, ready outline, exact current generation and exact same-scope/token CAD scene with current assembly revision and no blocking/error findings. It captures scope, token, revision, session epoch, document and executor identities; asynchronous delivery rechecks them. `web/src/presentation/export_workspace.rs` already routes its Generated mechanical package row to this same method. Do not add a provider, API, or second export route. The minimal originating UI belongs in `web/src/cad_presentation.rs::CasePanel`; capture rendered scope/token/session and recheck them plus `InstanceSelection::is_current` at click before calling `export_mechanical`.

## Regression constraint and mounted plan

No meaningful native RED exists for this omission: `CasePanel` is in the wasm-only presentation tree and is absent from native compilation. `web/src/cad_presentation.rs` puts its mounted test module behind `cfg(all(test, target_arch = "wasm32"))`. A helper-only native test would manufacture an abstraction that cannot fail because a DOM control is absent. Existing wasm mounted fixtures have `set_definition_name_test_state`, `set_definition_name_test_generation`, and `set_cad_scene_test` seams; their current document has no mechanical configuration, so it is not already a ready exact-stack fixture.

Concrete mounted RED: on published candidate 34822, select the fresh Sofle, add authored Plate, configure mechanical stack, wait for exact-ready status, and assert a button named “Export geometry” is present and enabled; it is currently absent. Assert it is disabled during wall 2.1 generation/Previous geometry. After configuring Right, capture its click callback, switch to Left, then invoke the stale callback and verify no export is routed for Right. GREEN should route one current Left action through the existing method. The coordinator owns paired browser verification; no build/browser was run here.

## Owned files

Production ownership requested: `web/src/cad_presentation.rs` only. `web/src/case_generation_lifecycle.rs` or other native helper/test files are unnecessary for the UI omission. This investigation changed no production source.
