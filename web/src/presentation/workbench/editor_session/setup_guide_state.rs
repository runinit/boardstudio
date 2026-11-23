use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct GuideStateInput<'a> {
    pub active_workspace: Workspace,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub render_scope: Scope,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_generation: u64,
    pub adapter: SelectionAdapter,
    pub runtime: Rc<Runtime>,
    pub workspace: Signal<Workspace>,
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub model: &'a ReadModel,
    pub visible: Vec<&'a Part>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub part_placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub parts_query: parts::PartsQuery,
    pub guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    pub matrices: Vec<&'a boardstudio_core::model::Matrix>,
    pub mirrored_pair: objects::MirroredPairMount,
    pub resolution: pcb_wiring::PcbWiringResolution,
}

pub(super) struct GuideState<'a> {
    pub selected_bridge: Option<&'a boardstudio_core::model::OutlineBridge>,
    pub active_part_position: Option<Vec2>,
    pub guide: Option<setup_guide::SetupGuidePreferences>,
    pub has_matrix: bool,
    pub setup_guide_active: bool,
    pub setup_wiring_error: Signal<Option<String>>,
}

pub(super) fn use_guide_state(input: GuideStateInput<'_>) -> GuideState<'_> {
    let GuideStateInput {
        active_workspace,
        document,
        render_scope,
        snapshot,
        render_generation,
        adapter,
        runtime,
        workspace,
        selected_tree_context,
        model,
        visible,
        scene,
        part_placement,
        matrix_placement,
        parts_query,
        guide_preferences,
        matrices,
        mirrored_pair,
        resolution,
    } = input;
    let _board_reference_editor = if matches!(active_workspace, Workspace::Layout | Workspace::Case)
    {
        let reference = document
            .board_references
            .iter()
            .find(|reference| reference.board_id == render_scope.board_id)
            .cloned();
        let owner = LayoutOwnerIdentity {
            scope: Some(render_scope.clone()),
            token: Some(snapshot.token),
            revision: Some(snapshot.document.revision),
            generation: render_generation,
            workspace: active_workspace,
        };
        let assets = document.assets.clone();
        let editable = boardstudio_web_pcb::board_reference_owner::board_reference_owner_is_current(
            &runtime, workspace, &adapter, &owner,
        );
        Some((reference, assets, editable, owner))
    } else {
        None
    };
    let selected_bridge = selected_tree_context.as_ref().and_then(|selected| {
        let objects::TreeContext::Bridge {
            board_id,
            bridge_id,
        } = &selected.context
        else {
            return None;
        };
        snapshot
            .scene
            .board_outline_scenes
            .iter()
            .find(|scene| scene.board_id == *board_id)
            .and_then(|scene| scene.bridges.iter().find(|bridge| bridge.id == *bridge_id))
    });
    let active_part_position = model.selected_part_ids.first().and_then(|selected_id| {
        visible
            .iter()
            .find(|part| part.id == *selected_id)
            .map(|part| {
                scene
                    .transforms
                    .iter()
                    .find(|transform| transform.id == part.id)
                    .map(|transform| transform.pose.at)
                    .unwrap_or(part.pose.at)
            })
    });
    let controller_guide_hidden = part_placement.projection.is_some()
        || part_placement.busy
        || matrix_placement.placement.is_some()
        || matrix_placement.busy
        || (active_workspace == Workspace::Parts
            && parts_query().trim() == "controller"
            && guide_preferences()
                .as_ref()
                .is_some_and(|preferences| preferences.current_stage == SetupGuideStage::Wiring));
    let guide = guide_preferences().filter(|preferences| {
        preferences.project_id == document.id
            && preferences.open
            // Keep the guide visible while matrix setup overlays the Layout canvas.
            && !(active_workspace == Workspace::Layout
                && (matrix_placement.placement.is_some()
                    || matrix_placement.busy
                    || mirrored_pair.form.is_some()
                    || mirrored_pair.placement.is_some()))
            && !controller_guide_hidden
    });
    let has_matrix = !matrices.is_empty();
    let setup_guide_active = guide.is_some();
    let mut setup_wiring_error = use_signal(|| None::<String>);
    use_effect(use_reactive((&guide_preferences(), &resolution), {
        let runtime = runtime.clone();
        move |(guide, resolution)| {
            if !guide
                .as_ref()
                .is_some_and(|guide| guide.open && guide.current_stage == SetupGuideStage::Wiring)
            {
                return;
            }
            let pcb_wiring::PcbWiringResolution::Current { identity, plan } = resolution else {
                return;
            };
            let Some(accepted) = runtime.model().accepted else {
                return;
            };
            if identity.token != accepted.token
                || plan.controller_part_id.is_none()
                || plan
                    .diagnostics
                    .iter()
                    .any(|finding| finding.severity == "error")
            {
                return;
            }
            let mut proposal = (*accepted.document).clone();
            if let Err(error) = boardstudio_core::electrical::materialize(&mut proposal, &plan) {
                setup_wiring_error.set(Some(error));
                return;
            }
            setup_wiring_error.set(None);
            if proposal == *accepted.document {
                return;
            }
            runtime.submit(boardstudio_application::Event::ResolveEdit {
                operation_id: runtime.operation(),
                label: "Setup wiring".into(),
                resolver: boardstudio_application::EditResolver::new(
                    "setup-wiring",
                    move |current| {
                        if current.token != identity.token {
                            return boardstudio_application::Resolution::Retire(
                                "The board changed while wiring was being prepared.".into(),
                            );
                        }
                        boardstudio_application::Resolution::submit(
                            vec![identity.scope.board_id.clone()],
                            EditOperation::ReplaceDocument {
                                document: Box::new(proposal.clone()),
                            },
                        )
                    },
                ),
            });
        }
    }));
    GuideState {
        selected_bridge,
        active_part_position,
        guide,
        has_matrix,
        setup_guide_active,
        setup_wiring_error,
    }
}

pub(super) struct GuideOverlaysInput<'a> {
    pub guide: &'a Option<setup_guide::SetupGuidePreferences>,
    pub matrix_setup: objects::MatrixSetupMount,
    pub mechanical_settings: mechanical_settings::MechanicalSettingsState,
    pub snapshot: &'a AcceptedSnapshot,
    pub current_scope: Option<Scope>,
    pub parts_selection: parts::PartsSelection,
    pub part_placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub setup_wiring_error: Signal<Option<String>>,
    pub active_workspace: Workspace,
    pub layout_assembly_3d: Signal<bool>,
    pub visible: Vec<&'a Part>,
    pub matrices: Vec<&'a boardstudio_core::model::Matrix>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub model: &'a ReadModel,
    pub mirrored_pair: objects::MirroredPairMount,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub layout_owner: LayoutOwnerIdentity,
    pub render_scope: Scope,
    pub adapter: SelectionAdapter,
    pub accepted_canvas_source: layout_canvas::SceneSource,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub outline_inspector: Option<outline_lifecycle::OutlineInspectorProjection>,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub has_matrix: bool,
    pub guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    pub workspace: Signal<Workspace>,
    pub width: f64,
    pub zoom_surface_size: Signal<(f64, f64)>,
}

