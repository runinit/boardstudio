# 024 sub-lane A outcome — run 3 partial

Base `64763a392dd32964db9d3a6316aa62be334d3e59`; branch `lane/024a-shell-migrate`. **Not a completed migration or ready for 024 closure.** Eight more owned files now have no legacy classes: Case settings example, Case collision and viewer, PCB position inspector, Keymap workspace, selected-keys inspector, Parts placeholder, and ZMK export row. Prior run detail and its RED/GREEN and mutation results remain at `git show 18deb3326:.evidence/024/sub-a-outcome.md`.

**Remaining in owned paths: 153 distinct class names / 224 class uses / 57 dotted references.** These are untriaged, not justified needs-home exceptions. The shell, Case dock/viewer and export trees still have contextual, descendant, responsive and SVG styling dependencies; replacing names without matching the legacy cascade or changing class-dependent test assertions would be unsafe. Examples include `.m1-case-panel` child/`:has()` rules, `m1-case-assembly-layers` in both docked and standalone contexts, and `.m1-matrix-key` state queries. See the file counts and complete name inventory below. No CSS or ticket was directly edited.

The user-authorized file-level approach was used; no assertions or event handlers were changed in run 3. The three identical Keymap unavailable class replacements were made in one patch, rather than the requested asserted pattern script: a process deviation despite subsequent green verification. The referenced research note was absent from this checkout. Target `/home/chris/.cache/boardstudio-target/024a-shell-migrate` is covered by `.claude/settings.json`'s `/home/chris/.cache/boardstudio-target` allowWrite entry.

## Verification

Long logs are under `.evidence/024/`, prefixed `run3-`; browser/page tests do not establish pixel-level equivalence or a full Theme/viewport visual matrix.

- `case_viewer.rs`: `python3 scripts/check.py build` passed; `python3 scripts/check.py browser --only case` 100/100; `cargo test -p boardstudio-web-case --locked --lib` 34/34 (`run3-case-viewer-{build,browser,native}.log`).
- `case_collision.rs`: the same Case browser 100/100 and native 34/34 (`run3-case-collision-{browser,native}.log`).
- `case/examples/settings.rs`: `cargo check --locked -p boardstudio-web-case --example settings --target wasm32-unknown-unknown` passed and Case browser 100/100 (`run3-settings-example-{check,browser}.log`). The dedicated `scripts/test-case-settings.py` fixture was not run: it sets `CARGO_PROFILE_DEV_DEBUG` itself, contrary to the current build-flag guardrail. Thus the example wrapper was compiled, not individually browser-verified.
- `parts_workspace.rs`: `cargo test -p boardstudio-web --locked --lib` passed (0 native tests), and `python3 scripts/run-wasm-tests.py --files web/src/presentation/parts_workspace.rs` passed 1/1 (`run3-parts-{native,page}.log`).
- `layout_key_selection_actions.rs`: same web native command passed; its page subset passed 6/6 (`run3-key-actions-{native,page}.log`).
- `keymap_workspace.rs`: same web native command passed; `python3 scripts/check.py browser --only keymap` passed 46/46; its page subset passed 1/1 (`run3-keymap-{native,browser,page}.log`).
- `zmk_firmware_export.rs`: same web native command passed; ZMK + Export workspace page subsets passed 6/6, including after cascade correction (`run3-zmk-cascade-{native,page}.log`).
- `inspector.rs`: same web native command passed; inspector + PCB workspace page subsets passed 12/12 (`run3-inspector-{native,page}.log`). The hidden headings match the only production caller, PCB inspector; its standalone test fixture now also hides them.
- `python3 scripts/check.py build` passed on final owned changes (`run3-final-build.log`); generated declarations were audited (`run3-css-audit.log`). `git diff --check 18deb3326` passed and the complete run-3 owned diff was read.

### Requested parent merge and affected checks

After the source commits, the parent advanced concurrently from `64763a392` to `4b2087a32`; merging `lane/024-delete-the-old-layer` into this branch produced `73ae64214`, importing the sibling Layout changes and its evidence. Those unowned files were imported by the requested merge only, not directly edited. Post-merge:

