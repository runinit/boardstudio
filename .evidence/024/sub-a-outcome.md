# 024 sub-lane A outcome — still partial

Base `64763a392dd32964db9d3a6316aa62be334d3e59`; branch `lane/024a-shell-migrate`. **Not ready for integration as a completed migration.** In this continuation all 24 previously inventoried dead names and their 12 dotted test occurrences were removed, and two live Case classes were migrated. **161 live names remain untriaged**, not justified needs-home exceptions. They are listed below; the prior full inventory with CSS locations is available from `git show e666c7e41:.evidence/024/sub-a-needs-home.md`. No authored CSS, ticket, or source outside lane ownership was edited.

The Layout findings page test now finds rows in the named findings region without `.m1-layout-finding` or `.m1-layout-findings`, unblocking sibling removal. The companion-placement page test now drives the actual named disclosure and Select and mounts the required notification provider; its alert lookup remains scoped to the disclosure. Other removed test classes were replaced by text or test-only generation markers. No native test names changed.

Case fit measurements use the existing muted foreground, 11px and mono utilities; Case help paragraphs use muted foreground, small text and normal leading. Original tokens were 11px mono and 14px/1.5 muted respectively; new theme tokens intentionally follow the selected Theme. The browser test checks the rendered utility class contract and the build output audit checks the corresponding generated declarations; this is not a pixel-level or visual regression test.

## Per-file counts

Occurrences are legacy names after `class:` on a line plus dotted legacy selectors, compared with the base; quoted IDs/domain data are excluded except where they appear later on the same class line. This reproduces the prior outcome's counts for its nine files.

| File | Base | Current |
| --- | ---: | ---: |
| `web/crates/case/src/cad_presentation.rs` | 8 | 7 |
| `web/crates/case/src/mechanical_settings.rs` | 135 | 32 |
| `web/crates/case/src/shared_viewer.rs` | 23 | 22 |
| `web/crates/case/src/shared_viewer_020_tests.rs` | 4 | 2 |
| `web/src/presentation.rs` | 51 | 49 |
| `web/src/presentation/case_workspace.rs` | 21 | 14 |
| `web/src/presentation/export_workspace.rs` | 20 | 14 |
| `web/src/presentation/inspector/companion_placement.rs` | 3 | 0 |
| `web/src/presentation/inspector/layout_component_inspector.rs` | 20 | 16 |
| `web/src/presentation/keycaps_workspace.rs` | 7 | 4 |
| `web/src/presentation/keymap_workspace.rs` | 6 | 5 |
| `web/src/presentation/layout_component_inspector_tests.rs` | 8 | 2 |
| `web/src/presentation/layout_key_selection_actions.rs` | 2 | 1 |
| `web/src/presentation/layout_key_selection_actions/tests.rs` | 8 | 0 |
| `web/src/presentation/layout_layers_tests.rs` | 27 | 25 |
| `web/src/presentation/layout_matrix_relations.rs` | 26 | 12 |
| `web/src/presentation/layout_workspace.rs` | 19 | 14 |
| `web/src/presentation/parts_workspace.rs` | 2 | 1 |
| `web/src/presentation/zmk_firmware_export.rs` | 8 | 3 |

## Verification

- Findings query RED: expected class-independence assertion; GREEN: 2/2 (`continuation-red-findings.log`, `continuation-green-findings.log`).
- Case measurement and help slices: each RED at the intended utility assertion, then `python3 scripts/check.py browser --only case` GREEN 100/100 (`continuation-{red,green}-fit.log`, `continuation-{red,green}-help.log`). The first fit RED attempt instead exposed a missing `class_list` web-sys feature; the test was corrected to inspect the class attribute without widening features.
- `scripts/mutation-check.py` reverted each Case utility slice to its legacy class: both mutants killed at the intended assertions, zero survivors/compile failures, files restored (`continuation-mutation.log`, `mutation-case/`).
- A class-dependent findings lookup mutant was also killed by the class-independence assertion, with the file restored (`continuation-mutation-findings.log`, `mutation-findings/`).
- `python3 scripts/check.py build`: PASS (`continuation-build.log`). Authored/generated CSS were audited: 24 dead-name selectors absent; new utility declarations present (`continuation-css-audit.log`).
- `cargo test -p boardstudio-web -p boardstudio-web-case --locked --lib`: PASS, 34 Case / 0 web lib tests (`continuation-native.log`).
- `python3 scripts/run-wasm-tests.py --files` for Case, Export, Keycaps, Keymap, Layout component inspector, key-selection actions, layers, relation highlight, Layout workspace, Parts, and ZMK: PASS 57/57 (`continuation-page-final.log`). Component inspector/relation/key-selection slice separately PASS 37/37 (`continuation-inspector-relations.log`). The earlier broad run failed closed on the unmapped relation file and found a pre-existing missing Toast context in the Editor fixture; the later mapped suite and corrected fixture passed.
- After scoping the companion alert, its page subset passed 6/6 (`continuation-page-alert-retry.log`). An initial retry attempt timed out loading Chrome before any test executed (`continuation-page-alert-scope.log`); the subsequent identical run passed.
- Full gate intentionally not run. Prior continuation evidence remains in this directory. Full regex enumeration is `continuation-enumeration.log`.

