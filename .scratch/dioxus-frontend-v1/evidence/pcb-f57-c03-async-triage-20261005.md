# F5.7-C03 routed-reference async qualification triage

Read-only source/evidence inspection at current worktree HEAD `cc728fc6`.

## Existing coverage and limits

- The paired public receipt `pcb-routed-folder-20261005/RECEIPT.md` covers valid import, replacement, removal, six-file attachment, SHA-256 persistence and reload. Do not replay those paths for this gap.
- Existing malformed-board public journey plus `f17ae37e` and `board_reference_effect::current_failure_keeps_specific_reason_and_stale_failure_is_retryable` cover current invalid-file messaging, retry usability, and only the pure current/stale error-string projection.
- `web/tests/board_reference_effect.rs` mounts `clear_missing_reference`, which checks absent-reference reset/epoch invalidation. It does not mount `pcb_board_reference::Editor` or run discovery/import/attachment handlers.
- No test in `pcb_board_reference.rs` currently drives `discover_stored_paths`, `import_routed_board`, `attach_model_files`, or their `begin_*` handlers. Owner admission and post-await guards in `presentation.rs` have no routed-reference mounted composition.

## Smallest useful qualification

1. Mount the production `Editor` with a Ready/Saved accepted Layout owner, a real `BoardReference` and matching `Asset` metadata, but a unique SHA-256 whose bytes are absent from `BrowserStore`. Let the real automatic discovery effect reach `load_asset`; assert the specific missing-browser-storage alert, enabled “Retry model-path discovery”, no project edit, and that the reference remains intact. This tests the missing-blob branch, not malformed KiCad or missing project metadata.
2. Drive the actual Replace file input with the retained supported `.scratch/dioxus-frontend-v1/evidence/pcb-routed-folder-20261005/Replacement_PCB.kicad_pcb`. Hold a test-only scheduling gate immediately after the real `Runtime::preview_routed_board_source(...).await` reply and before its existing owner recheck. While held, change current accepted project ID, active board/scope, or accepted revision/token (one identity dimension per rerun, same fixture/handler—not a cross-product), then release. Assert retryable stale-owner alert, no saved board asset/project Edit, and old reference unchanged in the changed owner's document. This executes file read + real Core worker preview; the added gate controls scheduling only and does not emulate the worker or prove cancellation of an in-flight worker request.

## Fixture/context requirements

In one `#[cfg(all(test,target_arch="wasm32"))]` mounted test module under `pcb_board_reference.rs`: create `Runtime::new()` and unique DOM root; prepare an accepted snapshot/scene from `firmware_export_test_support::opened_session`, adding one board reference and asset record; provide a `ReadModel` with that accepted token/revision, `Lifecycle::Ready`, `Durability::Saved { revision }`, matching active board/instance, no preview/gesture; use `workspace="Layout"`, `SelectionAdapter` with generation 1, and `LayoutOwnerIdentity` captured from that exact snapshot/scope. Use a unique never-saved SHA to avoid IndexedDB cross-test contamination. For stale cases mutate snapshot identity/scope and install the corresponding read model before releasing the gate. Await an explicit gate-entered signal and then gate release; assert status/error and event/asset-store counts before DOM cleanup. Existing Core worker deployment and IndexedDB are real browser dependencies.

## Lease recommendation

The only required source lease is `web/src/presentation/pcb_board_reference.rs`: private test-only gate after the real preview await, mounted fixture/host, and tests. Its descendant test module can use ancestor-private `LayoutOwnerIdentity` and `SelectionAdapter`; existing crate-private Runtime test setup methods suffice, so no `runtime.rs` or `presentation.rs` bridge is indicated. `board_reference_effect.rs` needs no change; its pure projection already has focused coverage. If project/scope fixture mutation cannot be expressed with existing Runtime test setters, stop and request the exact smallest helper in `runtime.rs` rather than widening production APIs.

## Bounded implementation and harness result

Root authorized a one-file test lease; the only live change is `web/src/presentation/pcb_board_reference.rs` (SHA-256 `814448b45fbec2c6ea70d77b5c901e61b7d99ce3f86a1a769d311964a9040ae6`). It adds a cfg(test) gate after real `File.array_buffer()` completion and before the existing owner check, a mounted Editor missing-blob test, and a mounted host driving the same `begin_board_import` function for independent project, board, and revision changes. The supported replacement fixture gains only trailing newlines in each stale case to ensure unique storage hashes; the owner guard runs before Core preview, so bytes are not parsed by Core.

Focused command, run through `scripts/migration-deliver.py focused-test`: `wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- mounted_async_tests --nocapture`. First compilation found two generated-props type-name errors in this file; those were corrected. The second compilation failed elsewhere before any test executed: `web/src/presentation/case_viewer.rs:1926–1927` passes `Rc<&CadScene>` where `Rc<CadScene>` is required. Treat both attempts as harness/compile failures, not product RED. Source is now frozen while Astra resolves that shared target error; no retry is authorized until its terminal handshake. `rustfmt` and `git diff --check` passed.

## Final focused qualification result

After Astra released the compiler slot, the focused module was run through `scripts/migration-deliver.py focused-test`:

`wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- mounted_async_tests --nocapture`

Final result: **2 passed, 0 failed, 324 filtered, 51.49 s**. The first runnable attempt also compiled and passed the three stale-owner variants but failed the missing-blob test's selector because it selected the earlier “Remove PCB reference” button; the test was corrected to find the exact retry label, click it, and verify the missing-blob alert returns. Final success includes that retry cycle. Final leased-source SHA-256 is `1b467f5e7309b777d717d18960b7114ea7d7b4e6099ba9fbe066e65440585a34`; `rustfmt` and `git diff --check` pass.

Boundary remains deliberate: the owner-change cases exercise `begin_board_import` after a real browser `File.array_buffer()` await, paused before the existing owner guard. They establish no worker dispatch, project edit, or asset save after project/board/revision ownership changes. Runtime's CoreWorker is unavailable in this headless fixture, so post-Core-reply stale admission, worker cancellation, and parsed-replacement behavior are not claimed. The missing-blob case does mount production `Editor`, uses real BrowserStore/IndexedDB lookup, clicks its actual Retry control, and confirms the same failure remains recoverable.