pub(super) struct GuideOverlays {
    pub inline_matrix_setup: Option<Element>,
    pub inline_case_basics: Option<Element>,
    pub inline_controller_picker: Option<Element>,
    pub show_empty_board: bool,
    pub outline_pitch: Vec2,
    pub outline_snap_origins: Rc<Vec<outline_snapping::Origin>>,
    pub outline_snap_settings: objects::LayoutSnapSettings,
    pub canvas_grid: objects::LayoutCanvasGridStyle,
    pub outline_overlay_key: Option<String>,
}

pub(super) fn use_guide_overlays(input: GuideOverlaysInput<'_>) -> GuideOverlays {
    let GuideOverlaysInput {
        guide,
        matrix_setup,
        mechanical_settings,
        snapshot,
        current_scope,
        parts_selection,
        part_placement,
        matrix_placement,
        setup_wiring_error,
        active_workspace,
        layout_assembly_3d,
        visible,
        matrices,
        scene,
        model,
        mirrored_pair,
        tree_cell_anchor,
        layout_owner,
        render_scope,
        adapter,
        accepted_canvas_source,
        layout_snap_settings,
        outline_inspector,
        document,
        has_matrix,
        guide_preferences,
        workspace,
        width,
        zoom_surface_size,
    } = input;
    let matrix_count = matrices.len();
    let placing_matrix = matrix_placement.placement.is_some() || matrix_placement.busy;
    let auto_open_matrix = use_reactive(
        (
            &guide_preferences(),
            &workspace(),
            &matrix_setup.can_open,
            &matrix_count,
            &placing_matrix,
        ),
        {
            let project_id = document.id.clone();
            let on_open = matrix_setup.on_open;
            move |(preferences, active_workspace, can_open, matrix_count, placing)| {
                if active_workspace == Workspace::Layout
                    && can_open
                    && matrix_count == 0
                    && !placing
                    && preferences.as_ref().is_some_and(|preferences| {
                        preferences.project_id == project_id
                            && preferences.open
                            && preferences.current_stage == SetupGuideStage::Layout
                    })
                {
                    on_open.call(());
                }
            }
        },
    );
    use_effect(auto_open_matrix);
    let inline_matrix_setup = if guide.is_some() {
        matrix_setup.projection.clone().map(|mut projection| {
            projection.can_cancel &= has_matrix;
            rsx! {
                objects::MatrixSetup {
                    projection,
                    on_cancel: matrix_setup.on_cancel,
                    on_create: matrix_setup.on_create,
                }
            }
        })
    } else {
        None
    };
    let inline_case_basics = guide
        .as_ref()
        .and_then(|_| mechanical_settings.props.clone())
        .map(|settings| {
            rsx! {
                boardstudio_web_case::mechanical_settings::MechanicalSetupBasics { settings }
            }
        });
    let inline_controller_picker = guide
        .as_ref()
        .filter(|preferences| {
            preferences.current_stage == SetupGuideStage::Wiring
        })
        .map(|_| {
            rsx! {
                div {
                    parts::ControllerSetupPicker {
                        snapshot: snapshot.clone(),
                        scope: current_scope.clone(),
                        selected: parts_selection,
                        on_place: part_placement.on_place_controller,
                        enabled: part_placement.controller_placement_enabled,
                        busy: part_placement.busy || matrix_placement.busy,
                        error: setup_wiring_error().or_else(|| matrix_placement.error.clone()).or_else(|| part_placement.error.clone()),
                    }
                }
            }
        });
    let show_empty_board = guide.is_none()
        && matches!(
            active_workspace,
            Workspace::Layout | Workspace::Pcb | Workspace::Keymap | Workspace::Keycaps
        )
        && !layout_assembly_3d()
        && visible.is_empty()
        && matrices.is_empty()
        && !scene.board_contours.iter().any(|contours| {
            contours.board_id == model.active_board_id && !contours.contours.is_empty()
        })
        && mirrored_pair.placement.is_none()
        && part_placement.projection.is_none()
        && !part_placement.busy;
    let show_empty_board =
        show_empty_board && matrix_placement.placement.is_none() && !matrix_placement.busy;
    let outline_pitch = {
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
        matrix_snap_parameters(model, &render_scope, &adapter, retained.as_ref())
            .0
            .unwrap_or(Vec2 { x: 19.05, y: 19.05 })
    };
    let outline_snap_origins = use_memo(use_reactive((&accepted_canvas_source,), |(source,)| {
        Rc::new(layout_canvas::snap_origins(
            &source.document,
            &source.board_id,
            &source.scene,
        ))
    }))();
    let outline_snap_settings = layout_snap_settings.read().clone();
    let outline_grid_active = outline_inspector
        .as_ref()
        .is_some_and(|projection| (projection.editing_points)());
    let canvas_grid = objects::layout_canvas_grid_style(
        width,
        zoom_surface_size().0,
        outline_pitch.x,
        outline_snap_settings.snap_fraction,
        outline_grid_active,
    );
    let outline_overlay_key = outline_inspector
        .as_ref()
        .map(outline_lifecycle::OutlineInspectorProjection::canvas_edit_key);
    GuideOverlays {
        inline_matrix_setup,
        inline_case_basics,
        inline_controller_picker,
        show_empty_board,
        outline_pitch,
        outline_snap_origins,
        outline_snap_settings,
        canvas_grid,
        outline_overlay_key,
    }
}

