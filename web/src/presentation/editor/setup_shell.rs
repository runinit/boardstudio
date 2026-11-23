use super::super::*;
use super::EditorContext;
use boardstudio_web_ui_model::Workspace;

pub(super) struct LayoutViewState {
    pub spacing_ghost_state: Signal<Option<spacing_ghost::SpacingGhostState>>,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub layout_assembly_3d: Signal<bool>,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub matrix_inspector: objects::MatrixInspectorMount,
    pub board_inspector: board_inspector::BoardInspectorMount,
    pub key_size: objects::KeySizeMount,
    pub matrix_splay_affect: Signal<boardstudio_core::model::MatrixSplayAffect>,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub pending_splay_origin_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub layout_align: objects::LayoutAlignMount,
    pub pcb_align: objects::LayoutAlignMount,
    pub matrix_setup: objects::MatrixSetupMount,
    pub parts_state: parts_workspace::PartsWorkspaceState,
    pub parts_assembly_orientation: Signal<parts::SwitchOrientation>,
    pub canvas_interaction: CanvasInteractionArbiter,
}

pub(super) fn use_layout_view_state(
    editor: EditorContext,
    adapter: &SelectionAdapter,
    layout_state: layout_workspace::LayoutWorkspaceState,
) -> LayoutViewState {
    let runtime = editor.runtime();
    let EditorContext {
        version, workspace, ..
    } = editor;
    // A spacing commit shows the previous accepted scene until another edit,
    // undo/redo, or selection change. The tracker only writes view state.
    let mut last_spacing_seen = use_signal(|| None::<spacing_ghost::SeenAccepted>);
    let mut spacing_ghost_state = use_signal(|| None::<spacing_ghost::SpacingGhostState>);
    let ghost_effect_runtime = runtime.clone();
    let ghost_effect_selection = adapter.selected_context;
    use_effect(move || {
        let _ = version();
        let tree_context = (ghost_effect_selection)();
        let model = ghost_effect_runtime.model();
        let Some(snapshot) = model.accepted.clone() else {
            return;
        };
        let current = spacing_ghost::AcceptedView {
            revision: snapshot.document.revision,
            transaction: snapshot.scene.transaction_id.clone(),
            document: snapshot.document.clone(),
            scene: snapshot.scene.clone(),
            selected_ids: model.selected_part_ids.clone(),
            selected_context: tree_context,
        };
        // Clone the settled values first: the tracker borrows nothing, and the
        // guarded writes below only fire on real advances.
        let seen_before = last_spacing_seen.peek().clone();
        let ghost_before = spacing_ghost_state.peek().clone();
        match spacing_ghost::advance_spacing_ghost(
            seen_before.as_ref(),
            &current,
            ghost_before.as_ref(),
        ) {
            spacing_ghost::GhostAdvance::Keep => {}
            spacing_ghost::GhostAdvance::Set { seen, ghost } => {
                if Some(&seen) != seen_before.as_ref() {
                    last_spacing_seen.set(Some(seen));
                }
                if ghost != ghost_before {
                    spacing_ghost_state.set(ghost);
                }
            }
        }
    });
    let layout_snap_settings = layout_state.snap_settings();
    let layout_command_menu = layout_state.command_menu();
    let layout_transform_tool = layout_state.transform_tool();
    let layout_assembly_3d = layout_state.assembly_3d();
    let tree_cell_anchor = use_hook(|| Rc::new(RefCell::new(None::<OwnedTreeCellAnchor>)));
    let matrix_inspector = objects::use_matrix_inspector(runtime.clone(), adapter.selected_context);
    let board_inspector = board_inspector::use_board_inspector(
        runtime.clone(),
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let key_size = objects::use_key_size(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let matrix_splay_affect = layout_state.splay_affect();
    let matrix_transform_inspector = objects::use_workspace_matrix_transform(
        runtime.clone(),
        adapter.selected_context,
        matrix_splay_affect,
    );
    let pending_splay_origin_pick = layout_state.pending_splay_origin_pick();
    let pending_anchor_pick = layout_state.pending_anchor_pick();
    let layout_align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        Workspace::Layout,
    );
    let pcb_align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        Workspace::Pcb,
    );
    let matrix_setup = objects::use_matrix_setup(
        runtime.clone(),
        version,
        adapter.selected_context,
        adapter.anchor_scope,
        workspace,
        adapter.generation,
    );
    let parts_state = parts_workspace::use_parts_workspace_state();
    let parts_assembly_orientation = parts_state.assembly_orientation();
    let canvas_interaction = use_hook(CanvasInteractionArbiter::default);
    LayoutViewState {
        spacing_ghost_state,
        layout_snap_settings,
        layout_command_menu,
        layout_transform_tool,
        layout_assembly_3d,
        tree_cell_anchor,
        matrix_inspector,
        board_inspector,
        key_size,
        matrix_splay_affect,
        matrix_transform_inspector,
        pending_splay_origin_pick,
        pending_anchor_pick,
        layout_align,
        pcb_align,
        matrix_setup,
        parts_state,
        parts_assembly_orientation,
        canvas_interaction,
    }
}

