# Inventory: existing UI building blocks and control sites

Ticket: [003](issues/003-inventory-existing-ui.md). Compiled 2026-10-21 from `web/src` and `web/crates`.

**Method.** Counts are regex counts of `name {` in Dioxus `rsx!` code, so they are approximate
(a macro-generated control would be missed). Production code only: files named `*_test(s).rs`, `tests/`
and `#[cfg(test)] mod … { }` blocks are excluded, which removed 14 test-fixture components.
"Raw" means a plain HTML element written directly, not through a `Board*` wrapper; the wrappers'
own internals in `controls.rs` (8 raw elements) are included in raw counts.

## 1. Totals

| What | Count |
| --- | --- |
| Components in production code | 210, in 88 files (14 test fixtures excluded) |
| `Board*` wrapper uses outside `controls.rs` | 397 |
| Raw `button` / `input` / `select` / `textarea` / `details` / `dialog` | 163 / 126 / 26 / 3 / 11 / 2 (331) |
| Raw `label` / `summary` / inline `svg` | 167 / 75 / 81 |
| Whole-app control totals (wrapper + raw) | buttons 357, selects 105, number inputs about 72, checkboxes about 34 |
| Legacy CSS | `web/assets/m1.css` 1,402 lines and `web/tailwind.css` 1,448 lines (622 distinct `.board-*` / `.m1-*` selectors); 80 distinct `--wb-*` variables referenced |

## 2. Shared building blocks (`web/crates/ui-shared`)

### 2.1 The `Board*` controls (`controls.rs`, 11 components)

| Component | Uses | Built on | Surface and behaviour it owns | Mapping notes |
| --- | --- | --- | --- | --- |
| `BoardButton` | 189 | native `button` | variants Primary / Secondary / Quiet / Danger; density Comfortable / Compact; `pressed` becomes `aria-pressed`; attribute passthrough. No behaviour. | rust-ui `Button` has 8 variants and 6 sizes but no compact-density concept and no `pressed`; 163 more raw buttons bypass this wrapper |
| `BoardToolbar`, `BoardToolbarButton` | 3, 3 | `dioxus-primitives` Toolbar | labelled group with arrow-key roving between buttons; `index`, `pressed` | rust-ui `Toolbar` exists (no script); keep the primitive for roving focus |
| `BoardToggle` | 2 | primitives Toggle | controlled `pressed` / `on_pressed_change` | rust-ui has Toggle Group, no single Toggle |
| `BoardPopover` | 5 | primitives Popover | controlled `open` / `on_open_change`; focus restore to trigger on close; Escape handling; `menu_kind`, `toolbar_index`, role and label props. Carries the Layout command menus. | rust-ui `Popover` is script-driven and uncontrolled (ticket 009) |
| `BoardDialog` | 3 | native `<dialog>` (`show_modal`) when modal, otherwise inline `div role=dialog` | `open` / `on_close`; `dismissible`; stops keydown propagation; handles `cancel` | rust-ui `Dialog` is script-driven; ours relies on the native dialog's focus handling |
| `BoardCheckbox` | 26 | native checkbox | controlled `checked`; label activation and form events stay native | rust-ui `Checkbox` |
| `BoardNumberField` | 20 | native `number` input | label, unit, error with `aria-describedby`; `oninput` / `onblur` / `onkeydown` pass through to the caller, which owns precise parsing | no rust-ui equivalent beyond `Input` and `Field`; precise entry must stay ours |
| `BoardSelectField` | 79 | native `select` | label, error, `aria-describedby`; native keyboard and list | rust-ui `Select` is a custom dropdown (Enter on options and the trigger label were broken in the compile spike) |
| `BoardDisclosure` | 65 | native `details` / `summary` | `open`; heading slot | rust-ui `Collapsible` or `Accordion` |
| `BoardAccordionSection` | 2 | primitives Accordion | sidebar section with keyboard accordion | rust-ui `Accordion` |

### 2.2 Other shared components

