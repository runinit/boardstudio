use super::super::*;
use super::EditorContext;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) struct SessionState<'a> {
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: &'a Scope,
    pub views: super::setup_workspaces::Views,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub setup_state: setup_workspace::SetupWorkspaceState,
    pub guide_name_edit: SetupProjectNameEdit,
    pub adapter: SelectionAdapter,
    pub observed_version: u64,
    pub case_selection: shared_viewer::CaseSelection,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub canvas_navigation: CanvasNavigationState,
    pub active_workspace: Workspace,
    pub requested_workspace_panel: Signal<Option<Workspace>>,
    pub render_generation: u64,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub matrix_inspector: objects::MatrixInspectorMount,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub parts_state: parts_workspace::PartsWorkspaceState,
    pub matrix_pair: MatrixPairWiring,
    pub view: EditorContexts,
}

pub(in crate::presentation) fn use_session_state<'a>(
    snapshot: &'a AcceptedSnapshot,
    render_scope: &'a Scope,
) -> (
    SessionState<'a>,
    WorkspaceRoutingSlots,
    objects::MatrixSetupMount,
) {
    let editor = use_context::<EditorContext>();
    boardstudio_web_ui_shared::layer_dock::use_layer_dock_provider();
    parts::use_assembly_action_provider();
    let workspace_routing = use_hook(|| WorkspaceRoutingSlots {
        select_tree: EventHandler::new(|_: objects::TreeSelectRequest| {}),
        navigate: EventHandler::new(|_: (Scope, String, Option<String>)| {}),
    });
    let runtime = editor.runtime();
    let CompactPanelState {
        objects_open,
        inspector_open: inspect_open,
    } = editor.panels;
    let preference_warning = use_context::<PreferenceStorageWarning>().0;
    let objects_panel_settings = use_panel_settings(PanelSide::Objects, preference_warning);
    let inspector_panel_settings = use_panel_settings(PanelSide::Inspector, preference_warning);
    let created_request_signal = use_context::<Signal<Option<SetupGuideRequest>>>();
    let setup_state = setup_workspace::use_setup_workspace_state(
        runtime.clone(),
        created_request_signal,
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
    );
    let guide_name_edit = use_setup_project_name(runtime.clone());
    let adapter = use_context::<SelectionAdapter>();
    let observed_version = (editor.version)();
    let case_workspace_state = case_workspace::use_case_workspace_state();
    use_context_provider(|| case_workspace_state);
    let case_selection = case_workspace_state.selection();
    let workspace = editor.workspace;
    let layout_state = layout_workspace::use_layout_workspace_state(
        adapter.selected_context,
        workspace,
        use_context::<layout_input::LayoutInput>().menu,
    );
    use_context_provider(|| layout_state);
    let interaction = use_context::<LayoutInteraction>();
    let selected_context = adapter.selected_context;
    use_effect(move || {
        let selected = selected_context.read();
        for (mode, pending) in [
            (
                CanvasMode::PickingSplayOrigin,
                layout_state.pending_splay_origin_pick(),
            ),
            (
                CanvasMode::PickingMatrixAnchor,
                layout_state.pending_anchor_pick(),
            ),
        ] {
            let stale = pending.read().as_ref().is_none_or(|owner| {
                selected.as_ref().is_none_or(|selected| {
                    selected.scope != owner.scope || selected.context != owner.context
                })
            });
            if stale && interaction.current() == Some(mode) {
                interaction.handle(CanvasEvent::ScopeChanged);
            }
        }
    });
    let canvas_navigation = use_canvas_navigation_state();
    let active_workspace = workspace();
    let requested_workspace_panel =
        panels::use_workspace_panel_defaults(active_workspace, objects_open, inspect_open);
    let render_generation = (editor.scope_generation)();
    let layout_workspace::LayoutViewState {
        tree_cell_anchor,
        matrix_inspector,
        matrix_transform_inspector,
        mut matrix_setup,
        parts_state,
        parts_assembly_orientation,
    } = layout_workspace::use_layout_view_state(editor, &adapter, layout_state);
    let matrix_pair = use_matrix_pair_wiring(MatrixPairWiringInput {
        editor,
        adapter: adapter.clone(),
        layout_assembly_3d: layout_state.assembly_3d(),
        layout_state,
        inspector_panel_settings,
        parts_assembly_orientation,
        matrix_setup: &mut matrix_setup,
    });
    let view = use_editor_contexts(
        editor,
        &adapter,
        parts_state,
        &tree_cell_anchor,
        render_generation,
        active_workspace,
    );
    let views = super::setup_workspaces::derive_views(
        &view.model,
        snapshot,
        view.resolved_document
            .clone()
            .unwrap_or_else(|| snapshot.document.clone()),
    );
    (
        SessionState {
            snapshot,
            render_scope,
            views,
            objects_panel_settings,
            inspector_panel_settings,
            setup_state,
            guide_name_edit,
            adapter,
            observed_version,
            case_selection,
            layout_state,
            canvas_navigation,
            active_workspace,
            requested_workspace_panel,
            render_generation,
            tree_cell_anchor,
            matrix_inspector,
            matrix_transform_inspector,
            parts_state,
            matrix_pair,
            view,
        },
        workspace_routing,
        matrix_setup,
    )
}