pub(super) struct MatrixPairWiringInput<'a> {
    pub editor: EditorContext,
    pub adapter: SelectionAdapter,
    pub layout_assembly_3d: Signal<bool>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub parts_assembly_orientation: Signal<parts::SwitchOrientation>,
    pub matrix_setup: &'a mut objects::MatrixSetupMount,
}

pub(super) struct MatrixPairWiring {
    pub matrix_placement: objects::MatrixPlacementMount,
    pub on_place_matrix_assembly: EventHandler<objects::MatrixPlacementSource>,
    pub pair_created_selection: Signal<Option<objects::MirroredPairCreated>>,
    pub mirrored_pair: objects::MirroredPairMount,
}

pub(super) fn use_matrix_pair_wiring(input: MatrixPairWiringInput<'_>) -> MatrixPairWiring {
    let MatrixPairWiringInput {
        editor,
        adapter,
        layout_assembly_3d,
        canvas_interaction,
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
            canvas_interaction: canvas_interaction.clone(),
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
        let canvas_interaction = canvas_interaction.clone();
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
        canvas_interaction.clone(),
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

pub(super) struct EditorContexts {
    pub outline_inspector: Option<outline_lifecycle::OutlineInspectorProjection>,
    pub outline_activation: EventHandler<outline_lifecycle::OutlineAction>,
    pub parts_query: parts::PartsQuery,
    pub parts_selection: parts::PartsSelection,
    pub parts_selection_generation: Signal<u64>,
    pub layout_target: Signal<Option<String>>,
    pub keymap_state: keymap_workspace::KeymapWorkspaceState,
    pub keymap_layer_id: Signal<String>,
    pub has_inspector: bool,
    pub model: ReadModel,
    pub current_scope: Option<Scope>,
    pub accepted_token: Option<boardstudio_application::SnapshotToken>,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub field_preview: Signal<Option<FieldPreviewState>>,
    pub resolved_document: Option<std::sync::Arc<boardstudio_core::model::ProjectDoc>>,
}

pub(super) fn use_editor_contexts(
    editor: EditorContext,
    adapter: &SelectionAdapter,
    parts_state: parts_workspace::PartsWorkspaceState,
    tree_cell_anchor: &Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    render_generation: u64,
    active_workspace: Workspace,
) -> (EditorContexts, EventHandler<()>) {
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
    let keymap_state = keymap_workspace::use_keymap_workspace_state();
    let keymap_layer_id = keymap_state.active_layer_id();
    let has_inspector = active_workspace != Workspace::Export;
    let on_parts_select = EventHandler::new({
        let CompactPanelState {
            mut objects_open,
            inspector_open: mut inspect_open,
        } = editor.panels;
        move |_| {
            objects_open.set(false);
            inspect_open.set(true);
        }
    });
    let model = runtime.model();
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
    // The Relations highlight is transient view state. A selection, workspace or board change
    // clears it; an accepted revision that drops its parts is handled by the canvas filter.
    let relation_highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| RelationHighlight(relation_highlight));
    let highlight_context = (
        adapter.selected_context.read().clone(),
        model.selected_part_ids.clone(),
        model.active_board_id.clone(),
        active_workspace,
    );
    use_effect(use_reactive((&highlight_context,), move |(_,)| {
        RelationHighlight(relation_highlight).clear();
    }));
    // Hovering or focusing an Inspector number field shows its accepted geometry.
    // Leave, blur, unmount, selection changes, and accepted revisions clear it.
    let field_preview = use_signal(|| None::<FieldPreviewState>);
    use_context_provider(|| FieldPreview(field_preview));
    let field_preview_source =
        model
            .accepted
            .as_ref()
            .zip(resolved_document.as_ref())
            .map(|(snapshot, document)| FieldPreviewSource {
                document: document.clone(),
                scene: snapshot.scene.clone(),
            });
    use_context_provider(|| field_preview_source.clone());
    let field_preview_context = (
        adapter.selected_context.read().clone(),
        model.selected_part_ids.clone(),
        model.active_board_id.clone(),
        active_workspace,
        model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
    );
    use_effect(use_reactive((&field_preview_context,), move |(_,)| {
        FieldPreview(field_preview).clear();
    }));
    (
        EditorContexts {
            outline_inspector,
            outline_activation,
            parts_query,
            parts_selection,
            parts_selection_generation,
            layout_target,
            keymap_state,
            keymap_layer_id,
            has_inspector,
            model,
            current_scope,
            accepted_token,
            layout_owner,
            layout_findings,
            field_preview,
            resolved_document,
        },
        on_parts_select,
    )
}
