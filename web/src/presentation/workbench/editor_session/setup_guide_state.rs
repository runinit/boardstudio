use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct GuideStateInput<'a> {
    pub active_workspace: Workspace,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub runtime: Rc<Runtime>,
    pub model: &'a ReadModel,
    pub visible: Vec<&'a Part>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub part_placement: part_placement::PartPlacementSession,
    pub matrix_placement: objects::MatrixPlacementSession,
    pub parts_query: parts::PartsQuery,
    pub guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    pub matrices: Vec<&'a boardstudio_core::model::Matrix>,
    pub mirrored_pair: objects::MirroredPairSession,
    pub resolution: pcb_wiring::PcbWiringResolution,
}

pub(in crate::presentation) struct GuideState {
    pub active_part_position: Option<Vec2>,
    pub guide: Option<setup_guide::SetupGuidePreferences>,
    pub has_matrix: bool,
    pub setup_guide_active: bool,
    pub setup_wiring_error: Signal<Option<String>>,
}

pub(super) fn use_guide_state(input: GuideStateInput<'_>) -> GuideState {
    let GuideStateInput {
        active_workspace,
        document,
        runtime,
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
    pub part_placement: part_placement::PartPlacementSession,
    pub matrix_placement: objects::MatrixPlacementSession,
    pub setup_wiring_error: Signal<Option<String>>,
    pub active_workspace: Workspace,
    pub layout_assembly_3d: Signal<bool>,
    pub visible: Vec<&'a Part>,
    pub matrices: Vec<&'a boardstudio_core::model::Matrix>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub model: &'a ReadModel,
    pub mirrored_pair: objects::MirroredPairSession,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub has_matrix: bool,
    pub guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    pub workspace: Signal<Workspace>,
}

pub(super) struct GuideOverlays {
    pub inline_matrix_setup: Option<Element>,
    pub inline_case_basics: Option<Element>,
    pub inline_controller_picker: Option<Element>,
    pub show_empty_board: bool,
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
        document,
        has_matrix,
        guide_preferences,
        workspace,
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
    GuideOverlays {
        inline_matrix_setup,
        inline_case_basics,
        inline_controller_picker,
        show_empty_board,
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

pub(in crate::presentation) struct GuideNamed {
    pub state: GuideState,
    pub show_empty_board: bool,
    pub on_name_change: Callback<String>,
    pub on_name_commit: Callback<()>,
    pub full_case_settings: Signal<bool>,
    pub guide_stage_readiness: [bool; 5],
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
    shared_wiring: pcb_wiring::SharedPcbWiring,
    state: &'a super::session_state::SessionState<'a>,
    mechanical_settings: &mechanical_settings::MechanicalSettingsState,
    cam: &super::camera::CamNamed,
    (visible, matrices): (&[&'a Part], &[&'a boardstudio_core::model::Matrix]),
    matrix_setup: &objects::MatrixSetupMount,
) -> GuideNamed {
    let views = &state.views;
    let snapshot = state.snapshot;
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let active_workspace = state.active_workspace;
    let document = &views.document;
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
    // Read the shared solve only while the guide is open; a disabled read
    // never subscribes this session to wiring resolution changes.
    let resolution = shared_wiring.current_resolution(&editor.runtime(), guide_wiring_active);
    let resolution = &resolution;
    let current_scope = &state.view.current_scope;
    let parts_selection = state.view.parts_selection;
    let layout_assembly_3d = state.layout_state.assembly_3d();
    let guide_name_draft = state.guide_name_edit.draft;
    let guide_name_edit = &state.guide_name_edit;
    let setup_state = state.setup_state;
    let requested_workspace_panel = state.requested_workspace_panel;
    let objects_open = editor.panels.objects_open;
    let inspect_open = editor.panels.inspector_open;
    let objects_panel_settings = state.objects_panel_settings;
    let inspector_panel_settings = state.inspector_panel_settings;
    let GuideState {
        active_part_position,
        guide,
        has_matrix,
        setup_guide_active,
        setup_wiring_error,
    } = use_guide_state(GuideStateInput {
        active_workspace,
        document: document.clone(),
        runtime: runtime.clone(),
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
        document: document.clone(),
        has_matrix,
        guide_preferences,
        workspace,
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
    let (guide_stage_readiness, guide_stage_detail) = guide
        .as_ref()
        .map(|preferences| {
            let guide_statuses = setup_guide::stage_statuses(
                document,
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
    GuideNamed {
        state: GuideState {
            active_part_position,
            guide,
            has_matrix,
            setup_guide_active,
            setup_wiring_error,
        },
        show_empty_board,
        on_name_change,
        on_name_commit,
        full_case_settings,
        guide_stage_readiness,
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
