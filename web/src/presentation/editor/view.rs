use super::super::*;

pub(super) struct ObjectsPanelInput<'a, 'x> {
    pub guide_none: bool,
    pub objects_open: Signal<bool>,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub active_workspace: &'static str,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub objects_input: workspace_composition::WorkspaceObjectsInput<'x>,
    pub layout_assembly_3d: Signal<bool>,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: Scope,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
}

pub(super) fn objects_panel(input: ObjectsPanelInput<'_, '_>) -> Option<Element> {
    let ObjectsPanelInput {
        guide_none,
        objects_open,
        objects_panel_settings,
        active_workspace,
        layout_state,
        objects_input,
        layout_assembly_3d,
        snapshot,
        render_scope,
        layout_snap_settings,
        layout_command_menu,
        layout_snap_intent,
    } = input;
    if guide_none {
        Some(rsx! {
            ObjectsPanel { compact_open: objects_open, settings: objects_panel_settings,
                header_action: if active_workspace == "Layout" {
                    Some(layout_workspace::add_object_header_action(layout_state.add_object_menu_open()))
                } else { None },
                div { class: "min-h-0 min-w-0 flex-[1_1_auto] overflow-x-hidden overflow-y-auto",
                {workspace_composition::objects(objects_input)}
                }
                div { class: "relative mt-auto min-w-0 flex-none border-t border-border max-[760px]:[&_button]:min-h-control-touch [&>[data-slot=collapsible]]:overflow-visible [&_[id=m1-layers-list]]:absolute [&_[id=m1-layers-list]]:bottom-full [&_[id=m1-layers-list]]:left-0 [&_[id=m1-layers-list]]:z-30 [&_[id=m1-layers-list]]:w-full [&_[id=m1-layers-list]]:max-h-[min(65vh,520px)] [&_[id=m1-layers-list]]:overflow-auto [&_[id=m1-layers-list]]:rounded-t-xl [&_[id=m1-layers-list]]:border [&_[id=m1-layers-list]]:border-border-strong [&_[id=m1-layers-list]]:bg-card [&_[id=m1-layers-list]]:p-3",
                    if active_workspace == "Layout" && !layout_assembly_3d() {
                        CanvasLayers {
                            initially_open: false,
                            trigger_label: "Canvas visibility".to_owned(),
                            trigger_id: String::from("m1-layers-trigger"),
                            list_id: String::from("m1-layers-list"),
                            groups: canvas_layers::layout_groups(),
                        }
                    }
                    if active_workspace == "PCB" {
                        pcb_layers::PcbLayerControls { initially_open: false, snapshot: snapshot.clone(), scope: render_scope.clone() }
                    }
                    boardstudio_web_ui_shared::layer_dock::LayerDockOutlet {}
                }
                if active_workspace == "Layout" && !layout_assembly_3d() {
                    objects::LayoutSnapDock {
                        settings: layout_snap_settings.read().clone(),
                        open_menu: layout_command_menu,
                        on_snap_intent: layout_snap_intent,
                        handles_visible: layout_state.transform_handles()(),
                        on_toggle_handles: EventHandler::new(move |visible| layout_state.transform_handles().set(visible)),
                    }
                }
            }
        })
    } else {
        None
    }
}

pub(super) struct InspectorPanelInput<'a> {
    pub guide_none: bool,
    pub has_inspector: bool,
    pub inspect_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub active_workspace: &'static str,
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub findings: &'a Vec<LayoutFinding>,
    pub render_scope: Scope,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_generation: u64,
    pub on_close_layout_findings: Callback<(), ()>,
    pub on_layout_finding: Callback<layout_findings::Request, ()>,
    pub inspector_input: workspace_composition::WorkspaceInspectorInput,
}