pub(in crate::presentation) fn use_case_and_project_setup(
    state: &SessionState,
) -> (
    pcb_physical_setup::PhysicalSetupMount,
    mechanical_settings::MechanicalSettingsState,
) {
    let editor = use_context::<EditorContext>();
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let version = editor.version;
    let adapter = &state.adapter;
    let instance_selection = editor.instance_selection;
    let guide_preferences = state.setup_state.guide_preferences();
    let inspect_open = editor.panels.inspector_open;
    let case_selection = state.case_selection;
    let model = &state.view.model;
    let current_scope = &state.view.current_scope;
    let physical_setup_mount = super::setup_workspaces::use_physical_setup(
        runtime.clone(),
        guide_preferences,
        version,
        adapter.clone(),
        instance_selection,
        workspace,
    );
    let on_show_mechanical_finding = super::setup_workspaces::on_show_mechanical_finding_handler(
        super::setup_workspaces::OnShowMechanicalFindingArgs {
            runtime: runtime.clone(),
            current_scope: current_scope.clone(),
            model,
            adapter_generation: adapter.generation,
            workspace,
            inspect_open,
            case_selection,
            instance_selection,
        },
    );
    let mechanical_settings = mechanical_settings::use_mechanical_settings(
        runtime.clone(),
        workspace,
        instance_selection,
        case_selection,
        on_show_mechanical_finding,
    );
    (physical_setup_mount, mechanical_settings)
}

pub(in crate::presentation) struct MatrixPairWiringInput<'a> {
    pub editor: EditorContext,
    pub adapter: SelectionAdapter,
    pub layout_assembly_3d: Signal<bool>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub parts_assembly_orientation: Signal<parts::SwitchOrientation>,
    pub matrix_setup: &'a mut objects::MatrixSetupMount,
}

pub(in crate::presentation) struct MatrixPairWiring {
    pub matrix_placement: objects::MatrixPlacementMount,
    pub on_place_matrix_assembly: EventHandler<objects::MatrixPlacementSource>,
    pub pair_created_selection: Signal<Option<objects::MirroredPairCreated>>,
    pub mirrored_pair: objects::MirroredPairMount,
}

pub(in crate::presentation) fn use_matrix_pair_wiring(
    input: MatrixPairWiringInput<'_>,
) -> MatrixPairWiring {
    let MatrixPairWiringInput {
        editor,
        adapter,
        layout_assembly_3d,
        layout_state,
        inspector_panel_settings,
        parts_assembly_orientation,
        matrix_setup,
    } = input;
    let runtime = editor.runtime();
    let EditorContext {
        version,
        workspace,
        panels,
        ..
    } = editor;
    let CompactPanelState {
        objects_open,
        inspector_open: inspect_open,
    } = panels;
    let matrix_placement = objects::use_matrix_placement(
        runtime.clone(),
        objects::MatrixPlacementInput {
            version,
            selected_context: adapter.selected_context,
            anchor_scope: adapter.anchor_scope,
            workspace,
            scope_generation: adapter.generation,
            assembly_orientation: parts::PartsAssemblyOrientation(parts_assembly_orientation),
        },
    );
    let on_place_matrix_assembly = {
        let on_place = matrix_placement.on_place;
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        let mut layout_assembly_3d = layout_assembly_3d;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let canvas_interaction = use_context::<LayoutInteraction>();
        EventHandler::new(move |source: objects::MatrixPlacementSource| {
            if workspace() != Workspace::Parts || canvas_interaction.current().is_some() {
                return;
            }
            on_place.call(source);
            workspace.set(Workspace::Layout);
            layout_assembly_3d.set(false);
            selected_context.set(None);
            anchor_scope.set(None);
            objects_open.set(false);
            inspect_open.set(false);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
        })
    };
    let pair_created_selection = layout_state.pair_created_selection();
    let on_mirrored_pair_created = use_callback({
        let runtime = runtime.clone();
        let mut adapter = adapter.clone();
        let mut pair_created_selection = pair_created_selection;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |created: objects::MirroredPairCreated| {
            let current_scope = runtime.scope();
            if workspace() != Workspace::Layout
                || (adapter.generation)() != created.owner.scope_generation
                || current_scope.as_ref() != Some(&created.owner.scope)
                || created.scope != created.owner.scope
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if snapshot.token != created.result_token
                || snapshot.document.revision != created.result_revision
                || snapshot.token == created.owner.snapshot_token
                || snapshot.document.revision <= created.owner.revision
                || snapshot.document.id != created.owner.scope.document_id
                || snapshot.session_epoch != created.owner.scope.session_epoch
                || model.active_board_id != created.owner.board_id
                || model.active_instance_id != created.owner.scope.instance_id
                || model.lifecycle != Lifecycle::Ready
                || !matches!(
                    model.durability,
                    Durability::Saved { revision } | Durability::Saving { revision }
                        if revision == created.result_revision
                )
                || !snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == created.left_matrix_id)
                || !snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == created.right_matrix_id)
                || !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == created.left_layout_id)
                || !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == created.right_layout_id)
            {
                return;
            }
            let context = objects::TreeContext::Matrix {
                matrix_id: created.left_matrix_id.clone(),
            };
            let Some(part_ids) = objects::resolve_selection(&model, &context)
                .filter(|part_ids| !part_ids.is_empty())
            else {
                return;
            };
            adapter
                .selected_context
                .set(Some(objects::ScopedTreeContext {
                    scope: created.scope.clone(),
                    context,
                }));
            adapter.anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids,
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            pair_created_selection.set(Some(created));
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    });
    let mut mirrored_pair = objects::use_mirrored_pair(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        on_mirrored_pair_created,
    );
    if mirrored_pair.owns_canvas {
        matrix_setup.can_open = false;
    }
    if matrix_setup.projection.is_some() {
        mirrored_pair.can_open = false;
    }
    MatrixPairWiring {
        matrix_placement,
        on_place_matrix_assembly,
        pair_created_selection,
        mirrored_pair,
    }
}

