# 002: Layout's measured 42px accordion and existing tools

Status: claimed
Type: build
Blocked by: [009](009-shared-toolbar-panel-host.md)
Parent: [map.md](../map.md)
Owner: Layout toolbar/transform/align modules and `web/assets/layout-toolbar.css`

## Boundary

Use 009's shared row/utility host, not new panel flags in LayoutWorkspaceState. This slice
owns Layout's `Option<ToolbarSection>` and Select/Transform/Align/Snap/View content. Reuse
LayoutSelectionKind/context conversion, LayoutContextActionToolbar activation/guards,
LayoutAlignMount, LayoutSnapSettings/Intent and existing visibility/view state. PCB may
reuse these mounts in 010; do not force them into other domains. Feed 009's single generic
fitting/clip implementation, not a second Layout engine. The coordinator serializes any
shared-host hunks; this ticket owns Layout action composition and its measured acceptance.

Measure the final post-toggle strip before motion; action labels outrank captions. Translate
icon+label units under a stationary clip, never opacity/stagger/pop-in; reduced motion is
instant. Follow [responsive/keyboard literals](../architecture.md#responsive-and-keyboard-contract-proposed).
Merge snap-in-dock/duplicate Footprints into one owner. View includes existing 2D/3D and
visibility/handles. Keep camera in footer, canvas manipulation rails/pick overlays intact.
Save/recovery alerts move to shell status in 006, not inside the bounded strip.

## Acceptance criteria

- [ ] All actions in the Layout tools inventory reachable with unchanged guards, reasons,
      reference selection (>12 combobox), Relationships and snap settings.
- [ ] One 42px row, all seven entry points visible with each section open; action overflow
      scrolls without page overflow, trigger movement or scope popups.
- [ ] No duplicate position, snap or footprint editor and no second permanent tool row.
- [ ] Existing tool assertions preserved; only uncovered interaction stories gain tests.

## Proof map

- Bounded fitting — 320/390/728/desktop, real built Theme/toolbar stylesheet and font loaded,
  actual Layout action/section controls mounted → open each section/resize →
  height 42px, seven entry points hit-testable, one full action available, action labels
  allocated before captions and before movement · mounted · planned
- Solid motion — labelled section moving, then rapid switch → sample initial/middle/final
  frames → constant icon/text offset and opacity 1, stationary clip, no stale transition · mounted · planned
- Reduced motion — preference reduce before open → toggle section/detail → final state
  immediate and zero running workbench animations · mounted · planned
- Availability — existing first-column / Follow-rotation-off / locked / empty fixtures →
  attempt tool activation → unchanged disabled reason and accepted state; legal tools still work · mounted · planned
- Action parity — valid selection with >12 references → Align, change reference, open Snap
  details and toggle View → named action/result matches existing behaviour, no duplicate entry · mounted · planned

## Checks and mutations

[Verification contract](../verification.md): `check.py quick`, `check.py lint`,
`check.py browser --only layout`; page tests selected with `run-wasm-tests.py --files` when
page wiring changes. Motion/measurement/reduced-motion/guard proof-gap mutations are named
there. Run one combined Spec+Standards review and affected gate before closure.

## Implementation progress

- 2026-11-11 Snap builder (002-snap): RED was `accordion_sections_keep_existing_tools_inline` at `by_role_name(&root, "combobox", "Snap increment")` (`.evidence/toolbar-m1/002-accordion-green.log`). Diagnosed as genuinely wrong role expectation: `LayoutSnapIncrementSelect` renders shared `Select` with `aria_label "Snap increment"`, whose trigger semantics are role `button` (listbox + options), not `combobox`. Independent evidence: `primitives_mount_tests::select_floor_label_navigation_selection_and_focus_return` (`button` "Fruit"), `select_close_requests_are_not_duplicated_while_popup_is_retained` (`button` "Select an option"), `scripts/test-command-palette.py` `action("button", "Snap increment")`, and `combobox_floor_*` (`combobox` "Search fruit" for `Combobox` input); closed detail `m1-layout-snap-menu` is removed from DOM so inline Select is unique. Changed only that one role string `combobox`->`button` in `web/crates/layout/src/objects/layout_toolbar.rs:670`; no production/CSS change. GREEN: `python3 scripts/check.py browser --only layout` -> `.evidence/toolbar-m1/002-snap-button.log` (269 passed + 2 setup_guide passed, `accordion_sections_keep_existing_tools_inline ... ok`). Five sections, Transform-once-inline, Select/Snap/View switching, closed-Transform inertness still asserted. No closure claim.
- 2026-11-11 Page builder (002-page-consumer): RED was `actual_layout_tool_row_has_seven_entry_points_and_stable_canvas` at `assert_eq!(count(&root, ".workbench-tool-row"), 1)` (`.evidence/toolbar-m1/002-page-red.log`, executed 7 failed 1, left 0 right 1, existing 6 pass). Implemented actual Layout ToolRow consumer (owned files only, no crate edits): `layout_workspace::ToolbarInput` + `handles_visible`/`on_toggle_handles`/`canvas_layers`; workspace-leave clears `command_menu` section; `layout_workspace::toolbar` rebuilt as normal-flow `ToolRow` (fit_key from section/selection/tool/snap/assembly/footprints/handles/context/transform/align; 2D pill `accordion:true` `snap_in_dock:false` with View (2D/3D idx41-42, Footprints idx43 if 2D, CanvasLayers Footprints-filtered, handles idx44) + `PanelUtilities`; 3D ToolRow 2D/3D+utilities only, pill hidden, no second row; save/recovery below row in flex-none div; wrapper div flex-none `data-workspace-toolbar=layout` with Space guard); `presentation.rs` View data construction (Footprints filtered from CanvasLayers groups, ToggleGroup sole owner), footer removes Layout CanvasLayers+SnapDock (keeps LayerDockOutlet/PCB), SVG overlay offsets → `flex-1`; `layout_layers_tests` host +`layout-toolbar.css`, new test (42px, 7 entry-points visible/pointer-events/no-cover, sections switch singly, canvas stable), `switch_visibility` steps moved Objects footer→View (oracles unchanged); `layout-toolbar.css` View ToggleGroups 32px, CanvasLayers trigger 32px list below clamped/scrollable with `:has` overflow escape, `.layout-toolbar-accordion` `pointer-events:auto` (fixes legacy overlay `none` making triggers non-hit-testable). No `workbench_shell.rs`/`shell.css` change (existing host sufficient). GREEN: `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_layers_tests.rs web/src/presentation/workbench_shell.rs` → `.evidence/toolbar-m1/002-page-green.log` (executed 7 failed 0); `python3 scripts/check.py browser --only layout` → `.evidence/toolbar-m1/002-page-browser.log` (269 passed + 2 setup_guide passed, test-css ok). 3D preserved: pill hidden, utilities+2D/3D reachable in single ToolRow, Footprints/handles/CanvasLayers hidden (matches `!assembly_3d`), section persists across 3D toggle. Remains planned (not claimed): measured widths 320/390/728 + motion frames + reduced-motion zero-animation + guard parity (first-column/Follow-off/locked/empty) + >12 reference action parity; proof-map rows stay planned; no closure claim.
- 2026-11-11 Toggle builder (002-toggle-fix): RED was `view_mode_and_visibility_toggles_keep_pressed_state_and_revision` at `2D starts pressed (left: "false", right: "true")` (`.evidence/toolbar-m1/002-toggle-red.log`, executed 8 failed 5 with 4 cascading panic-hook follow-ons). Root: `layout_workspace::toolbar` View ToggleItems used indices 41/42 (2D/3D), 43 (Footprints), 44 (handles) while controlled `pressed` and callbacks used 0/1 and 0, so pressed never matched and mode callbacks never fired; both 2D and 3D branches affected. Fix (owned files only): indices 41->0, 42->1 (both branches), 43->0, 44->0 in `web/src/presentation/layout_workspace.rs`, matching `DesignViewToolbar` convention (0/1, 0); callbacks, pressed logic, roving, PCB, disabled guards unchanged. Test: new actual-Editor `view_mode_and_visibility_toggles_keep_pressed_state_and_revision` in `layout_layers_tests.rs` (open View, assert initial pressed 2D on/3D off/Footprints off/handles on + revision, click 3D -> 3D on/2D off/View-only hidden/one row/revision, click 2D -> restore + reopen View if remount closed section, toggle Footprints on -> independence, toggle handles off -> independence, revision unchanged throughout). Corrects overstated hit-test claim: seven-entry test proves bounds/pointer-events/no-hidden-inert/enabled + synthetic click switches sections + canvas stable, not unobscured/topmost; `elementFromPoint` topmost check attempted but reverted after three viewport attempts (button center persistently outside viewport, e.g. (500,-149), scroll_into_view + wait did not bring into viewport) - remains planned follow-up, do not imply synthetic click proves unobscured. Also corrects section-persists claim: View section does not persist across 3D remount (pill remount closes section), test reopens View; follow-up to verify intended behavior. GREEN: `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_layers_tests.rs web/src/presentation/workbench_shell.rs` -> `.evidence/toolbar-m1/002-toggle-green5.log` (executed 8 failed 0). No `workbench_shell.rs`/CSS change. Remains planned: measured widths 320/390/728 + motion + reduced-motion + guard parity + >12 reference parity + true elementFromPoint hit-test + section-persistence intent; proof-map rows stay planned; no closure claim.