pub(super) fn guide_name_fields(
    guide_name_draft: Signal<Option<(String, String)>>,
    document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    on_change: Callback<String>,
    on_commit: Callback<()>,
) -> (String, Callback<String>, Callback<()>) {
    let name_value = guide_name_draft()
        .filter(|(project_id, _)| project_id == &document.id)
        .map(|(_, name)| name)
        .unwrap_or_else(|| document.name.clone());
    (name_value, on_change, on_commit)
}

pub(super) struct OnStageChangeArgs {
    pub setup_state_for_stage: setup_workspace::SetupWorkspaceState,
    pub guide_project_id: String,
    pub guide_workspace_for_stage: Signal<Workspace>,
    pub requested_workspace_panel: Signal<Option<Workspace>>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub guide_adapter: SelectionAdapter,
    pub guide_runtime: Rc<Runtime>,
}

pub(super) fn on_stage_change_handler(input: OnStageChangeArgs) -> EventHandler<SetupGuideStage> {
    let OnStageChangeArgs {
        setup_state_for_stage,
        guide_project_id,
        guide_workspace_for_stage,
        requested_workspace_panel,
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
        mut guide_adapter,
        guide_runtime,
    } = input;
    EventHandler::new(move |stage: SetupGuideStage| {
        if !setup_state_for_stage.update_guide_stage(&guide_project_id, stage) {
            return;
        }
        setup_guide::activate_stage(
            stage,
            guide_workspace_for_stage,
            requested_workspace_panel,
            objects_open,
            inspect_open,
            objects_panel_settings,
            inspector_panel_settings,
        );
        guide_adapter.selected_context.set(None);
        guide_adapter.anchor_scope.set(None);
        guide_runtime.submit(Event::SelectParts {
            operation_id: guide_runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    })
}

pub(super) struct OnOpenWorkspaceArgs {
    pub guide_workspace_for_action: Signal<Workspace>,
    pub guide_adapter_for_action: SelectionAdapter,
    pub guide_runtime_for_action: Rc<Runtime>,
    pub settings_project_id: String,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub inspector_panel_settings: Signal<PanelSettings>,
}

pub(super) fn use_on_open_workspace(
    input: OnOpenWorkspaceArgs,
) -> (Signal<bool>, EventHandler<Workspace>) {
    let OnOpenWorkspaceArgs {
        mut guide_workspace_for_action,
        mut guide_adapter_for_action,
        guide_runtime_for_action,
        settings_project_id,
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
    } = input;
    let mut full_case_settings = use_signal(|| false);
    let on_open_workspace = EventHandler::new(move |target: Workspace| {
        if target == Workspace::Case {
            full_case_settings.set(true);
            guide_workspace_for_action.set(Workspace::Case);
            return;
        }
        guide_workspace_for_action.set(target);
        setup_guide::reveal_panels(
            crate::setup_guide_state::GuideReveal::Settings,
            objects_open,
            inspect_open,
            objects_panel_settings,
            inspector_panel_settings,
        );
        let focus_runtime = guide_runtime_for_action.clone();
        let focus_project_id = settings_project_id.clone();
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            if *guide_workspace_for_action.peek() == target
                && focus_runtime
                    .model()
                    .accepted
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.document.id == focus_project_id)
            {
                setup_guide::focus_settings(target);
            }
        });
        guide_adapter_for_action.selected_context.set(None);
        guide_adapter_for_action.anchor_scope.set(None);
        guide_runtime_for_action.submit(Event::SelectParts {
            operation_id: guide_runtime_for_action.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    });
    (full_case_settings, on_open_workspace)
}

pub(super) fn on_dismiss_guide_handler(
    setup_state_for_dismiss: setup_workspace::SetupWorkspaceState,
    guide_project_id_for_dismiss: String,
) -> EventHandler<()> {
    EventHandler::new(move |_| {
        setup_state_for_dismiss.dismiss_guide(&guide_project_id_for_dismiss);
    })
}

pub(super) fn on_finish_guide_handler(
    setup_state_for_finish: setup_workspace::SetupWorkspaceState,
    guide_project_id_for_finish: String,
    mut guide_workspace_for_finish: Signal<Workspace>,
) -> EventHandler<()> {
    EventHandler::new(move |_| {
        guide_workspace_for_finish.set(Workspace::Layout);
        setup_state_for_finish.dismiss_guide(&guide_project_id_for_finish);
    })
}

pub(super) struct GuideReadinessInput<'a> {
    pub guide: &'a Option<setup_guide::SetupGuidePreferences>,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub model: &'a ReadModel,
    pub snapshot: &'a AcceptedSnapshot,
    pub resolution: &'a pcb_wiring::PcbWiringResolution,
    pub toolbar_input: Option<&'a layout_workspace::ToolbarInput>,
    pub layout_assembly_3d: Signal<bool>,
    pub visible: Vec<&'a Part>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub zoom_surface_size: Signal<(f64, f64)>,
    pub width: f64,
    pub height: f64,
    pub view_x: f64,
    pub active_workspace: Workspace,
}

