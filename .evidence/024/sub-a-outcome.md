# 024 sub-lane A outcome — run 4

Base `64763a392dd32964db9d3a6316aa62be334d3e59`; branch `lane/024a-shell-migrate`. The assigned Rust migration is complete: **0 distinct literal legacy class names / 0 class uses / 0 dotted legacy selector names or uses** under `web/src/**`, `web/crates/case/**` and `web/crates/catalogue/**`, by the line-based inventory described below. Stable IDs, ARIA targets, domain board IDs and IDs embedded in utility selectors remain. Earlier run details and verification are preserved in `git show edc6c99f7:.evidence/024/sub-a-outcome.md`.

Run 4 replaces shell, canvas, inspector, export, workspaces, Case viewer, bodies, assembly layers and mechanical settings styling with existing utilities and Theme tokens. The Layout findings region and rows were already selected semantically at the starting revision; the remaining layer/canvas tests now use observable geometry/state data for noninteractive SVG and semantic names for controls. No new per-utility assertions, mutation checks or computed-style tests were added. Existing Case fixture assertions are retained and now load the generated utility stylesheet. IDs and data used for behavior were not renamed. No CSS, ticket, or sibling-owned Layout file was directly edited.

Source commits `459186ce`, `4db0a8e5`, `26f543d1`, `44ba060b`; parent `lane/024-delete-the-old-layer` advanced to `c8fa341ad` and was merged locally as `6ab47787f`, importing C's files without direct edits. After that merge, the docked canvas-layer container was given visible overflow to preserve the existing upward popup placement when C's migrated Collapsible removed the legacy ancestor selector. A final commit contains that integration adjustment, fixture stylesheet fixes and this handoff. No push.

## Verification

Logs are under `.evidence/024/run4-*.log`; the target path was covered by the worktree's sandbox allowWrite entry. Checks are partial, not the ticket-closure gate.

- `cargo check --locked -p boardstudio-web --target wasm32-unknown-unknown` and `cargo check --locked -p boardstudio-web-case --target wasm32-unknown-unknown`: pass during groups.
- `python3 scripts/check.py build`: pass before the parent merge (`run4-final-build.log`); the supported `dx build --web --no-default-features --features page --cargo-args=--locked` and generated utilities were rebuilt after the parent merge and final shell integration adjustment (`run4-final-integrated-page-build.log`). The complete provider build was not repeated after merge, per the one-build budget.
- Initial `python3 scripts/check.py browser --only case layout keymap pcb --keep-going`: Case failed its existing layer-menu display assertion (expected grid, got block) because its fixture loaded only the old stylesheet; 24 follow-on panic-hook failures and a timeout followed. PCB 56, Layout 164+2 and Keymap 46 passed. The fixture was changed to load the generated stylesheet; the exact failing test then passed 1/1, and `check.py browser --only case` passed 100/100.
- After parent merge, `python3 scripts/check.py browser --only case layout keymap pcb ui-shared library --keep-going`: pass — Case 100, PCB 57, Layout 164+2, UI-shared 79+2, Library 19, Keymap 48 (`run4-postmerge-browser.log`).
- After merge, `cargo test -p boardstudio-web -p boardstudio-web-case -p boardstudio-web-layout --locked --lib`: pass — web 0, Case 34, Layout 80 with 1 ignored (`run4-postmerge-native.log`).
- After merge, `python3 scripts/run-wasm-tests.py --files` for Layout canvas, component inspector, layers, relation highlights, Layout workspace, Case, Export, Keycaps, PCB and Keymap workspaces: 50/50 (`run4-postmerge-page.log`). After the dock adjustment, the Layout layers and Case workspace subset passed 6/6 (`run4-postmerge-findings.log`).
- The exact `presentation::layout_layers_tests::paired_rejoin_preview_findings_reach_editor_footer_panel_and_clear_before_commit` passed 1/1 after merge, with the sibling Layout findings classes absent (`run4-paired-findings.log`).
- `cargo fmt --all -- --check` and `git diff --check`: pass. Generated CSS includes the SVG stroke/fill and dock utilities. This is not a computed-style or screenshot equivalence claim.

