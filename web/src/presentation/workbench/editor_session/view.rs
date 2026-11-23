use super::super::*;
use boardstudio_web_ui_model::Workspace;
use workspace_composition::{Surface, WorkspaceOutlet};

pub(super) struct ObjectsPanelInput {
    pub guide_none: bool,
    pub objects_open: Signal<bool>,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub active_workspace: Workspace,
    pub export_objects: Option<Element>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub layout_assembly_3d: Signal<bool>,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
}

pub(super) fn objects_panel(input: ObjectsPanelInput) -> Option<Element> {
    let ObjectsPanelInput {
        guide_none,
        objects_open,
        objects_panel_settings,
        active_workspace,
        export_objects,
        layout_state,
        layout_assembly_3d,
        layout_snap_settings,
        layout_command_menu,
        layout_snap_intent,
    } = input;
    if guide_none {
        Some(rsx! {
            ObjectsPanel { compact_open: objects_open, settings: objects_panel_settings,
                header_action: if active_workspace == Workspace::Layout {
                    Some(layout_workspace::add_object_header_action(layout_state.add_object_menu_open()))
                } else { None },
                div { class: "min-h-0 min-w-0 flex-[1_1_auto] overflow-x-hidden overflow-y-auto",
                if active_workspace == Workspace::Export {
                    {export_objects}
                } else {
                    WorkspaceOutlet { workspace: active_workspace, surface: Surface::Objects }
                }
                }
                div { class: "relative mt-auto min-w-0 flex-none border-t border-border max-[760px]:[&_button]:min-h-control-touch [&>[data-slot=collapsible]]:overflow-visible [&_[id=m1-layers-list]]:absolute [&_[id=m1-layers-list]]:bottom-full [&_[id=m1-layers-list]]:left-0 [&_[id=m1-layers-list]]:z-30 [&_[id=m1-layers-list]]:w-full [&_[id=m1-layers-list]]:max-h-[min(65vh,520px)] [&_[id=m1-layers-list]]:overflow-auto [&_[id=m1-layers-list]]:rounded-t-xl [&_[id=m1-layers-list]]:border [&_[id=m1-layers-list]]:border-border-strong [&_[id=m1-layers-list]]:bg-card [&_[id=m1-layers-list]]:p-3",
                    if active_workspace == Workspace::Layout && !layout_assembly_3d() {
                        {layout_workspace::canvas_layers()}
                    }
                    if active_workspace == Workspace::Pcb {
                        WorkspaceOutlet { workspace: active_workspace, surface: Surface::Layers }
                    }
                    boardstudio_web_ui_shared::layer_dock::LayerDockOutlet {}
                }
                if active_workspace == Workspace::Layout && !layout_assembly_3d() {
                    {layout_workspace::snap_dock(layout_state, layout_snap_settings, layout_command_menu, layout_snap_intent)}
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
    pub active_workspace: Workspace,
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub findings: &'a Vec<LayoutFinding>,
    pub render_scope: Scope,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_generation: u64,
    pub on_close_layout_findings: Callback<(), ()>,
    pub on_layout_finding: Callback<layout_findings::Request, ()>,
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
    } = input;
    if has_inspector && guide_none {
        Some(rsx! {
            InspectorPanel { compact_open: inspect_open, settings: inspector_panel_settings,
                if active_workspace != Workspace::Layout && (layout_findings.open)() {
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
                    WorkspaceOutlet { workspace: active_workspace, surface: Surface::Inspector }
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
    pub active_workspace: Workspace,
    pub mechanical_settings: &'a mechanical_settings::MechanicalSettingsState,
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
    on_stage_change: EventHandler<setup_guide::SetupGuideStage>,
    on_open_workspace: EventHandler<Workspace>,
    on_dismiss: EventHandler<()>,
    on_finish: EventHandler<()>,
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
            if full_case_settings() && active_workspace == Workspace::Case {
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
    workspace: Signal<Workspace>,
) -> impl FnMut(FocusEvent) + Clone + 'static {
    let mut objects_open = objects_open;
    let mut inspect_open = inspect_open;
    move |_| {
        let compact = web_sys::window()
            .and_then(|window| window.match_media("(max-width: 760px)").ok().flatten())
            .is_some_and(|query| query.matches());
        if workspace() == Workspace::Case && compact {
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
    pub active_workspace: Workspace,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub workspace: Signal<Workspace>,
    pub matrix_setup: objects::MatrixSetupMount,
    pub mirrored_pair: objects::MirroredPairMount,
    pub guide_none: bool,
    pub guide_project_stage: bool,
    pub hardware_split: bool,
    pub hardware_wired: bool,
    pub layout_assembly_3d: Signal<bool>,
    pub return_workspace: Signal<Workspace>,
    pub inspector_panel_settings: Signal<PanelSettings>,
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
}

pub(super) fn canvas_section(input: CanvasSectionInput<'_>) -> Element {
    let CanvasSectionInput {
        setup_guide_active,
        active_workspace,
        objects_open,
        inspect_open,
        workspace,
        matrix_setup,
        mirrored_pair,
        guide_none,
        guide_project_stage,
        hardware_split,
        hardware_wired,
        layout_assembly_3d,
        return_workspace,
        inspector_panel_settings,
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
    } = input;
    rsx! {
        section { class: if setup_guide_active { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border-0 bg-card shadow-sm [container:layout-canvas/inline-size] [&>[data-workspace-toolbar]]:hidden" } else if active_workspace == Workspace::Export { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-auto rounded-xl border-0 bg-card shadow-sm" } else { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border-0 bg-card shadow-sm [container:layout-canvas/inline-size]" }, role: "region", id: "m1-workspace-panel", aria_label: "{active_workspace} workspace",
            onfocusin: canvas_focus_handler(objects_open, inspect_open, workspace),
            if active_workspace == Workspace::Layout {
                {layout_workspace::add_matrix_surface(matrix_setup, mirrored_pair, !guide_none)}
            }
            if active_workspace != Workspace::Export {
                WorkspaceOutlet { workspace: active_workspace, surface: Surface::Toolbar }
            }
            if guide_project_stage {
                setup_guide::SetupHardwarePreview {
                    split: hardware_split,
                    wired: hardware_wired,
                }
            } else if active_workspace != Workspace::Export {
                WorkspaceOutlet { workspace: active_workspace, surface: Surface::Canvas }
            } else if active_workspace == Workspace::Export {
                zmk_firmware_export::ExportWithWiring {
                    workspace,
                    return_workspace,
                    inspect_open,
                    inspector_settings: inspector_panel_settings,
                }
            }
            if show_empty_board {
                empty_board_canvas::EmptyBoardCanvas {
                    revision: snapshot.document.revision,
                    on_matrix: on_empty_board_matrix,
                    on_parts: on_browse_parts,
                }
            }
            {layout_workspace::canvas_status_pills(layout_workspace::CanvasStatusPillsInput {
                active: active_workspace == Workspace::Layout,
                part_placement,
                document,
                pending_splay_origin_pick,
                pending_anchor_pick,
                field_preview,
                layout_assembly_3d,
                canvas_anchor_badge,
            })}
        }
    }
}

pub(super) fn view_footer(
    workspace: Workspace,
    layout_assembly_3d: Signal<bool>,
    runtime: Rc<Runtime>,
    status: &workspace_composition::StatusFooter,
    model: &ReadModel,
    cam: &super::camera::CamNamed,
) -> Element {
    let undo = runtime.clone();
    let redo = runtime;
    rsx! {
        footer { class: "shell-footer", hidden: workspace == Workspace::Export, aria_label: "Canvas status",
            boardstudio_web_ui_shared::ui::toolbar::Toolbar { class: "shell-history", aria_label: "Footer edit history",
                boardstudio_web_ui_shared::ui::toolbar::ToolbarButton { index: 0usize, aria_label: "Undo", on_click: move |_| undo.submit(Event::Undo { operation_id: undo.operation() }), "Undo" }
                boardstudio_web_ui_shared::ui::toolbar::ToolbarButton { index: 1usize, aria_label: "Redo", on_click: move |_| redo.submit(Event::Redo { operation_id: redo.operation() }), "Redo" }
            }
            if workspace == Workspace::Export {
                span { "{model.camera.zoom * 100.0:.0}%" }
            } else if matches!(workspace, Workspace::Keymap | Workspace::Keycaps) {
                WorkspaceOutlet { workspace, surface: Surface::Status }
            } else if matches!(workspace, Workspace::Layout | Workspace::Pcb) && !layout_assembly_3d() {
                {status.controls(cam.footer_bounds.footer_selection_available, cam.scene_bounds.zoom_percent, model, cam.actions)}
            } else {
                {status.summary(model.camera.zoom)}
            }
        }
    }
}