| Component | File | What it is | Notes |
| --- | --- | --- | --- |
| `ObjectsPanel`, `InspectorPanel` | `panels.rs` | side panels with compact-viewport drawer behaviour, resize, bounded scroll owner | layout shell; relevant to the structure ticket |
| `LayoutPanelSheet`, `CompactPanelControls`, `CompactPanelScrim` | `panels.rs` | narrow-viewport sheet, toggles and scrim | candidates for rust-ui `Sheet`, `Drawer` |
| `LayerToggleButton`, `LayerPanel`, `CanvasLayers` | `canvas_layers.rs` | per-layer visibility toggles with swatches, in a collapsible panel | no direct rust-ui match; candidate for an equivalence ticket |
| `LayerDockOutlet` | `layer_dock.rs` | outlet where viewers dock their layer controls | custom plumbing |
| `ThemePicker` | `project_menu.rs` | System / Light / Dark select labelled "Appearance" | extended by the Theme choice; ticket 005 |
| `ObjectOptions` | `object_options.rs` | tree grouping select | thin |
| `GeometryScriptsEditor` | `geometry_scripts.rs` | script editor with findings | app-specific |
| `FootprintGraphics`, `GraphicElement` | `footprint_graphics.rs` | SVG footprint drawing | canvas, out of scope |

## 3. Plain HTML control sites

| Element | Raw | Via wrapper | Total | Comment |
| --- | --- | --- | --- | --- |
| `button` | 163 | 189 | 352 | plus 3 `BoardToolbarButton` and 2 `BoardToggle` |
| `select` | 26 | 79 | 105 | all native today |
| `input` | 126 | n/a | | typed raw inputs: number 52, file 9, checkbox 8, text 6, search 5, color 4, range 4; the rest have no explicit type. Wrapped: number 20, checkbox 26 |
| `textarea` | 3 | none | 3 | |
| `details` / `summary` | 11 / 75 | 65 | | `summary` count includes wrapper headings |
| `dialog` | 2 | 3 | | |
| `label` | 167 | n/a | | mostly field wrappers: `label { span "Name" input {} }` |

**Where the raw controls cluster** (raw count per file): `web/src/presentation/inspector/layout_component_inspector.rs` (29); `web/crates/case/src/shared_viewer.rs` (26); `web/crates/layout/src/outline/inspector.rs` (20); `web/crates/library/src/library.rs` (19); `web/crates/case/src/mechanical_settings.rs` (17); `web/crates/parts/src/parts/module_profile_editor.rs` (16); `web/crates/parts/src/parts.rs` (15); `web/crates/parts/src/parts/assembly_editor.rs` (12).
**Where `Board*` is used most:** `web/crates/case/src/mechanical_settings.rs` (86); `web/crates/pcb/src/pcb_module_inspector.rs` (26); `web/crates/parts/src/parts/assembly_editor.rs` (15); `web/crates/keycaps/src/keycaps_settings.rs` (14).

**Recurring patterns that need one decision each**

| Pattern | Production count | Likely rust-ui target |
| --- | --- | --- |
| Status and alert messages (`role="status"` 136, `role="alert"` 110): inline `p` / `small` with a message or error | 246 | `Alert`, `Callout`, `Status`, `Field` error text |
| Section headings (`h2` / `h3`) | 104 | `Card` headers, `Item` |
| Native `title` attributes (some are props, not tooltips) | 84 | `Tooltip` (hover-only upstream; see 009) |
| Key handlers (`onkeydown`) | 73 | keep; behaviour is ours |
| Inline SVG icons | 81 | icon set (ticket 008) |
| `aria_pressed` toggle buttons | 19 | `Toggle Group`, `Toggle` |
| Pointer handlers (`onpointerdown`) | 18 | mostly canvas gestures; stay |
| Lists (`ul` / `ol`) | 17 | `Item`, `Table`, `Data Grid` where tabular |
| Tables | 1 | `Table` |
| Context menus, `kbd` | 0 | none exist today, so rust-ui `Context Menu` and `Kbd` would be additions |

## 4. Out of scope

