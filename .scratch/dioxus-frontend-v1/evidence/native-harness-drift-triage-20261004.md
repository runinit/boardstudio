# Native integration harness drift triage (2026-10-04)

Read-only source review at `4e716156a069` on `codex/rust-v1-ui-parity-20261001`. No build/test, browser, source edit or exclusion change. Source remains frozen during package work.

## Finding

The three native integration targets still compile actual production component source in their harness crates; their immediate failures are fixture/API drift, separate from RF-022's hand-maintained unit-test `presentation` mirror in `web/src/main.rs`.

- `matrix_field_lifecycle/harness.rs`: `MatrixInspectorProjection` now requires `definition_id`, `switch_choices`, `diode_direction`, `edge_gap_x/y`, `preset`, `orientation`, `baseline_variant`, and `layout_relation`; its fixture only initializes the prior fields. `MatrixInspector` also now requires `on_apply_preset`, `on_delete`, `on_unlink`, and `on_duplicate` handlers. Add representative neutral fixture values and inert handlers; preserve existing five named field checks. The component now also renders Edge gap X/Y inputs, so the fixture's `inputs.len() == 5` mount/no-remount checks are stale (seven total). Update those totals to account for the two new controls, without removing the five original field lifecycle assertions.
- `parts_standard_profile_editor.rs`: `ManualProfileEditorPorts` gained required `request_mechanical_extraction`; fixture port initializer lacks it. Its fixture definition has no `kicad_source`, so a fail-fast/unexpected-call stub can satisfy the new seam without affecting standard-profile assertions. No extra helper-module import is implicated.
- `keycaps_fit_lifecycle.rs`: `KeycapsFitInspector` now requires `mechanical_layer_ids` and `on_navigate`, both supplied by production `keycaps_workspace.rs`. Empty layer IDs and a no-op navigation handler fit this harness's empty document/findings fixture; retain request lifecycle assertions.

`web/src/parts_assembly_preset_draft.rs` is already gated and imported by `web/src/main.rs` for page tests, and depends on `crate::parts_preview`. Its pure conversion unit test belongs to that binary test target. None of these three integration harnesses references the helper or assembly editor; adding another `#[path]` copy there is unnecessary and risks duplicate test/module ownership.

## Scope recommendation

Make the three localized fixture updates when the source freeze lifts. Keep their separate integration targets and native-only `not(target_arch = "wasm32")` guards. This preserves their distinct Dioxus event/lifecycle checks, avoids duplicating wasm coverage, and leaves RF-022's broader shared-module-graph consolidation deferred as recorded. Do not treat a no-bin invocation as whole-repository native verification; retain the binary target that owns the pure helper test and the required wasm/page and headless gates.

## Localized repair and verification

After candidate `4e716156a069` was published and the source freeze lifted, root authorized these four integration fixture files plus the single test-module cfg in `web/src/presentation/keycaps_fit.rs`.

RED reproduced with the requested command:

```text
python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml --features page --test matrix_field_lifecycle --test parts_standard_profile_editor --test keycaps_fit_lifecycle
```

It failed compiling for the stale Matrix projection/Props, missing profile extraction requester, and missing Keycaps props. It also exposed the wasm-only `keycaps_fit.rs` internal test module compiling in the native integration target, followed by the native runtime stub missing `KeycapsPreviewInput`.

Changes are limited to:

- `web/tests/matrix_field_lifecycle/harness.rs`: neutral values for the eight added projection fields, no-op handlers for four unused matrix actions, and expected seven inputs (the five tested fields plus Edge gap X/Y).
- `web/tests/parts_standard_profile_editor.rs`: an unexpected-call extraction requester stub. The fixture definition has no KiCad source, so existing standard-profile journeys cannot invoke it.
- `web/tests/keycaps_fit_lifecycle.rs`: empty mechanical-layer IDs, no-op navigation handler, and matching `KeycapsPreviewInput` in its runtime stub.
- `web/src/presentation/keycaps_fit.rs`: only the inline test-module guard changed to `#[cfg(all(test, target_arch = "wasm32"))]`; all nine wasm test bodies are retained.

Focused GREEN, each through `migration-deliver.py focused-test`:

```text
cargo test --manifest-path web/Cargo.toml --features page --test matrix_field_lifecycle
4 passed, 0 failed
cargo test --manifest-path web/Cargo.toml --features page --test parts_standard_profile_editor
18 passed, 0 failed
cargo test --manifest-path web/Cargo.toml --features page --test keycaps_fit_lifecycle
4 passed, 0 failed
```

Total: 26 native tests executed and passed. `cargo fmt --manifest-path web/Cargo.toml -- --check` and `git diff --check` for the changed files pass. No other source/test files, assertions, or exclusions were changed. Coordinator still owns the mandatory changed-module wasm gate and review.
