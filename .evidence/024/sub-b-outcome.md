# 024 sub-lane B outcome

## Run 4 result

Started clean at `80de3d14e` on `lane/024b-layout-migrate`; grouped commits `cdec9cfc`, `3964eb00`, and `e05b4396` complete the layout-side legacy class migration using existing theme tokens and utilities, including canvas overlays, objects, inspectors, setup and test fixtures. Prior run history is available in `git show 80de3d14e:.evidence/024/sub-b-outcome.md`. No CSS or ticket was edited. Parent `lane/024-delete-the-old-layer` was `64763a392` and `git merge --no-edit` reported already up to date; final checks below ran afterwards. No push.

**Remaining:** zero literal `m1-*`/`board-*` class tokens in layout `class:` expressions and zero dotted legacy selectors in layout Rust. Nonzero broad-prefix counts below are not outstanding classes: they include IDs/ARIA targets, test helpers that strip legacy prefixes, and domain fixture board IDs. These cannot safely be renamed as if they were styling. The removed setup `step_class` helper is included in the base class counts. Direct code did not change selection, focus routing, geometry calculations or event handlers; SVG pointer/touch and form placement styling was migrated and exercised by the existing browser subset.

### Paired sibling dependency (blocking page check)

- `web/crates/layout/src/layout_findings.rs:101` (findings region) and `:188` (finding row): removed `m1-layout-findings` and `m1-layout-finding`; **pairs with sibling A layout_layers_tests change**. This checkout's `web/src/presentation/layout_layers_tests.rs:184,324` still queries those old classes; sibling `lane/024a-shell-migrate` from `e666c7e41` onward selects the accessible Layout findings region and list items. Coordinator must merge sibling A and rerun the page subset. Until then the one page test below fails; a passing integrated result is not claimed.

## Per-file inventory against `64763a392`

Counts in the first pair are distinct legacy CSS class tokens in direct `class:` expressions (plus the old setup step helper), not IDs. The second pair uses the previous run's broad literal-prefix/dotted regex `"(?:m1-|board-)|(?<![A-Za-z0-9_])\.(?:m1-|board-)` and counts occurrences, including nonclass strings. The table covers every matching or changed layout Rust file at either revision.

| File under `web/crates/layout/` | Classes base | Classes now | Broad base | Broad now |
| --- | ---: | ---: | ---: | ---: |
| `examples/toolbar_strips.rs` | 2 | 0 | 2 | 0 |
| `src/board_inspector.rs` | 4 | 0 | 10 | 8 |
| `src/layout_findings.rs` | 6 | 0 | 9 | 6 |
| `src/layout_findings_state.rs` | 0 | 0 | 13 | 4 |
| `src/layout_viewer.rs` | 2 | 0 | 17 | 8 |
| `src/matrix_relations/tests.rs` | 0 | 0 | 1 | 1 |
| `src/matrix_transform_operation.rs` | 0 | 0 | 1 | 1 |
| `src/objects.rs` | 23 | 0 | 27 | 3 |
| `src/objects/board_setup_controller.rs` | 0 | 0 | 2 | 2 |
| `src/objects/existing_half.rs` | 3 | 0 | 3 | 1 |
| `src/objects/keycap_size.rs` | 6 | 0 | 6 | 0 |
| `src/objects/keycap_size_tests.rs` | 0 | 0 | 10 | 10 |
| `src/objects/layout_align.rs` | 0 | 0 | 1 | 1 |
| `src/objects/layout_align_controller.rs` | 0 | 0 | 1 | 1 |
| `src/objects/layout_canvas_controls.rs` | 2 | 0 | 2 | 0 |
| `src/objects/layout_toolbar.rs` | 2 | 0 | 26 | 25 |
| `src/objects/layout_transform_toolbar.rs` | 8 | 0 | 16 | 4 |
| `src/objects/matrix_edit_test_support.rs` | 0 | 0 | 1 | 1 |
| `src/objects/matrix_inspector.rs` | 8 | 0 | 17 | 0 |
| `src/objects/matrix_inspector_controller.rs` | 0 | 0 | 3 | 2 |
| `src/objects/matrix_setup.rs` | 7 | 0 | 10 | 2 |
| `src/objects/matrix_setup_focus_tests.rs` | 0 | 0 | 1 | 0 |
| `src/objects/matrix_transform_inspector.rs` | 18 | 0 | 33 | 0 |
| `src/objects/matrix_transform_inspector_tests.rs` | 0 | 0 | 2 | 0 |
| `src/objects/mirrored_pair.rs` | 9 | 0 | 8 | 0 |
| `src/objects/mirrored_pair/canvas_overlay.rs` | 1 | 0 | 1 | 0 |
| `src/objects/mirrored_pair/canvas_overlay_tests.rs` | 1 | 0 | 9 | 3 |
| `src/outline/hook.rs` | 0 | 0 | 0 | 0 |
| `src/outline/inspector.rs` | 34 | 0 | 50 | 0 |
| `src/outline/overlays.rs` | 11 | 0 | 14 | 0 |
| `src/outline_lifecycle_browser_tests.rs` | 0 | 0 | 6 | 2 |
| `src/part_placement.rs` | 0 | 0 | 40 | 40 |
| `src/setup_guide.rs` | 17 | 0 | 27 | 3 |
| `src/setup_guide/tests.rs` | 3 | 0 | 7 | 2 |

## Checks and limitations

All long logs are under `.evidence/024/` (gitignored); previous-run details remain in Git history, but the old logs were cleared at task start.

- `cargo check -p boardstudio-web-layout --target wasm32-unknown-unknown --locked` passed after each group (`run4-group{1,2,3}-wasm-check.log`). An initial native check without test support failed because `boardstudio_web_runtime::runtime` is gated out (`run4-group1-check.log`); this is not the WASM page target.
- `cargo fmt -p boardstudio-web-layout -- --check`, `git diff --check`: pass.
- `python3 scripts/check.py build`: pass (`run4-final-build.log`); generated CSS contains the setup inline-matrix override and SVG pointer/stroke utilities.
- `cargo test -p boardstudio-web-layout --locked --lib`: 80 passed, 1 ignored (`run4-final-native.log`).
- `python3 scripts/check.py browser --only layout`: 164 layout and 2 setup-guide browser tests passed (`run4-final-browser.log`).
- `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_layers_tests.rs`: **1 passed, 1 failed** (`run4-final-page.log`); paired-rejoin test timed out waiting for findings because it still uses the sibling-owned legacy selectors above. This test must pass after integration.
- `python3 scripts/test-layout-toolbar.py`: pass (production strips, owner callbacks, keyboard, widths and four Theme appearances; `run4-toolbar-fixture.log`). Its newly generated screenshots were removed from `.evidence/` to respect the logs-only rule.
- Full gate, mutation checks, independent combined spec/standards review, and integrated application screenshot matrix were not run by this sub-lane. Coordinator owns them; class migration has nontrivial SVG, touch-target, responsive and layout styling risk, and the layout 3D viewer now wraps the shared viewer to retain its flex/canvas sizing. The integrator should verify these visually and rerun the paired page check after sibling A is merged.
- Tooling mishap: an early relative patch addressed the main checkout's `mirrored_pair/canvas_overlay.rs` rather than this worktree. The exact one-line change was immediately reversed; subsequent edits used absolute worktree paths.