Canvas and 3D surfaces (marked ⛔ in section 5) are out of scope: `Scene`, `PartContents`, `CaseViewer`,
`CasePreviewViewer`, `SharedViewer`, `CaseSharedViewer`, `LayoutCanonicalViewer`, `KeycapsCanvas`, `PcbScene`,
`KeymapCanvas`, `FootprintGraphics`, `GraphicElement`, the canvas overlays (`MirroredPairCanvasOverlay`,
`OutlinePointCanvasOverlay`, `OutlineDraftCanvasOverlay`, `OutlineDimension`, `OutlineCoordinate`,
`CanvasDimension`), `PartsSampleViewer`, `ControllerFootprintThumbnail`, `ModuleSourcePreview`,
`ModuleSourceFootprints`, `StackupPreview`, `SetupHardwarePreview`, `LibraryPreview`,
`FocusedFindingMarker`, `PcbFindingMarkers`, `RelationPartOutline`, `ModelAxisGrid`. Their toolbars,
inspectors and menus are in scope, including `DesignViewToolbar`.

Classifying by component name is a judgement call. Some entries (`ModelAxisGrid`, `LibraryPreview`,
`StackupPreview`) are small form or illustration widgets rather than interactive canvases; confirm in
[011](issues/011-target-workbench-structure.md).

## 5. Sweep checklist by workspace

Each file is one sweep unit. "Raw" and "Board" are per-file control counts. Check a file off when its
raw controls and `Board*` uses are replaced by the mapped components.

### `web/crates/case`: 8 files, 41 components, raw 49, Board 121

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/case/src/cad_presentation.rs` | CasePanel | 0 | 5 |
| [ ] | `web/crates/case/src/case_assembly_layers.rs` | CaseAssemblyLayers, CaseLayerButton, ComponentLayerButton, LayerEye | 1 | 3 |
| [ ] | `web/crates/case/src/case_bodies.rs` | CaseBodies, CaseNumberField | 5 | 7 |
| [ ] | `web/crates/case/src/case_collision.rs` | CaseCollisionChecks, CollisionFindingView | 0 | 6 |
| [ ] | `web/crates/case/src/case_controller.rs` | CaseBodyInspector | 0 | 0 |
| [ ] | `web/crates/case/src/case_viewer.rs` | ⛔ CaseViewer, ⛔ CasePreviewViewer | 0 | 2 |
| [ ] | `web/crates/case/src/mechanical_settings.rs` | MechanicalSettings, MechanicalLegacyConstructionControl, MechanicalPairMemberRemoval, MechanicalPairLinkControls, MechanicalPairControls, GasketClosureControls, CriticalFitControls, MountingControls, MountCollectionControls, MountRow, BatteryControls, OpeningControls, ContextualLayerInspector, ManufacturingControls, MechanicalSetupBasics, ConstructionControls, ProfileGuidance, ProfileGeometryEditor, ProfilePolygonListEditor, ProfileOpeningListEditor, DimensionControls, DimensionField, TextDraftField, ResolvedStack, Diagnostics, ⛔ StackupPreview | 17 | 86 |
| [ ] | `web/crates/case/src/shared_viewer.rs` | DesignViewToolbar, ⛔ CaseSharedViewer, ⛔ SharedViewer | 26 | 12 |

### `web/crates/keycaps`: 4 files, 8 components, raw 13, Board 16

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/keycaps/src/keycaps_finding_marker.rs` | ⛔ FocusedFindingMarker | 0 | 0 |
| [ ] | `web/crates/keycaps/src/keycaps_fit.rs` | KeycapsFitInspector, KeycapsFitFinding | 1 | 2 |
| [ ] | `web/crates/keycaps/src/keycaps_scene.rs` | ⛔ KeycapsCanvas, KeycapsKeyList | 2 | 0 |
| [ ] | `web/crates/keycaps/src/keycaps_settings.rs` | KeycapsBoardSettingsEditor, KeycapsMatrixSettingsEditor, KeycapsSettingsEditor | 10 | 14 |