- `python3 scripts/check.py build` passed (`run3-postmerge-build.log`).
- `cargo test -p boardstudio-web -p boardstudio-web-case -p boardstudio-web-layout --locked --lib` passed: web 0, Case 34, Layout 80 with one ignored (`run3-postmerge-native.log`); the Case example WASM check passed (`run3-postmerge-example.log`).
- `python3 scripts/check.py browser --only case keymap layout` passed: Case 100, Keymap 46, Layout 164 plus setup guide 2 (`run3-postmerge-browser.log`).
- `python3 scripts/run-wasm-tests.py --files` over Case workspace, Export workspace, Inspector, Keymap, Layout component inspector, selected-keys actions, layers, relation highlights, Layout workspace, Parts workspace, PCB workspace and ZMK passed 66/66 (`run3-postmerge-page.log`). The paired sibling findings check alone passed 2/2 (`run3-postmerge-findings.log`).
- Generated CSS declarations for the new heights, media rule, touch action, widths, ZMK title and unclamped description exist (`run3-postmerge-css-audit.log`); this is not a computed-style/visual regression check.

Full gate, strict old-CSS check, full browser suite, independent combined review and Theme/viewport visual checks were not run; this sub-lane is incomplete and the coordinator owns closure. The merge did not change any owned source file. No push to origin.

## Per-file inventory against base

Class counts use tokens after `class:` only up to the first comma on that source line, plus dotted legacy references anywhere on the line. This deliberately avoids counting the `m1-workspace-panel` ID after a class on the same line; it is still a line-based heuristic, not a parser, and differs by one from the prior run's `presentation.rs` counts. Includes all matching owned Rust files at either revision. IDs and domain data are not to be renamed as styling.

| File | Base classes + dotted | Current classes + dotted |
| --- | ---: | ---: |
| `web/crates/case/examples/settings.rs` | 1 | 0 |
| `web/crates/case/src/cad_presentation.rs` | 8 | 7 |
| `web/crates/case/src/case_assembly_layers.rs` | 10 | 10 |
| `web/crates/case/src/case_bodies.rs` | 15 | 15 |
| `web/crates/case/src/case_collision.rs` | 1 | 0 |
| `web/crates/case/src/case_viewer.rs` | 1 | 0 |
| `web/crates/case/src/mechanical_settings.rs` | 135 | 32 |
| `web/crates/case/src/shared_viewer.rs` | 23 | 22 |
| `web/crates/case/src/shared_viewer_020_tests.rs` | 4 | 2 |
| `web/src/presentation.rs` | 50 | 48 |
| `web/src/presentation/case_workspace.rs` | 21 | 14 |
| `web/src/presentation/export_workspace.rs` | 20 | 14 |
| `web/src/presentation/inspector.rs` | 1 | 0 |
| `web/src/presentation/inspector/companion_placement.rs` | 3 | 0 |
| `web/src/presentation/inspector/layout_component_inspector.rs` | 20 | 16 |
| `web/src/presentation/keycaps_workspace.rs` | 7 | 4 |
| `web/src/presentation/keymap_workspace.rs` | 6 | 0 |
| `web/src/presentation/layout_canvas.rs` | 14 | 14 |
| `web/src/presentation/layout_component_inspector_tests.rs` | 8 | 2 |
| `web/src/presentation/layout_key_selection_actions.rs` | 2 | 0 |
| `web/src/presentation/layout_key_selection_actions/tests.rs` | 8 | 0 |
| `web/src/presentation/layout_layers_tests.rs` | 27 | 25 |
| `web/src/presentation/layout_matrix_relations.rs` | 26 | 12 |
| `web/src/presentation/layout_relation_highlight_tests.rs` | 20 | 20 |
| `web/src/presentation/layout_workspace.rs` | 19 | 14 |
| `web/src/presentation/parts_workspace.rs` | 2 | 0 |
| `web/src/presentation/pcb_workspace.rs` | 10 | 10 |
| `web/src/presentation/zmk_firmware_export.rs` | 8 | 0 |

Base: 210 names, 385 class uses, 85 dotted references.
Current: 153 names, 224 class uses, 57 dotted references.

