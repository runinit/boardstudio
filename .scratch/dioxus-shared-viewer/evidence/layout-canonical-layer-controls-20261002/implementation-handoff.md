# F7.3b Layout viewer layer/context and F3.6 placement implementation handoff

**Contract:** existing F7.3b issue 03 and existing F3.6 issue 10, both unchanged in count/readiness/parent joins by this implementation. The planning packet at `a125bbaf3e1cd3363fcdcd1baa04ff9db28466cd` is both-axis clear (Sol report SHA-256 `d7e7b72188ce906bc2eaf233134e6ccad9609eb465b86e3edd089680080cd8d6`).

**Base:** `7d09d0a60fbb2cc12e259541614b9483ee618d29`, pinned React `5a472a9426e6e38993361da402cd4ec730feb369`.

## Bounded source changes

- `web/src/presentation/shared_viewer.rs` selects the component model rows from the active source: live Layout lease uses `LayoutPreviewSnapshot.preview.models`; Case/CAD keeps the current matched physical preview. It joins rows to the already owner-qualified delivery data and reuses existing renderer IDs, model availability, display state, and shared viewer. Stale Layout lease cannot fall back to Case/Native rows.
- The common canvas accessible name is now source-specific. Layout identifies the PCB assembly and current Layout-part picking; Case retains its existing Case wording.
- `web/assets/m1.css` applies React's actual view-group placement: `top:68px; left:12px` by default, top-right when the `.m1-workspace-content` inline-size container reaches 760px, and compact `top:60px; left:8px` at viewport width 900px or less. Existing focus style and 44px compact targets remain.
- `.scratch/dioxus-frontend-v1/refactor-findings.json` and `docs/migration/POST-PORT-REFACTOR.md` append RF-015 for the confirmed source-selection mismatch. This records the minimal source-discriminated fix and keeps a typed viewer-source descriptor as a post-parity question, not a current API expansion.

## Verification and limits

- Candidate browser receipt before the fix: the accepted Layout fixture rendered its board models while expanded Layers contained zero component rows. Paired source, route, screenshot, fixture hash, rendered mesh, pick and view transition evidence are recorded in `planning-evidence.md` and `/home/chris/.local/share/boardstudio/retained-tmp/20261002/layout-toolbar-paired/layout-view-group-public-readiness.md`.
- Existing `case_assembly_layers::tests::layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked` verifies exact-row toggling, disabled missing rows, Escape close and trigger focus restoration. `physical_component_layers` already verifies model order, exact renderer IDs and available/pending states.
- A new browser-hosted Rust regression exercises active-source selection (Layout never falls back to a physical preview; stale Layout returns no rows; non-Layout retains the matched physical source) and context-specific accessible names. The runner was started with `wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- component_rows_use_active_layout_models_without_falling_back_to_case_models`; its result is pending due to concurrent disk-I/O pressure and will be recorded before source freeze.
- `cargo fmt --manifest-path web/Cargo.toml -- --check`, strict page-target all-target WASM Clippy, and the strict page binary check passed before the test annotation adjustment; those checks will be rerun before freeze. The RF register parses as valid JSON and `git diff --check` passes.
- Fresh integrated package, current public per-model visibility hide/restore, paired 1280px rail-open/rail-closed and 375px placement, full Light/Dark, and existing F7.3b/F3.6 parent gates remain open. No real-model readiness, F7.3 closure, or full Layout parity is claimed here.