### `web/crates/keymap`: 5 files, 13 components, raw 8, Board 21

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/keymap/src/keymap/binding_editor.rs` | BindingEditor, KeycodeField | 1 | 4 |
| [ ] | `web/crates/keymap/src/keymap/canvas.rs` | ⛔ KeymapCanvas | 0 | 0 |
| [ ] | `web/crates/keymap/src/keymap/encoder_editor.rs` | EncoderEditor | 0 | 3 |
| [ ] | `web/crates/keymap/src/keymap/macro_editor.rs` | MacroEditor, MacroCard, StepEditor, TextDraft, NumberDraft | 2 | 5 |
| [ ] | `web/crates/keymap/src/keymap/panel.rs` | KeymapPanel, KeymapNavigation, KeymapLayersSection, KeymapLayerControls | 5 | 9 |

### `web/crates/layout`: 18 files, 45 components, raw 54, Board 77

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/layout/src/board_inspector.rs` | BoardInspector | 1 | 0 |
| [ ] | `web/crates/layout/src/layout_findings.rs` | LayoutFindingsFooterButton, LayoutFindingsInspector, LayoutFinding | 2 | 1 |
| [ ] | `web/crates/layout/src/layout_viewer.rs` | ⛔ LayoutCanonicalViewer | 0 | 2 |
| [ ] | `web/crates/layout/src/objects.rs` | Objects, AddObjectEntry | 10 | 4 |
| [ ] | `web/crates/layout/src/objects/existing_half.rs` | ExistingHalfSetup | 1 | 4 |
| [ ] | `web/crates/layout/src/objects/keycap_size.rs` | KeySizeControls | 0 | 2 |
| [ ] | `web/crates/layout/src/objects/layout_align.rs` | LayoutAlignToolbar | 0 | 4 |
| [ ] | `web/crates/layout/src/objects/layout_canvas_controls.rs` | LayoutCanvasControls, ⛔ CanvasDimension | 0 | 0 |
| [ ] | `web/crates/layout/src/objects/layout_toolbar.rs` | LayoutCommandMenuHeader, LayoutCommandPill, LayoutSelectionToolbar, LayoutSelectionIcon, LayoutSnapDock, LayoutSelectMenu, LayoutSnapMenu, LayoutSnapOptions | 4 | 9 |
| [ ] | `web/crates/layout/src/objects/layout_transform_toolbar.rs` | LayoutTransformToolOverlay, LayoutContextActionToolbar, LayoutContextToolButton, LayoutTransformToolbar | 0 | 9 |
| [ ] | `web/crates/layout/src/objects/matrix_inspector.rs` | MatrixInspector, MatrixFieldEditor | 4 | 7 |
| [ ] | `web/crates/layout/src/objects/matrix_setup.rs` | MatrixSetup | 0 | 2 |
| [ ] | `web/crates/layout/src/objects/matrix_transform_inspector.rs` | MatrixTransformInspector, SelectionDeleteControl, NumericTransformField, MirrorTransformField, OriginModeTransformField, EnabledTransformField, KeyAssemblyField, AttachedComponentsField, ResetTransformButton | 2 | 13 |
| [ ] | `web/crates/layout/src/objects/mirrored_pair.rs` | MirroredPairForm | 6 | 2 |
| [ ] | `web/crates/layout/src/objects/mirrored_pair/canvas_overlay.rs` | ⛔ MirroredPairCanvasOverlay | 0 | 0 |
| [ ] | `web/crates/layout/src/outline/inspector.rs` | OutlineVersionInspector | 20 | 12 |
| [ ] | `web/crates/layout/src/outline/overlays.rs` | ⛔ OutlinePointCanvasOverlay, ⛔ OutlineDraftCanvasOverlay, ⛔ OutlineDimension, ⛔ OutlineCoordinate | 2 | 0 |
| [ ] | `web/crates/layout/src/setup_guide.rs` | ProjectSetupGuide, ⛔ SetupHardwarePreview | 2 | 6 |