pub(in crate::presentation) struct EditorContexts {
    pub outline_inspector: Option<outline_lifecycle::OutlineInspectorProjection>,
    pub outline_activation: EventHandler<outline_lifecycle::OutlineAction>,
    pub parts_query: parts::PartsQuery,
    pub parts_selection: parts::PartsSelection,
    pub parts_selection_generation: Signal<u64>,
    pub layout_target: Signal<Option<String>>,
    pub has_inspector: bool,
    pub model: Rc<ReadModel>,
    pub current_scope: Option<Scope>,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub resolved_document: Option<std::sync::Arc<boardstudio_core::model::ProjectDoc>>,
}

pub(in crate::presentation) fn use_editor_contexts(
    editor: EditorContext,
    adapter: &SelectionAdapter,
    parts_state: parts_workspace::PartsWorkspaceState,
    tree_cell_anchor: &Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    render_generation: u64,
    active_workspace: Workspace,
) -> EditorContexts {
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let (outline_inspector, outline_activation) = outline_lifecycle::use_outline_lifecycle(
        runtime.clone(),
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let parts_query: PartsQuery = parts_state.query();
    let parts_selection: PartsSelection = parts_state.selected();
    let parts_selection_generation = parts_state.selection_generation();
    let layout_target: Signal<Option<String>> = use_signal(|| None);
    let has_inspector = active_workspace != Workspace::Export;
    let model = Rc::new(runtime.model());
    let resolved_doc_cache = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(
                std::sync::Arc<boardstudio_core::model::ProjectDoc>,
                std::sync::Arc<boardstudio_core::model::ProjectDoc>,
            )>,
        ))
    });
    let resolved_document = model.accepted.as_ref().map(|snapshot| {
        if let Some((raw, resolved)) = resolved_doc_cache.borrow().as_ref()
            && std::sync::Arc::ptr_eq(raw, &snapshot.document)
        {
            return resolved.clone();
        }
        let resolved = match boardstudio_application::resolved_document(&snapshot.document) {
            std::borrow::Cow::Borrowed(_) => snapshot.document.clone(),
            std::borrow::Cow::Owned(doc) => std::sync::Arc::new(doc),
        };
        *resolved_doc_cache.borrow_mut() = Some((snapshot.document.clone(), resolved.clone()));
        resolved
    });
    let current_scope = runtime.scope();
    let accepted_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
    let layout_owner = LayoutOwnerIdentity {
        scope: current_scope.clone(),
        token: accepted_token,
        revision: model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
        generation: render_generation,
        workspace: active_workspace,
    };
    let layout_findings = use_layout_findings_state(
        runtime.clone(),
        workspace,
        adapter.clone(),
        layout_owner.clone(),
    );
    use_effect(use_reactive((&layout_owner,), {
        let tree_cell_anchor = tree_cell_anchor.clone();
        move |(owner,)| {
            let stale = tree_cell_anchor
                .borrow()
                .as_ref()
                .is_some_and(|anchor| anchor.owner != owner);
            if stale {
                tree_cell_anchor.borrow_mut().take();
            }
        }
    }));
    // Provides the RelationHighlight and FieldPreview contexts; Layout reads
    // the field preview through its context inside its canvas surface.
    layout_workspace::use_layout_transient_highlights(
        adapter,
        &model,
        active_workspace,
        resolved_document.clone(),
    );
    EditorContexts {
        outline_inspector,
        outline_activation,
        parts_query,
        parts_selection,
        parts_selection_generation,
        layout_target,
        has_inspector,
        model,
        current_scope,
        layout_owner,
        layout_findings,
        resolved_document,
    }
}