pub(super) struct GuideReadiness {
    pub guide_stage_readiness: [bool; 5],
    pub guide_stage_detail: String,
    pub canvas_transform_menu: Option<objects::LayoutTransformMenuMount>,
    pub selection_geometry_anchor: Option<(f64, f64, f64)>,
}

pub(super) fn use_guide_readiness(input: GuideReadinessInput<'_>) -> GuideReadiness {
    let GuideReadinessInput {
        guide,
        document,
        model,
        snapshot,
        resolution,
        toolbar_input,
        layout_assembly_3d,
        visible,
        scene,
        svg,
        zoom_surface_size,
        width,
        height,
        view_x,
        active_workspace,
    } = input;
    let (guide_stage_readiness, guide_stage_detail) = guide
        .as_ref()
        .map(|preferences| {
            let guide_statuses = setup_guide::stage_statuses(
                &document,
                &model.active_board_id,
                &snapshot.scene,
                match resolution {
                    pcb_wiring::PcbWiringResolution::Current { identity, plan }
                        if identity.scope.document_id == document.id
                            && identity.scope.board_id == model.active_board_id
                            && identity.scope.instance_id.is_none()
                            && identity.token == snapshot.token
                            && identity.revision == document.revision =>
                    {
                        Some(plan.as_ref())
                    }
                    _ => None,
                },
            );
            let guide_stage_index = match preferences.current_stage {
                SetupGuideStage::Project => 0,
                SetupGuideStage::Layout => 1,
                SetupGuideStage::Wiring => 2,
                SetupGuideStage::Case => 3,
                SetupGuideStage::Review => 4,
            };
            let guide_stage_detail = guide_statuses[guide_stage_index].detail.clone();
            (
                guide_statuses.map(|status| status.ready),
                guide_stage_detail,
            )
        })
        .unwrap_or_default();
    let canvas_transform_menu = toolbar_input.map(|input| input.transform.clone());
    let selection_geometry_anchor = (!layout_assembly_3d()
        && active_workspace == Workspace::Layout)
        .then(|| {
            let points: Vec<_> = visible
                .iter()
                .filter(|part| model.selected_part_ids.contains(&part.id))
                .map(|part| {
                    scene
                        .transforms
                        .iter()
                        .find(|t| t.id == part.id)
                        .map_or(part.pose.at, |t| t.pose.at)
                })
                .collect();
            if points.is_empty() {
                return None;
            }
            let scale = svg.borrow().as_ref().map_or(1.0, |_surface| {
                let (surface_width, surface_height) = zoom_surface_size();
                (width / surface_width.max(1.0)).max(height / surface_height.max(1.0))
            });
            let x = (points.iter().map(|p| p.x).sum::<f64>() / points.len() as f64).clamp(
                view_x + 60.0 * scale,
                (view_x + width - 60.0 * scale).max(view_x + 60.0 * scale),
            );
            let y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min) - 12.0 - 6.0 * scale;
            Some((x, y, scale))
        })
        .flatten();
    GuideReadiness {
        guide_stage_readiness,
        guide_stage_detail,
        canvas_transform_menu,
        selection_geometry_anchor,
    }
}