### `web/crates/library`: 1 files, 4 components, raw 19, Board 12

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/library/src/library.rs` | KeyboardCard, ⛔ LibraryPreview, DemoActionCard, Library | 19 | 12 |

### `web/crates/parts`: 17 files, 36 components, raw 65, Board 64

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/parts/src/parts.rs` | AssemblyCreateButton, AddObjectChoice, AddObjectComponentChooser, PartsLibraryPanel, ControllerSetupPicker, PartsInspectorPanel, PartsInspectorPlacementAction, PartsPreviewWorkspace | 15 | 8 |
| [ ] | `web/crates/parts/src/parts/assembly_editor.rs` | SavedAssembliesEditor, AssemblyDraftFields, AssemblyMemberFields, AssemblyModelFields, ModelVectorFields | 12 | 15 |
| [ ] | `web/crates/parts/src/parts/component_model_editor.rs` | ComponentModelEditor, ModelVectorEditor | 1 | 3 |
| [ ] | `web/crates/parts/src/parts/details.rs` | SelectedDefinition | 0 | 0 |
| [ ] | `web/crates/parts/src/parts/generator_settings.rs` | GeneratorSettingsEditor, GeneratorParameterField | 5 | 5 |
| [ ] | `web/crates/parts/src/parts/mechanical_profile_editor.rs` | ManualProfileEditor | 1 | 11 |
| [ ] | `web/crates/parts/src/parts/mechanical_profile_ui.rs` | PartsMechanicalProfileWorkspace | 0 | 1 |
| [ ] | `web/crates/parts/src/parts/model_axis.rs` | ⛔ ModelAxisGrid, ModelAxisField | 0 | 1 |
| [ ] | `web/crates/parts/src/parts/module_profile_editor.rs` | ModuleProfileEditor | 16 | 9 |
| [ ] | `web/crates/parts/src/parts/modules_catalogue.rs` | ⛔ ModuleSourcePreview, ModuleInspector | 1 | 2 |
| [ ] | `web/crates/parts/src/parts/modules_catalogue/module_attachment.rs` | ModuleAttachment | 0 | 3 |
| [ ] | `web/crates/parts/src/parts/preview.rs` | PartsPreviewPanel, ⛔ PartsSampleViewer, ⛔ ControllerFootprintThumbnail, PartsPreviewLayers | 0 | 2 |
| [ ] | `web/crates/parts/src/parts_custom_definition.rs` | CustomDefinitionFields, PadFields | 11 | 2 |
| [ ] | `web/crates/parts/src/parts_definition_name.rs` | DefinitionNameEditor | 1 | 1 |
| [ ] | `web/crates/parts/src/parts_import_footprint.rs` | ImportKiCadFootprintAction | 1 | 1 |
| [ ] | `web/crates/parts/src/parts_new_component.rs` | NewCustomComponentAction | 1 | 0 |
| [ ] | `web/crates/parts/src/parts_view_generation.rs` | PartsViewGenerationOwner | 0 | 0 |