## Remaining live names — UNTRIAGED

These are not claims of inexpressibility. Each still requires rule-by-rule triage against existing utilities, contextual selectors, responsive behavior and sibling ownership; the two earlier toolbar candidates are included, not silently treated as justified exceptions.

`board-tool-pill`, `m1-add-icon`, `m1-board-context`, `m1-canvas`, `m1-canvas-context`, `m1-canvas-delete-host`, `m1-canvas-placement-hint`, `m1-canvas-toolbar`

`m1-case-assembly-layer-group`, `m1-case-assembly-layer-heading`, `m1-case-assembly-layer-label`, `m1-case-assembly-layer-swatch`, `m1-case-assembly-layers`, `m1-case-assembly-layers-list`, `m1-case-bodies`, `m1-case-bodies-generated`

`m1-case-bodies-heading`, `m1-case-body-editor`, `m1-case-body-list`, `m1-case-body-tab`, `m1-case-camera-controls`, `m1-case-collision-checks`, `m1-case-edit-hint`, `m1-case-header`

`m1-case-keycap-findings`, `m1-case-live-status`, `m1-case-live-toggle`, `m1-case-measures`, `m1-case-mismatch`, `m1-case-model-delivery`, `m1-case-mount-editor`, `m1-case-panel`

`m1-case-physical-setup`, `m1-case-picked-part-badge`, `m1-case-selection-breadcrumb`, `m1-case-selection-footer`, `m1-case-setup-basics`, `m1-case-subsection`, `m1-case-subsection-content`, `m1-case-view`

`m1-case-view-adjustments`, `m1-case-view-canvas-shell`, `m1-case-view-settings`, `m1-case-view-status`, `m1-case-view-toolbar`, `m1-component`, `m1-component-list`, `m1-design-canvas-toolbar`

`m1-design-view-group`, `m1-editor`, `m1-editor-body`, `m1-editor-body--guided-setup`, `m1-editor-body--layout`, `m1-editor-host`, `m1-export-action`, `m1-export-actions`

`m1-export-current`, `m1-export-heading`, `m1-export-list`, `m1-export-option`, `m1-export-panel`, `m1-export-portable`, `m1-export-rail`, `m1-export-review-links`

`m1-export-row`, `m1-export-row-action`, `m1-export-row-copy`, `m1-export-section-title`, `m1-export-workspace`, `m1-inspector`, `m1-inspector-body`, `m1-inspector-lock`

`m1-inspector-mode`, `m1-inspector-section`, `m1-keycap-overlay`, `m1-keycap-top`, `m1-keycaps-board-appearance`, `m1-keycaps-canvas`, `m1-keycaps-workspace`, `m1-keymap-panel`

`m1-keymap-unavailable`, `m1-layers`, `m1-layout-add-object`, `m1-layout-component-constraint`, `m1-layout-component-constraint-actions`, `m1-layout-component-constraint-grid`, `m1-layout-component-constraint-note`, `m1-layout-component-definition`

`m1-layout-component-electrical`, `m1-layout-component-inspector`, `m1-layout-component-inspector-heading`, `m1-layout-component-matrix-note`, `m1-layout-component-notice`, `m1-layout-component-outline`, `m1-layout-component-position`, `m1-layout-component-relation-summary`

`m1-layout-component-subtitle`, `m1-layout-findings-return`, `m1-layout-key-size-section`, `m1-layout-objects-content`, `m1-layout-snap-guide`, `m1-layout-toolbar-region`, `m1-layout-view-group`, `m1-library-landing`

`m1-matrix-key`, `m1-matrix-placement-preview`, `m1-matrix-transform-section`, `m1-mechanical-fit-points`, `m1-mechanical-group`, `m1-mechanical-opening`, `m1-mechanical-opening-fields`, `m1-mechanical-opening-title`

`m1-mechanical-opening-vertex`, `m1-mechanical-openings`, `m1-mechanical-pair-row`, `m1-mechanical-profile`, `m1-mechanical-settings`, `m1-mechanical-spec-card`, `m1-mechanical-spec-group`, `m1-mirror-pair-axis`

`m1-mirrored-pair-cell`, `m1-mirrored-pair-preview`, `m1-object-kind`, `m1-object-layer-dock`, `m1-object-scroll-content`, `m1-object-tree`, `m1-object-tree-list`, `m1-objects`

`m1-outline`, `m1-outline-bridge-selected`, `m1-part`, `m1-part-drill`, `m1-part-hit-area`, `m1-part-label`, `m1-part-pad`, `m1-part-placement-envelope`

`m1-part-placement-preview`, `m1-pcb-canvas`, `m1-pcb-inspector`, `m1-pcb-objects-content`, `m1-pcb-toolbar-region`, `m1-pcb-workspace`, `m1-placeholder-workspace`, `m1-save-error`

`m1-scene-part`, `m1-selected-context`, `m1-setup-case-settings`, `m1-setup-case-settings__content`, `m1-setup-drawer`, `m1-shared-viewer`, `m1-stackup-preview`, `m1-status`

`m1-tab-icon`, `m1-transform-handle`, `m1-tree-glyph`, `m1-tree-label`, `m1-tree-row`, `m1-tree-spacer`, `m1-viewer-layer`, `m1-workbench`

`m1-workspace-content`