pub(super) fn inspector_panel(input: InspectorPanelInput<'_>) -> Option<Element> {
    let InspectorPanelInput {
        guide_none,
        has_inspector,
        inspect_open,
        inspector_panel_settings,
        active_workspace,
        layout_findings,
        document,
        findings,
        render_scope,
        snapshot,
        render_generation,
        on_close_layout_findings,
        on_layout_finding,
        inspector_input,
    } = input;
    if has_inspector && guide_none {
        Some(rsx! {
            InspectorPanel { compact_open: inspect_open, settings: inspector_panel_settings,
                if active_workspace != "Layout" && (layout_findings.open)() {
                    layout_findings::LayoutFindingsInspector {
                        open: true,
                        document: Rc::new(document.as_ref().clone()),
                        findings: findings.clone(),
                        source: layout_findings::Source {
                            workspace: active_workspace,
                            scope: render_scope.clone(),
                            token: snapshot.token,
                            revision: snapshot.document.revision,
                            generation: render_generation,
                        },
                        on_close: on_close_layout_findings,
                        on_navigate: on_layout_finding,
                    }
                } else {
                    {workspace_composition::inspector(inspector_input)}
                }

            }
        })
    } else {
        None
    }
}

pub(super) struct GuidePanelInput<'a> {
    pub guide: &'a Option<setup_guide::SetupGuidePreferences>,
    pub full_case_settings: Signal<bool>,
    pub active_workspace: &'static str,
    pub mechanical_settings: &'a mechanical_settings_mount::MechanicalSettingsMount,
    pub guide_name_failure: Signal<Option<String>>,
    pub guide_stage_readiness: [bool; 5],
    pub guide_stage_detail: String,
    pub name_value: String,
    pub on_name_change: Callback<String>,
    pub on_name_commit: Callback<()>,
    pub matrix_setup_on_open: EventHandler<()>,
    pub inline_matrix_setup: Option<Element>,
    pub inline_case_basics: Option<Element>,
    pub inline_controller_picker: Option<Element>,
    pub has_matrix: bool,
    pub on_choose_controller: EventHandler<()>,
    pub physical_setup_mount: &'a pcb_physical_setup::PhysicalSetupMount,
}

pub(super) fn guide_panel(
    input: GuidePanelInput<'_>,
    on_stage_change: impl FnMut(setup_guide::SetupGuideStage) + 'static,
    on_open_workspace: impl FnMut(&'static str) + 'static,
    on_dismiss: impl FnMut(()) + 'static,
    on_finish: impl FnMut(()) + 'static,
) -> Element {
    let GuidePanelInput {
        guide,
        mut full_case_settings,
        active_workspace,
        mechanical_settings,
        guide_name_failure,
        guide_stage_readiness,
        guide_stage_detail,
        name_value,
        on_name_change,
        on_name_commit,
        matrix_setup_on_open,
        inline_matrix_setup,
        inline_case_basics,
        inline_controller_picker,
        has_matrix,
        on_choose_controller,
        physical_setup_mount,
    } = input;
    rsx! {
        if guide.is_some() {
            section { class: "absolute inset-y-0 left-0 z-24 flex w-[min(380px,calc(100vw-32px))] max-w-[calc(100%-16px)] flex-col overflow-hidden border-r border-border-strong bg-card shadow-[8px_0_24px_rgb(0_0_0/16%)] [&>[role=alert]]:mx-4 [&>[role=alert]]:mt-3 [&>[role=alert]]:text-destructive max-[760px]:z-40 max-[760px]:w-[min(380px,calc(100vw-16px))]", aria_label: "Keyboard setup",
            if full_case_settings() && active_workspace == "Case" {
                aside { class: "flex min-h-0 flex-1 flex-col bg-card p-4 [&>header]:flex [&>header]:items-center [&>header]:gap-3 [&>header]:pb-4 [&>header_h2]:m-0 [&>header_h2]:text-lg", aria_label: "Full case settings",
                    header {
                        Button { variant: ButtonVariant::Secondary,
                            class: "max-[960px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                            aria_label: "Back to setup guide",
                            onclick: move |_| full_case_settings.set(false),
                            svg { width: "20", height: "20", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", "aria-hidden": "true",
                                path { d: "M19 12H5m7-7-7 7 7 7" }
                            }
                            "Back to guide"
                        }
                        h2 { "Case settings" }
                    }
                    div { class: "min-h-0 overflow-y-auto",
                        if let Some(settings) = mechanical_settings.props.clone() {
                            boardstudio_web_case::mechanical_settings::MechanicalSettings { ..settings }
                        }
                    }
                }
            } else {
            if let Some(preferences) = guide.clone() {
                if let Some(message) = guide_name_failure() { FieldError { "{message}" } }
                setup_guide::ProjectSetupGuide {
                    stage: preferences.current_stage,
                    stage_readiness: guide_stage_readiness,
                    stage_detail: guide_stage_detail,
                    project_name: name_value,
                    on_name_change,
                    on_name_commit,
                    on_stage_change,
                    on_open_workspace,
                    on_open_matrix_setup: Some(matrix_setup_on_open),
                    matrix_setup: inline_matrix_setup,
                    case_basics: inline_case_basics,
                    controller_picker: inline_controller_picker,
                    has_matrix,
                    on_choose_controller: Some(on_choose_controller),
                    on_dismiss,
                    on_finish,
                    project_controls: if preferences.current_stage
                        == SetupGuideStage::Project
                    {
                        Some(pcb_physical_setup::controller::project_setup_controls(
                            physical_setup_mount.clone(),
                        ))
                    } else { None },
                }
            }
            }
            }
        }
    }
}

pub(super) fn canvas_focus_handler(
    objects_open: Signal<bool>,
    inspect_open: Signal<bool>,
    workspace: Signal<&'static str>,
) -> impl FnMut(FocusEvent) + Clone + 'static {
    let mut objects_open = objects_open;
    let mut inspect_open = inspect_open;
    move |_| {
        let compact = web_sys::window()
            .and_then(|window| window.match_media("(max-width: 760px)").ok().flatten())
            .is_some_and(|query| query.matches());
        if workspace() == "Case" && compact {
            if objects_open() {
                objects_open.set(false);
            }
            if inspect_open() {
                inspect_open.set(false);
            }
        }
    }
}

