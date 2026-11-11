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
