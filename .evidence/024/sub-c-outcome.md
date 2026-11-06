# 024 sub-lane C — run 4 outcome

Initial HEAD `5e0abdce8` on `lane/024c-rest-migrate` was clean; run-1 base is `64763a3`. Four grouped commits migrate PCB wiring/inspectors, PCB/keycap/keymap SVG scenes, Parts/controller previews and shared docks, and the Library/project browser to existing Theme tokens and utilities. No CSS or ticket was edited, no API was widened, no test-name baseline was changed, and no push or main-checkout operation was made. Run 4 does not add per-class utility assertions or run mutation checks. The controller-preview ancestor styling now uses the carousel's existing accessible attribute rather than the legacy dotted class.

**Remaining: one actual legacy class and one dotted functional query**, both for the same `m1-project-menu` dependency. The Library test host retains the class because `ui-shared::project_menu::close_project_menu` queries `details.m1-project-menu` to close and restore focus. Changing that production selector is outside this lane's ui-shared class-only brief and needs coordinated semantic-selector migration; deleting only the class would silently break the close/focus path. The former `.m1-inspector-body` class was removed after the parent changed the Layout focus fallback to `#m1-inspector-panel-content > div`. `web/src/presentation/layout_layers_tests.rs` has a now-vacuous `.m1-layers` absence query; its other legacy finding selectors are the known paired sibling-A page-test blocker described below. These are not evidence that arbitrary legacy identifiers are styling classes.

`web/assets/imported-modules.json` is an ignored build copy of the tracked `catalogue/modules/imported-modules.json`, not a UI snapshot. `scripts/build-web.py` copies it; `parts/modules_catalogue.rs` deserializes ModuleDefinition records. Its `board-outline` and `board-outline-missing` strings are diagnostic codes, not runtime CSS classes; the data is unchanged. Other unchanged counts below are board/document/operation IDs, input/list IDs, transaction IDs, cache keys, or prefix checks; they must not be renamed as styling.

## Per-file before/after vs `64763a3`

Distinct raw quoted-start or dotted `m1-` / `board-` names per file; **not class counts**: includes IDs, diagnostic codes, test-only prefix strings, and can miss a class later in a string. Includes all 35 named files plus earlier run changes. Asset base uses the tracked source copied to the ignored asset.
| File | Base raw | Current raw |
|---|---:|---:|
| `web/assets/imported-modules.json` | 2 | 2 |
| `web/crates/host/src/cad_jobs.rs` | 2 | 2 |
| `web/crates/host/src/cad_worker.rs` | 2 | 2 |
| `web/crates/keycaps/src/keycaps_finding_marker_tests.rs` | 2 | 2 |
| `web/crates/keycaps/src/keycaps_scene.rs` | 4 | 0 |
| `web/crates/keymap/src/keymap/binding_editor.rs` | 2 | 0 |
| `web/crates/keymap/src/keymap/canvas.rs` | 6 | 0 |
| `web/crates/keymap/src/keymap/encoder_editor.rs` | 4 | 0 |
| `web/crates/keymap/src/keymap/macro_editor.rs` | 3 | 0 |
| `web/crates/keymap/src/keymap/panel.rs` | 17 | 0 |
| `web/crates/library/src/library.rs` | 63 | 5 |
| `web/crates/parts/src/parts.rs` | 13 | 0 |
| `web/crates/parts/src/parts/details.rs` | 3 | 0 |
| `web/crates/parts/src/parts/modules_catalogue.rs` | 6 | 0 |
| `web/crates/parts/src/parts/modules_catalogue/module_attachment.rs` | 1 | 0 |
| `web/crates/parts/src/parts/placement_action_tests.rs` | 2 | 2 |
| `web/crates/parts/src/parts/preview.rs` | 17 | 3 |
| `web/crates/parts/src/parts_import_footprint.rs` | 1 | 1 |
| `web/crates/pcb/src/board_reference_owner.rs` | 2 | 2 |
| `web/crates/pcb/src/pcb_board_reference.rs` | 7 | 5 |
| `web/crates/pcb/src/pcb_layers.rs` | 2 | 2 |
| `web/crates/pcb/src/pcb_module_footprints.rs` | 12 | 0 |
| `web/crates/pcb/src/pcb_module_inspector.rs` | 15 | 1 |
| `web/crates/pcb/src/pcb_physical_setup/controller.rs` | 6 | 2 |
| `web/crates/pcb/src/pcb_scene.rs` | 9 | 1 |
| `web/crates/pcb/src/pcb_wiring.rs` | 17 | 0 |
| `web/crates/pcb/src/pcb_wiring/part_input_settings.rs` | 6 | 0 |
| `web/crates/runtime/src/case_preview.rs` | 1 | 1 |
| `web/crates/runtime/src/edit_ticket.rs` | 2 | 2 |
| `web/crates/runtime/src/export_lease.rs` | 3 | 3 |
| `web/crates/runtime/src/layout_viewer_source.rs` | 1 | 1 |
| `web/crates/runtime/src/model_delivery.rs` | 1 | 1 |
| `web/crates/runtime/src/portable_archive.rs` | 1 | 1 |
| `web/crates/runtime/src/runtime.rs` | 4 | 4 |
| `web/crates/ui-shared/examples/canvas_layers.rs` | 1 | 1 |
| `web/crates/ui-shared/examples/dock_layout.rs` | 4 | 0 |
| `web/crates/ui-shared/src/canvas_layers.rs` | 6 | 0 |
| `web/crates/ui-shared/src/footprint_graphics.rs` | 6 | 0 |
| `web/crates/ui-shared/src/layer_dock.rs` | 1 | 0 |
| `web/crates/ui-shared/src/panels.rs` | 5 | 4 |
| `web/crates/ui-shared/src/project_menu.rs` | 1 | 1 |
| `web/crates/ui-shared/src/panels_resize_tests.rs` | 6 | 0 |
| `web/crates/ui-shared/src/panels_scroll_tests.rs` | 6 | 0 |
| `web/tests/matrix_setup_operation.rs` | 1 | 1 |