pub(super) struct GuideNamed<'a> {
    pub state: GuideState<'a>,
    pub show_empty_board: bool,
    pub outline_pitch: Vec2,
    pub outline_snap_origins: Rc<Vec<outline_snapping::Origin>>,
    pub outline_snap_settings: objects::LayoutSnapSettings,
    pub canvas_grid: objects::LayoutCanvasGridStyle,
    pub outline_overlay_key: Option<String>,
    pub on_name_change: Callback<String>,
    pub on_name_commit: Callback<()>,
    pub full_case_settings: Signal<bool>,
    pub guide_stage_readiness: [bool; 5],
    pub canvas_transform_menu: Option<objects::LayoutTransformMenuMount>,
    pub selection_geometry_anchor: Option<(f64, f64, f64)>,
    pub name_value: String,
    pub inline_matrix_setup: Option<Element>,
    pub inline_case_basics: Option<Element>,
    pub inline_controller_picker: Option<Element>,
    pub guide_stage_detail: String,
    pub on_stage_change: EventHandler<SetupGuideStage>,
    pub on_open_workspace: EventHandler<Workspace>,
    pub on_dismiss_guide: EventHandler<()>,
    pub on_finish_guide: EventHandler<()>,
}

pub(super) fn use_setup_guide<'a>(
    state: &'a super::session_state::SessionState<'a>,
    mechanical_settings: &mechanical_settings::MechanicalSettingsState,
    cam: &super::camera::CamNamed,
    selected_tree_context: &Option<objects::ScopedTreeContext>,
    toolbar_input: Option<&layout_workspace::ToolbarInput>,
    (visible, matrices): (&[&'a Part], &[&'a boardstudio_core::model::Matrix]),
    matrix_setup: &objects::MatrixSetupMount,
) -> GuideNamed<'a> {
    let views = &state.views;
    let snapshot = state.snapshot;
    let render_scope = state.render_scope;
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let active_workspace = state.active_workspace;
    let document = &views.document;
    let render_generation = state.render_generation;
    let adapter = &state.adapter;
    let runtime = &editor.runtime();
    let workspace = editor.workspace;
    let model = &state.view.model;
    let scene = &views.scene;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &state.matrix_pair.matrix_placement;
    let parts_query = state.view.parts_query;
    let guide_preferences = state.setup_state.guide_preferences();
    let mirrored_pair = &state.matrix_pair.mirrored_pair;
    let guide_wiring_active = guide_preferences().as_ref().is_some_and(|guide| guide.open);
    let wiring =
        pcb_wiring::use_pcb_wiring_preview(editor.runtime(), editor.version, guide_wiring_active);
    let resolution = &wiring.resolution;
    let current_scope = &state.view.current_scope;
    let parts_selection = state.view.parts_selection;
    let layout_assembly_3d = state.layout_state.assembly_3d();
    let tree_cell_anchor = &state.tree_cell_anchor;
    let layout_owner = &state.view.layout_owner;
    let accepted_canvas_source = &cam.scene_bounds.accepted_canvas_source;
    let layout_snap_settings = state.layout_state.snap_settings();
    let outline_inspector = &state.view.outline_inspector;
    let width = cam.scene_bounds.width;
    let zoom_surface_size = cam.svg_surface.zoom_surface_size;
    let guide_name_draft = state.guide_name_edit.draft;
    let guide_name_edit = &state.guide_name_edit;
    let setup_state = state.setup_state;
    let requested_workspace_panel = state.requested_workspace_panel;
    let objects_open = editor.panels.objects_open;
    let inspect_open = editor.panels.inspector_open;
    let objects_panel_settings = state.objects_panel_settings;
    let inspector_panel_settings = state.inspector_panel_settings;
    let svg = &cam.svg_surface.svg;
    let height = cam.scene_bounds.height;
    let view_x = cam.scene_bounds.view_x;
    let GuideState {
        selected_bridge,
        active_part_position,
        guide,
        has_matrix,
        setup_guide_active,
        setup_wiring_error,
    } = use_guide_state(GuideStateInput {
        active_workspace,
        document: document.clone(),
        render_scope: render_scope.clone(),
        snapshot,
        render_generation,
        adapter: adapter.clone(),
        runtime: runtime.clone(),
        workspace,
        selected_tree_context: selected_tree_context.clone(),
        model,
        visible: visible.to_vec(),
        scene: scene.clone(),
        part_placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        parts_query,
        guide_preferences,
        matrices: matrices.to_vec(),
        mirrored_pair: mirrored_pair.clone(),
        resolution: resolution.clone(),
    });
    let GuideOverlays {
        inline_matrix_setup,
        inline_case_basics,
        inline_controller_picker,
        show_empty_board,
        outline_pitch,
        outline_snap_origins,
        outline_snap_settings,
        canvas_grid,
        outline_overlay_key,
    } = use_guide_overlays(GuideOverlaysInput {
        guide: &guide,
        matrix_setup: matrix_setup.clone(),
        mechanical_settings: mechanical_settings.clone(),
        snapshot,
        current_scope: current_scope.clone(),
        parts_selection,
        part_placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        setup_wiring_error,
        active_workspace,
        layout_assembly_3d,
        visible: visible.to_vec(),
        matrices: matrices.to_vec(),
        scene: scene.clone(),
        model,
        mirrored_pair: mirrored_pair.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        layout_owner: layout_owner.clone(),
        render_scope: render_scope.clone(),
        adapter: adapter.clone(),
        accepted_canvas_source: accepted_canvas_source.clone(),
        layout_snap_settings,
        outline_inspector: outline_inspector.clone(),
        document: document.clone(),
        has_matrix,
        guide_preferences,
        workspace,
        width,
        zoom_surface_size,
    });
    let (name_value, on_name_change, on_name_commit) = guide_name_fields(
        guide_name_draft,
        document.clone(),
        guide_name_edit.on_change,
        guide_name_edit.on_commit,
    );
    let on_stage_change = on_stage_change_handler(OnStageChangeArgs {
        setup_state_for_stage: setup_state,
        guide_project_id: document.id.clone(),
        guide_workspace_for_stage: workspace,
        requested_workspace_panel,
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
        guide_adapter: adapter.clone(),
        guide_runtime: runtime.clone(),
    });
    let (full_case_settings, on_open_workspace) = use_on_open_workspace(OnOpenWorkspaceArgs {
        guide_workspace_for_action: workspace,
        guide_adapter_for_action: adapter.clone(),
        guide_runtime_for_action: runtime.clone(),
        settings_project_id: document.id.clone(),
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
    });
    let on_dismiss_guide = on_dismiss_guide_handler(setup_state, document.id.clone());
    let on_finish_guide = on_finish_guide_handler(setup_state, document.id.clone(), workspace);
    let GuideReadiness {
        guide_stage_readiness,
        guide_stage_detail,
        canvas_transform_menu,
        selection_geometry_anchor,
    } = use_guide_readiness(GuideReadinessInput {
        guide: &guide,
        document: document.clone(),
        model,
        snapshot,
        resolution,
        toolbar_input,
        layout_assembly_3d,
        visible: visible.to_vec(),
        scene: scene.clone(),
        svg: svg.clone(),
        zoom_surface_size,
        width,
        height,
        view_x,
        active_workspace,
    });
    GuideNamed {
        state: GuideState {
            selected_bridge,
            active_part_position,
            guide,
            has_matrix,
            setup_guide_active,
            setup_wiring_error,
        },
        show_empty_board,
        outline_pitch,
        outline_snap_origins,
        outline_snap_settings,
        canvas_grid,
        outline_overlay_key,
        on_name_change,
        on_name_commit,
        full_case_settings,
        guide_stage_readiness,
        canvas_transform_menu,
        selection_geometry_anchor,
        name_value,
        inline_matrix_setup,
        inline_case_basics,
        inline_controller_picker,
        guide_stage_detail,
        on_stage_change,
        on_open_workspace,
        on_dismiss_guide,
        on_finish_guide,
    }
}