Current quoted class names:
`board-tool-pill`, `m1-add-icon`, `m1-board-context`, `m1-canvas`, `m1-canvas-context`, `m1-canvas-delete-host`, `m1-canvas-placement-hint`, `m1-canvas-toolbar`, `m1-case-assembly-layer-group`, `m1-case-assembly-layer-heading`, `m1-case-assembly-layer-label`, `m1-case-assembly-layer-swatch`, `m1-case-assembly-layers`, `m1-case-assembly-layers-list`, `m1-case-bodies`, `m1-case-bodies-generated`, `m1-case-bodies-heading`, `m1-case-body-editor`, `m1-case-body-list`, `m1-case-body-tab`, `m1-case-camera-controls`, `m1-case-edit-hint`, `m1-case-header`, `m1-case-live-status`, `m1-case-live-toggle`, `m1-case-measures`, `m1-case-mismatch`, `m1-case-model-delivery`, `m1-case-mount-editor`, `m1-case-panel`, `m1-case-physical-setup`, `m1-case-picked-part-badge`, `m1-case-selection-breadcrumb`, `m1-case-selection-footer`, `m1-case-setup-basics`, `m1-case-subsection`, `m1-case-subsection-content`, `m1-case-view`, `m1-case-view-adjustments`, `m1-case-view-canvas-shell`, `m1-case-view-settings`, `m1-case-view-status`, `m1-case-view-toolbar`, `m1-component`, `m1-component-list`, `m1-design-canvas-toolbar`, `m1-design-view-group`, `m1-editor`, `m1-editor-body`, `m1-editor-body--guided-setup`, `m1-editor-body--layout`, `m1-editor-host`, `m1-export-action`, `m1-export-actions`, `m1-export-current`, `m1-export-heading`, `m1-export-list`, `m1-export-option`, `m1-export-panel`, `m1-export-portable`, `m1-export-rail`, `m1-export-review-links`, `m1-export-row`, `m1-export-row-action`, `m1-export-row-copy`, `m1-export-section-title`, `m1-export-workspace`, `m1-inspector-lock`, `m1-inspector-mode`, `m1-inspector-section`, `m1-keycap-overlay`, `m1-keycap-top`, `m1-keycaps-board-appearance`, `m1-keycaps-canvas`, `m1-keycaps-workspace`, `m1-layout-add-object`, `m1-layout-component-constraint`, `m1-layout-component-constraint-actions`, `m1-layout-component-constraint-grid`, `m1-layout-component-constraint-note`, `m1-layout-component-definition`, `m1-layout-component-electrical`, `m1-layout-component-inspector`, `m1-layout-component-inspector-heading`, `m1-layout-component-matrix-note`, `m1-layout-component-notice`, `m1-layout-component-outline`, `m1-layout-component-position`, `m1-layout-component-relation-summary`, `m1-layout-component-subtitle`, `m1-layout-findings-return`, `m1-layout-key-size-section`, `m1-layout-objects-content`, `m1-layout-snap-guide`, `m1-layout-toolbar-region`, `m1-layout-view-group`, `m1-library-landing`, `m1-matrix-key`, `m1-matrix-placement-preview`, `m1-matrix-transform-section`, `m1-mechanical-fit-points`, `m1-mechanical-group`, `m1-mechanical-opening`, `m1-mechanical-opening-fields`, `m1-mechanical-opening-title`, `m1-mechanical-opening-vertex`, `m1-mechanical-openings`, `m1-mechanical-pair-row`, `m1-mechanical-profile`, `m1-mechanical-settings`, `m1-mechanical-spec-card`, `m1-mechanical-spec-group`, `m1-mirror-pair-axis`, `m1-mirrored-pair-cell`, `m1-mirrored-pair-preview`, `m1-object-kind`, `m1-object-layer-dock`, `m1-object-scroll-content`, `m1-object-tree`, `m1-object-tree-list`, `m1-objects`, `m1-outline`, `m1-outline-bridge-selected`, `m1-part`, `m1-part-drill`, `m1-part-hit-area`, `m1-part-label`, `m1-part-pad`, `m1-part-placement-envelope`, `m1-part-placement-preview`, `m1-pcb-canvas`, `m1-pcb-inspector`, `m1-pcb-objects-content`, `m1-pcb-toolbar-region`, `m1-pcb-workspace`, `m1-save-error`, `m1-scene-part`, `m1-selected-context`, `m1-setup-case-settings`, `m1-setup-case-settings__content`, `m1-setup-drawer`, `m1-shared-viewer`, `m1-stackup-preview`, `m1-status`, `m1-tab-icon`, `m1-transform-handle`, `m1-tree-glyph`, `m1-tree-label`, `m1-tree-row`, `m1-tree-spacer`, `m1-viewer-layer`, `m1-workbench`, `m1-workspace-content`

Dotted-only names: `m1-inspector-body`, `m1-layers`