## Pre-merge verification

- `cargo check --locked -p boardstudio-web-pcb`: failed as expected on native UI-shared runtime cfg gating (see `run4-pcb-check.log`); following WASM checks passed.
- `cargo check --locked --target wasm32-unknown-unknown` with `-p boardstudio-web-pcb`, then PCB/keymap/keycaps, then parts/pcb/ui-shared `--examples`, then library: all passed (`run4-{pcb-wasm,scenes,parts-ui,library}-check.log`).
- `python3 scripts/check.py build`: passed (`run4-build.log`); generated Tailwind output was inspected for carousel ancestor, settings, dock, SVG, and state variants.
- `cargo test --locked --lib -p boardstudio-web-ui-shared -p boardstudio-web-parts -p boardstudio-web-pcb -p boardstudio-web-case -p boardstudio-web-keymap -p boardstudio-web-keycaps -p boardstudio-web-library`: 137 passed (`run4-native.log`).
- `python3 scripts/check.py browser --only ui-shared parts pcb case keymap keycaps library --keep-going`: 409 passed (`run4-browser.log`); partial browser gate only.
- `python3 scripts/run-wasm-tests.py --files` on case, keycaps, keymap, layout, Parts, PCB, setup workspace modules and layout layers: 16 passed (`run4-page.log`).
- `cargo fmt --all -- --check` and `git diff --check` passed. No full gate, mutation, visual equivalence review, or tooling probe was run; integrator owns those gates. Theme/viewport visual equivalence is not proven by mounted interaction tests.

## Parent merge and affected re-verification

Merged `lane/024-delete-the-old-layer` at `4b2087a329e7df0a479d50ba21d224c7dcff96bc` in this worktree; no conflicts. Its Layout focus fallback no longer selects the old Inspector class, which was removed in the merge.

- `cargo check --locked -p boardstudio-web --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web`: passed (`run4-merge-check.log`).
- `cargo test --locked --lib -p boardstudio-web-ui-shared -p boardstudio-web-layout`: 97 passed, 1 ignored (`run4-merge-native.log`).
- `python3 scripts/check.py browser --only ui-shared layout --keep-going`: 247 passed, including the generated layout-test-css step (`run4-merge-browser.log`).
- `python3 scripts/run-wasm-tests.py --files` on Layout layers and workspace plus Parts, PCB, Keymap, Keycaps and Case workspace modules: **14 passed, 1 failed** (`run4-merge-page.log`). `paired_rejoin_preview_findings_reach_editor_footer_panel_and_clear_before_commit` times out because sibling B removed `.m1-layout-findings` and `.m1-layout-finding` while the page test still queries them at `web/src/presentation/layout_layers_tests.rs:184,324`. Parent outcome identifies sibling A as the owner of the matching accessible-selector change. This is an integration blocker, not a passing page gate; the now-vacuous `.m1-layers` absence assertion in that file also needs a role/name replacement.
- Post-merge `cargo fmt --all -- --check` and `git diff --check` passed. The one full build was **before** the merge, per the run budget; the integrator must rebuild the integrated tree and rerun the page gate after sibling A lands. No visual/theme/viewport equivalence, mutation, full gate, or independent review is claimed.