### `web/crates/pcb`: 9 files, 9 components, raw 28, Board 47

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/pcb/src/firmware_positions.rs` | FirmwareKeymapPanel | 1 | 1 |
| [ ] | `web/crates/pcb/src/pcb_board_reference.rs` | Editor | 8 | 4 |
| [ ] | `web/crates/pcb/src/pcb_layers.rs` | PcbLayerControls | 0 | 0 |
| [ ] | `web/crates/pcb/src/pcb_module_footprints.rs` | ⛔ ModuleSourceFootprints, ⛔ PcbFindingMarkers | 0 | 0 |
| [ ] | `web/crates/pcb/src/pcb_module_inspector.rs` | PcbMountedModuleInspector | 5 | 26 |
| [ ] | `web/crates/pcb/src/pcb_physical_setup/controller.rs` | (none) | 4 | 1 |
| [ ] | `web/crates/pcb/src/pcb_scene.rs` | ⛔ PcbScene | 0 | 0 |
| [ ] | `web/crates/pcb/src/pcb_wiring.rs` | PcbWiringInspector | 8 | 12 |
| [ ] | `web/crates/pcb/src/pcb_wiring/part_input_settings.rs` | PartInputInspector | 2 | 3 |

### `web/crates/ui-shared`: 8 files, 26 components, raw 22, Board 7

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/crates/ui-shared/src/canvas_layers.rs` | LayerToggleButton, LayerPanel, CanvasLayers | 1 | 1 |
| [ ] | `web/crates/ui-shared/src/controls.rs` | BoardToolbar, BoardToolbarButton, BoardButton, BoardCheckbox, BoardAccordionSection, BoardDisclosure, BoardDialog, BoardNumberField, BoardSelectField, BoardToggle, BoardPopover | 8 | 0 |
| [ ] | `web/crates/ui-shared/src/footprint_graphics.rs` | ⛔ FootprintGraphics, ⛔ GraphicElement | 0 | 0 |
| [ ] | `web/crates/ui-shared/src/geometry_scripts.rs` | GeometryScriptsEditor, ScriptFindings | 3 | 4 |
| [ ] | `web/crates/ui-shared/src/layer_dock.rs` | LayerDockOutlet | 0 | 0 |
| [ ] | `web/crates/ui-shared/src/object_options.rs` | ObjectOptions | 0 | 1 |
| [ ] | `web/crates/ui-shared/src/panels.rs` | CompactPanelControls, CompactPanelScrim, LayoutPanelSheet, ObjectsPanel, InspectorPanel | 10 | 0 |
| [ ] | `web/crates/ui-shared/src/project_menu.rs` | ThemePicker | 0 | 1 |

### app shell (`web/src`): 18 files, 28 components, raw 73, Board 32

| Done | File | Components | Raw | Board |
| --- | --- | --- | --- | --- |
| [ ] | `web/src/presentation.rs` | RuntimeReportBanner, LibraryLanding, ExportPanel, Editor | 7 | 5 |
| [ ] | `web/src/presentation/canvas_status_footer.rs` | (none) | 4 | 0 |
| [ ] | `web/src/presentation/case_workspace.rs` | (none) | 8 | 2 |
| [ ] | `web/src/presentation/empty_board_canvas.rs` | EmptyBoardCanvas | 0 | 2 |
| [ ] | `web/src/presentation/export_workspace.rs` | ExportWorkspace, ExportRowView | 0 | 6 |
| [ ] | `web/src/presentation/inspector.rs` | Inspector | 2 | 1 |
| [ ] | `web/src/presentation/inspector/companion_placement.rs` | CompanionPlacementInspector | 6 | 1 |
| [ ] | `web/src/presentation/inspector/layout_component_inspector.rs` | LayoutComponentInspector | 29 | 1 |
| [ ] | `web/src/presentation/keycaps_workspace.rs` | (none) | 0 | 2 |
| [ ] | `web/src/presentation/layout_canvas.rs` | ⛔ Scene, ⛔ PartContents | 0 | 0 |
| [ ] | `web/src/presentation/layout_key_selection_actions.rs` | SelectedKeysActions | 2 | 1 |
| [ ] | `web/src/presentation/layout_matrix_relations.rs` | MatrixRelationsPanel, StructureRow, DepartureRow, DepartureActionButton, ConstraintRow, ReferenceButton, RelationReference, RelationMatrixKey, ⛔ RelationPartOutline | 2 | 0 |
| [ ] | `web/src/presentation/layout_workspace.rs` | MatrixRelationshipEscape, PendingSplayOriginPickEscape | 7 | 4 |
| [ ] | `web/src/presentation/parts_workspace.rs` | (none) | 2 | 0 |
| [ ] | `web/src/presentation/pcb_workspace.rs` | PcbInspector | 0 | 3 |
| [ ] | `web/src/presentation/workspace_composition.rs` | (none) | 4 | 0 |
| [ ] | `web/src/presentation/workspace_navigation.rs` | WorkspaceObjectNavigation, InspectorActions | 0 | 3 |
| [ ] | `web/src/presentation/zmk_firmware_export.rs` | ZmkFirmwareExportRow | 0 | 1 |