**Integrator gates/risks:** the full gate, independent combined spec/standards review, strict old-CSS gate and Theme/viewport screenshot matrix remain the coordinator's responsibility. Cross-lane C reports one outside-A-scope `m1-project-menu` fixture class and its functional close/focus query; coordinate that dependency before strict deletion. Its merged `ui-shared/src/layer_dock.rs` includes an assertion on `empty:hidden`, outside A's ownership and contrary to this run's no-new-utility-assertion rule; coordinator should resolve/review it. Parent Library and dock migrations interact with the shell; the screenshot matrix must verify their desktop/compact layout, scrolling and popup placement. The new dock overflow restoration is covered by page interactions but not by a visual clipping assertion. Existing test fixture CSS embeds the generated stylesheet, so generate it before standalone Case browser tests.

**Process deviation:** an initial read-only inventory output was written to `/tmp/024a-inventory.txt`, outside the requested worktree-only boundary. No main checkout file was modified; initial main status had untracked files only. Existing prior-run evidence logs were preserved rather than deleting tracked handoff records.

## Per-file inventory vs base

Each pair is distinct literal legacy class tokens in a `class:` expression / distinct dotted legacy selector names anywhere in that file. A line-based heuristic reads the class expression up to its first comma; it excludes IDs and domain data and is not a Rust parser. Across scope the same heuristic finds 210 distinct class names and 385 class uses plus 26 dotted names and 85 dotted uses at base, versus zero of each now. The table includes every base/current matching owned Rust file; named files with only IDs have zero in both columns.

| File | Base classes/dotted | Current classes/dotted |
| --- | ---: | ---: |
| `web/crates/case/examples/settings.rs` | 1/0 | 0/0 |
| `web/crates/case/src/cad_presentation.rs` | 6/0 | 0/0 |
| `web/crates/case/src/case_assembly_layers.rs` | 6/0 | 0/0 |
| `web/crates/case/src/case_bodies.rs` | 11/0 | 0/0 |
| `web/crates/case/src/case_collision.rs` | 1/0 | 0/0 |
| `web/crates/case/src/case_viewer.rs` | 1/0 | 0/0 |
| `web/crates/case/src/mechanical_settings.rs` | 32/2 | 0/0 |
| `web/crates/case/src/shared_viewer.rs` | 19/2 | 0/0 |
| `web/crates/case/src/shared_viewer_020_tests.rs` | 2/2 | 0/0 |
| `web/crates/catalogue/src/physical_setup.rs` | 0/0 | 0/0 |
| `web/src/presentation.rs` | 38/2 | 0/0 |
| `web/src/presentation/canvas_status_footer.rs` | 0/0 | 0/0 |
| `web/src/presentation/case_workspace.rs` | 15/2 | 0/0 |
| `web/src/presentation/export_workspace.rs` | 18/2 | 0/0 |
| `web/src/presentation/header.rs` | 0/0 | 0/0 |
| `web/src/presentation/inspector.rs` | 1/0 | 0/0 |
| `web/src/presentation/inspector/companion_placement.rs` | 3/0 | 0/0 |
| `web/src/presentation/inspector/layout_component_inspector.rs` | 19/0 | 0/0 |
| `web/src/presentation/keycaps_workspace.rs` | 5/0 | 0/0 |
| `web/src/presentation/keymap_workspace.rs` | 4/0 | 0/0 |
| `web/src/presentation/layout_canvas.rs` | 7/4 | 0/0 |
| `web/src/presentation/layout_component_inspector_tests.rs` | 1/3 | 0/0 |
| `web/src/presentation/layout_key_selection_actions.rs` | 2/0 | 0/0 |
| `web/src/presentation/layout_key_selection_actions/tests.rs` | 0/1 | 0/0 |
| `web/src/presentation/layout_layers_tests.rs` | 0/7 | 0/0 |
| `web/src/presentation/layout_matrix_relations.rs` | 15/0 | 0/0 |
| `web/src/presentation/layout_relation_highlight_tests.rs` | 0/1 | 0/0 |
| `web/src/presentation/layout_workspace.rs` | 16/0 | 0/0 |
| `web/src/presentation/parts_workspace.rs` | 2/0 | 0/0 |
| `web/src/presentation/pcb_workspace.rs` | 10/0 | 0/0 |
| `web/src/presentation/workspace_navigation.rs` | 0/0 | 0/0 |
| `web/src/presentation/zmk_firmware_export.rs` | 5/2 | 0/0 |
