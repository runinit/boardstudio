# Parts sample per-model layers repair — 2026-10-04

## Reproduced gap

In the published `frontend-layout-grouped-repairs-20261004` candidate (`f320e6b8`, port `34821`), the Parts 3D sample exposed only the aggregate Models layer. The paired React reference exposed two independently toggleable rows, `P1 · 47.stp` and `P1 · 43.stp`. `CaseSharedViewer` admitted Parts delivery rows for renderer meshes, but selected component-layer rows only from Layout or the physical Case preview, leaving Parts' `PcbPreview.models` out of Layers.

## Repair and scope

`case_display.rs` now owns the private source-selection policy for component models. Layout selects only its admitted Layout models; physical Case previews select only their matched preview models; Parts selects only its leased sample models. An active source with absent or expired model rows returns no component rows and cannot borrow another source's rows. `shared_viewer.rs` supplies the Parts sample's current `preview.models` to that policy, guarded by the Parts owner lease. Existing Layout and physical-preview selection behavior is preserved.

The native policy test covers ordered Parts sample IDs and absent Parts rows with physical rows present (the mounted source maps an expired lease to absence), plus the existing Layout and physical source choices. The existing mounted layer-menu test is now strengthened with two delivered models sharing reference SW1 but having distinct filenames and renderer IDs, plus the retained unavailable D1 model. It asserts independent hide/show, exact hidden model ID, sibling preservation and restoration, while preserving appearance selection, unavailable-row and Escape/focus coverage. It now removes its root and stylesheet. The source-policy native test and actual paired candidate journey supply Parts-specific admission and mounting evidence; the generic menu test alone does not establish a real Parts runtime join or live-project immutability.

## RED/GREEN

Command:

```sh
python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml --bin boardstudio-web parts_component_layers_select_only_current_sample_models
```

- RED before the source-policy repair: one test ran and failed at the expected `Option::unwrap()` because Parts selection returned `None` for its sample models.
- GREEN after the repair: `test case_display::tests::parts_component_layers_select_only_current_sample_models ... ok`; result `1 passed, 0 failed`.

`git diff --check` passed. No WASM run or candidate build was performed by this author; the currently served candidate predates the source changes.

## Coordinator test-compile follow-up

The first actual wasm listing compiled the new mounted test and rejected its nested quoted RSX interpolation at case_assembly_layers.rs463. The coordinator changed it to a normal expression child for `display().hidden.join(",")`. This was a test-only compile repair; no assertion was weakened. The page compiler had already passed. The corrected mounted suite result follows below when available.

The next wasm compile also caught the new fixture signal missing `mut` for its display callback. The binding was corrected; product code and expected assertions were unchanged. Both diagnostics were test compile failures, not passing or skipped tests.

## Test consolidation

At the user’s request, removed the helper-only wasm `component_rows_use_active_layout_models_without_falling_back_to_case_models`. Independent review confirms that the executed native policy regression subsumes its Layout-current, Layout-empty/no-fallback, Physical-current and Parts-current assertions, and additionally covers absent Parts rows with physical rows present. The mounted per-model Hide/Show test remains because it exercises different UI behavior. No failure is hidden by this deletion: the removed test passed in the first raw shared-viewer run.

## Final Astra-approved consolidation

The separately added `parts_sample_layer_composition` fixture and `parts_sample_layers_toggle_exact_renderer_ids_in_sample_local_display` test were folded into the existing `layer_menu_composition` and `layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked`. Their unique assertions were retained; the module returns to six tests. The separate fixture's earlier compile and cleanup failures above are historical diagnostic evidence. The final strengthened fixture awaits the mandatory combined commit gate.