pub(super) struct CanvasSectionInput<'a> {
    pub setup_guide_active: bool,
    pub active_workspace: &'static str,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub workspace: Signal<&'static str>,
    pub matrix_setup: objects::MatrixSetupMount,
    pub mirrored_pair: objects::MirroredPairMount,
    pub guide_none: bool,
    pub toolbar_input: workspace_composition::WorkspaceToolbarInput,
    pub guide_project_stage: bool,
    pub hardware_split: bool,
    pub hardware_wired: bool,
    pub layout_assembly_3d: Signal<bool>,
    pub mounted_module_select: EventHandler<String>,
    pub keycaps_fit_state: keycaps_fit::KeycapsFitActions,
    pub focused_keycaps_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    pub zmk_firmware_export_panel: zmk_firmware_export::ZmkFirmwareExportPanelInput,
    pub return_workspace: Signal<&'static str>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub canvas_input: Option<workspace_composition::WorkspaceCanvasInput>,
    pub model: &'a ReadModel,
    pub snapshot: &'a AcceptedSnapshot,
    pub on_empty_board_matrix: Callback<(), ()>,
    pub on_browse_parts: EventHandler<()>,
    pub show_empty_board: bool,
    pub part_placement: &'a part_placement::PartPlacementMount,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub pending_splay_origin_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub field_preview: Signal<Option<FieldPreviewState>>,
    pub canvas_anchor_badge: &'a Option<anchor_badge::AnchorBadge>,
    pub svg: super::view_scene::LayoutSvgInput<'a>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn canvas_section(
    input: CanvasSectionInput<'_>,
    start_pan: impl FnMut(PointerEvent) + Clone + 'static,
    move_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    end_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    cancel_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    keyboard: impl FnMut(KeyboardEvent) + Clone + 'static,
    key_up: impl FnMut(KeyboardEvent) + Clone + 'static,
    wheel: impl FnMut(WheelEvent) + Clone + 'static,
) -> Element {
    let CanvasSectionInput {
        setup_guide_active,
        active_workspace,
        objects_open,
        inspect_open,
        workspace,
        matrix_setup,
        mirrored_pair,
        guide_none,
        toolbar_input,
        guide_project_stage,
        hardware_split,
        hardware_wired,
        layout_assembly_3d,
        mounted_module_select,
        keycaps_fit_state,
        focused_keycaps_finding,
        zmk_firmware_export_panel,
        return_workspace,
        inspector_panel_settings,
        canvas_input,
        model,
        snapshot,
        on_empty_board_matrix,
        on_browse_parts,
        show_empty_board,
        part_placement,
        document,
        pending_splay_origin_pick,
        pending_anchor_pick,
        field_preview,
        canvas_anchor_badge,
        svg,
    } = input;
    rsx! {
        section { class: if setup_guide_active { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border-0 bg-card shadow-sm [container:layout-canvas/inline-size] [&>[data-workspace-toolbar]]:hidden" } else if active_workspace == "Export" { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-auto rounded-xl border-0 bg-card shadow-sm" } else { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border-0 bg-card shadow-sm [container:layout-canvas/inline-size]" }, role: "region", id: "m1-workspace-panel", aria_label: "{active_workspace} workspace",
            onfocusin: canvas_focus_handler(objects_open, inspect_open, workspace),
            if active_workspace == "Layout" {
                objects::AddMatrixSurface {
                    matrix_setup,
                    mirrored_pair,
                    guide_covering_single: guide_none,
                }
            }
            {workspace_composition::toolbar(toolbar_input)}
            if guide_project_stage {
                setup_guide::SetupHardwarePreview {
                    split: hardware_split,
                    wired: hardware_wired,
                }
            } else if matches!(active_workspace, "Layout" | "Keymap" | "Keycaps")
                && layout_assembly_3d()
            {
                layout_viewer::LayoutCanonicalViewer {
                    on_mounted_module_pick: (active_workspace == "Layout")
                        .then_some(mounted_module_select),
                    keycaps_fit: if matches!(active_workspace, "Keymap" | "Keycaps") {
                        keycaps_fit_state.state.clone()
                    } else {
                        None
                    },
                    focused_finding: if active_workspace == "Layout" {
                        focused_keycaps_finding()
                    } else {
                        None
                    },
                }
            } else if active_workspace == "Layout" {
                {super::view_scene::layout_svg(svg, start_pan, move_pointer, end_pointer, cancel_pointer, keyboard, key_up, wheel)}
            } else if active_workspace == "Export" {
                export_workspace::ExportWorkspace {
                    zmk_firmware: Some(zmk_firmware_export_panel),
                    workspace,
                    return_workspace,
                    inspect_open,
                    inspector_settings: inspector_panel_settings,
                }
            } else if let Some(input) = canvas_input {
                {workspace_composition::canvas(input)}
            } else {
                span {}
            }
            if show_empty_board {
                empty_board_canvas::EmptyBoardCanvas {
                    editable: model.lifecycle == Lifecycle::Ready
                        && model.durability == (Durability::Saved { revision: snapshot.document.revision })
                        && model.gesture.is_none()
                        && model.display_preview.is_none(),
                    on_matrix: on_empty_board_matrix,
                    on_parts: on_browse_parts,
                }
            }
            if active_workspace == "Layout"
                && let Some(placement) = part_placement.projection.as_ref()
            {
                div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded-sm border border-border bg-card px-[9px] py-[7px] text-[10px] text-muted-foreground shadow-[0_4px_14px_#24373312]", role: "status",
                    {format!("Place {}", parts::placement_label(&placement.pending.definition))}
                    if let Some(layout_name) = placement.owner.layout_id.as_ref()
                        .and_then(|id| document.layouts.iter().find(|layout| &layout.id == id))
                        .map(|layout| layout.name.as_str())
                    {
                        " in {layout_name}"
                    }
                    " · Click or Enter to place · Esc cancels"
                }
            }
            if active_workspace == "Layout" && pending_splay_origin_pick().is_some() {
                div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded-sm border border-border bg-card px-[9px] py-[7px] text-[10px] text-muted-foreground shadow-[0_4px_14px_#24373312]", role: "status",
                    "Pick splay origin · Click the canvas · Esc cancels"
                }
            }
            if active_workspace == "Layout" && pending_anchor_pick().is_some() {
                div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded border border-border bg-card px-2 py-1.5 text-[10px] text-muted-foreground shadow-sm", role: "status",
                    "Pick anchor key · Click a key · Esc cancels"
                }
            }
            // Ticket 16: the at-rest field preview names the keys the
            // hovered or focused Inspector number field would move.
            if active_workspace == "Layout"
                && let Some(state) = field_preview() {
                div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded border border-border bg-card px-2 py-1.5 text-[10px] font-semibold text-warning shadow-sm", role: "status", "data-field-preview-hint": "",
                    "{state.hint}"
                }
            }
            // Ticket 02: the "Placed from" readout (prototype variant
            // D) names what the selection is placed from at the
            // canvas's bottom left. It shares the canvas ring's badge
            // projection and clears with the selection.
            if active_workspace == "Layout"
                && !layout_assembly_3d()
                && let Some(badge) = canvas_anchor_badge.as_ref()
            {
                div { class: "pointer-events-none absolute bottom-[62px] left-3 flex items-center gap-1.5 rounded border border-border bg-card px-2 py-1.5 font-mono text-[11px] text-geometry-part shadow-sm", "data-canvas-anchor-readout": "",
                    "Placed from {badge.label}"
                }
            }
        }
    }
}

