# F7.6 mounted Case export readiness tests

Prepared against HEAD `bda42cae76b342147330f971c108711b80549038` as an evidence-only patch. No live Rust files were changed and no test or compilation was run.

## Exact owned files

- `web/src/cad_presentation.rs`: reuse the existing exact mechanical fixture; mount `CasePanel` with readiness from the actual settings owner; add two required focused wasm tests.
- `web/src/runtime.rs`: private `#[cfg(test)]` ReadModel override for transient `display_preview`/`gesture` and lifecycle fields while retaining accepted snapshot/scope and existing event interception.
- `web/src/presentation.rs`: private wasm-test bridge that supplies current selection/workspace contexts and calls `use_mechanical_settings_mount`; no duplicate readiness predicate or visibility widening.

## Fixture and mounted contexts

Reuse `configured_exact_mechanical_runtime()`: opened accepted Session snapshot; board mechanical settings; ready outline; one Plate body and matching stack; exact `CadScene` with same scope/token/revision. The seeded ReadModel must report `Lifecycle::Ready`, `Durability::Saved { revision: accepted.revision }`, active board/instance matching Scope, and exact generation. The regression first asserts Export geometry is enabled through the real owner hook, catching fixture rejection separately.

`export_host` provides Runtime as root context plus the existing `SelectionAdapter` (generation signal), Case `WorkspaceState`, `InstanceSelection`, `CaseSelection`, `ResolvedTheme`, and `CaseGenerationState`. The mounted shared Case viewer consumes those contexts. The owner hook receives a no-op finding callback; it retains ownership of admission.

## Required test behavior

`mounted_failed_current_generation_keeps_previous_geometry_and_retries_owner`: pause live generation, advance accepted token and document/scene revision in the same Scope, set current lifecycle/saved revision and `GenerationStatus::Failed`, retain the old exact CadScene. Assert failure text, mounted CasePanel, old exact scene still retained under its old token/revision while accepted owner is new, Export disabled/no `StartExport`, then explicit Update emits exactly one `StartGeneration` for the same Scope and current token/revision.

The UI's failure title takes precedence over geometry wording, so “Previous geometry” is proven through the mounted panel's retained old exact scene versus advanced accepted owner, not a nonexistent failure-title substring.

`mounted_read_model_preview_and_session_gesture_block_export_until_cancelled`: seed the actual `ReadModel.display_preview` at the private fixture seam and show export disabled/no `StartExport`; clear it and prove readiness restores. Apply `GestureBegin` and `GestureCancel` to an opened real Session, mount each reducer-produced `GestureView` with the exact Case ReadModel and prove the same disable/no-export/restore sequence. It does not claim a Core preview worker or browser pointer-capture path ran; the preview case proves mounted readiness reacts to the real ReadModel field, and the gesture case proves Session reducer output gates that mounted owner.

Both tests use the existing 100 ms Dioxus `settle()` barrier after refreshes. No generation or export worker completion is expected: Runtime's existing test event interception asserts the dispatched `StartGeneration`/absence of `StartExport`. Actual local ZIP/signature and output-comparison evidence is already retained in `RECEIPT.md`; these tests do not duplicate public export qualification.

## Patch state

`F76_REQUIRED_TESTS_READY.patch` is a real unified diff against the three current source hashes recorded in `F76_REQUIRED_TESTS_RECEIPT.md`. `git apply --check` passed read-only. The patch remains unapplied and uncompiled until the coordinator releases the freeze.