pub(super) struct FooterInput<'a> {
    pub active_workspace: &'static str,
    pub layout_assembly_3d: Signal<bool>,
    pub undo: Rc<Runtime>,
    pub redo: Rc<Runtime>,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub active_part_position: Option<Vec2>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub footer_selection_available: bool,
    pub zoom_percent: f64,
    pub findings: &'a Vec<LayoutFinding>,
    pub snapshot: &'a AcceptedSnapshot,
    pub on_toggle_layout_findings: Callback<(), ()>,
    pub on_fit_keymap_board: EventHandler<()>,
    pub on_zoom_keymap_out: EventHandler<()>,
    pub on_zoom_keymap_in: EventHandler<()>,
    pub model: &'a ReadModel,
}

pub(super) fn view_footer(
    input: FooterInput<'_>,
    on_fit_selection: impl FnMut(()) + 'static,
) -> Element {
    let FooterInput {
        active_workspace,
        layout_assembly_3d,
        undo,
        redo,
        layout_snap_settings,
        active_part_position,
        layout_command_menu,
        footer_selection_available,
        zoom_percent,
        findings,
        snapshot,
        on_toggle_layout_findings,
        on_fit_keymap_board,
        on_zoom_keymap_out,
        on_zoom_keymap_in,
        model,
    } = input;
    rsx! {
        footer { class: "shell-footer", hidden: active_workspace == "Export", aria_label: "Canvas status",
            boardstudio_web_ui_shared::ui::toolbar::Toolbar { class: "shell-history", aria_label: "Footer edit history",
                boardstudio_web_ui_shared::ui::toolbar::ToolbarButton { index: 0usize, aria_label: "Undo", on_click: move |_| undo.submit(Event::Undo { operation_id: undo.operation() }), "Undo" }
                boardstudio_web_ui_shared::ui::toolbar::ToolbarButton { index: 1usize, aria_label: "Redo", on_click: move |_| redo.submit(Event::Redo { operation_id: redo.operation() }), "Redo" }
            }
            if matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps") && !layout_assembly_3d() {
                canvas_status_footer::CanvasStatusFooter {
                    snap_settings: layout_snap_settings.read().clone(),
                    active_part_position,
                    snap_menu_available: matches!(active_workspace, "Layout" | "PCB"),
                    open_menu: layout_command_menu,
                    board_available: true,
                    selection_available: footer_selection_available,
                    zoom_percent,
                    findings_count: Some(keycaps_fit::presented_findings(
                        findings,
                        &snapshot.document,
                    ).len()),
                    on_toggle_findings: on_toggle_layout_findings,
                    on_fit_board: on_fit_keymap_board,
                    on_fit_selection,
                    on_zoom_out: on_zoom_keymap_out,
                    on_zoom_in: on_zoom_keymap_in,
                }
                if let Some(guide) = model.snap_guide.as_ref() {
                    span { class: "ml-auto max-w-[min(35vw,340px)] truncate", role: "status", "{guide.label}" }
                }
            } else {
                span { "{model.camera.zoom * 100.0:.0}%" }
            }
            if active_workspace != "Export"
                && !(matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps")
                    && !layout_assembly_3d())
            {
                canvas_status_footer::FindingsButton {
                    count: keycaps_fit::presented_findings(
                        findings,
                        &snapshot.document,
                    ).len(),
                    on_toggle: on_toggle_layout_findings,
                }
            }
        }
    }
}
