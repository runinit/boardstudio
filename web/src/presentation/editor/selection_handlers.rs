use super::super::*;

pub(super) struct SelectTreeInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub generation: u64,
    pub workspace: Signal<&'static str>,
    pub inspector_settings: Signal<PanelSettings>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub outline_activation: EventHandler<outline_lifecycle::OutlineAction>,
}

pub(super) fn select_tree_handler(
    input: SelectTreeInput,
) -> impl FnMut(objects::TreeSelectRequest) + 'static {
    let SelectTreeInput {
        runtime,
        adapter,
        owner,
        tree_cell_anchor,
        generation,
        mut workspace,
        inspector_settings,
        mut objects_open,
        mut inspect_open,
        layout_state,
        outline_activation,
    } = input;
    move |request: objects::TreeSelectRequest| {
        if request.outline_action.as_ref().is_some_and(|action| {
            !outline_lifecycle::outline_action_is_current(action, &runtime, generation)
        }) || (adapter.generation)() != generation
            || runtime.scope().as_ref() != Some(&request.scope)
            || !selection::context_is_current(&runtime.model(), &request.scope, &request.context)
        {
            return;
        }
        let (close_objects, open_inspect) = match &request.context {
            objects::TreeContext::Board { .. } => (false, false),
            objects::TreeContext::MountedModule { .. }
            | objects::TreeContext::Outline { .. }
            | objects::TreeContext::OutlineVersion { .. }
            | objects::TreeContext::Bridge { .. } => (true, true),
            objects::TreeContext::LayoutGroup { .. } => (true, false),
            objects::TreeContext::Matrix { .. }
            | objects::TreeContext::Row { .. }
            | objects::TreeContext::Column { .. }
            | objects::TreeContext::Key { .. }
            | objects::TreeContext::Component { .. } => (true, true),
        };
        let outline_route = matches!(
            &request.context,
            objects::TreeContext::Outline { .. }
                | objects::TreeContext::OutlineVersion { .. }
                | objects::TreeContext::Bridge { .. }
        );
        let bridge_id = match &request.context {
            objects::TreeContext::Bridge { bridge_id, .. } => Some(bridge_id.clone()),
            _ => None,
        };
        let activation = request.outline_action.clone();
        let scope = request.scope.clone();
        layout_state.set_selection_kind_from_context(&request.context);
        update_tree_cell_anchor(
            &tree_cell_anchor,
            &owner,
            &runtime.model(),
            Some(&request.context),
        );
        selection::submit_context(&runtime, &adapter, request.into());
        if (adapter.generation)() != generation || runtime.scope().as_ref() != Some(&scope) {
            return;
        }
        if let Some(action) = activation {
            workspace.set("Layout");
            outline_activation.call(action);
        }
        if outline_route {
            workspace.set("Layout");
            fit_selected_bridge(&runtime, bridge_id.as_deref());
            pin_inspector_on_desktop(inspector_settings);
        }
        if close_objects {
            objects_open.set(false);
        }
        if open_inspect {
            inspect_open.set(true);
        }
    }
}

pub(super) struct SelectionKindInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub selection_kind: Signal<objects::LayoutSelectionKind>,
    pub workspace: Signal<&'static str>,
    pub inspector_panel_settings: Signal<PanelSettings>,
}

pub(super) fn on_selection_kind_handler(
    input: SelectionKindInput,
    expected_workspace: &'static str,
) -> EventHandler<objects::LayoutSelectionKind> {
    let SelectionKindInput {
        runtime,
        adapter,
        owner,
        tree_cell_anchor,
        mut objects_open,
        mut inspect_open,
        mut selection_kind,
        workspace,
        inspector_panel_settings,
    } = input;
    EventHandler::new(move |kind: objects::LayoutSelectionKind| {
        if !selection::workspace_owner_is_current(
            &runtime,
            workspace,
            &adapter,
            &owner,
            expected_workspace,
        ) {
            return;
        }
        selection_kind.set(kind);
        let model = runtime.model();
        let Some(scope) = owner.scope.as_ref() else {
            tree_cell_anchor.borrow_mut().take();
            return;
        };
        let Some(selected) = adapter
            .selected_context
            .read()
            .clone()
            .filter(|selected| selected.scope == *scope)
        else {
            tree_cell_anchor.borrow_mut().take();
            return;
        };
        if !selection::context_is_current(&model, scope, &selected.context) {
            tree_cell_anchor.borrow_mut().take();
            return;
        }
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
        let Some(projection) =
            objects::context_for_selection_kind(&model, &selected.context, kind, retained.as_ref())
        else {
            return;
        };
        if !selection::context_is_current(&model, scope, &projection.context) {
            return;
        }
        update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&projection.context));
        selection::submit_context(
            &runtime,
            &adapter,
            objects::TreeSelectRequest {
                scope: scope.clone(),
                context: projection.context,
                mode: SelectionMode::Replace,
                outline_action: None,
            }
            .into(),
        );
        if selection::workspace_owner_is_current(
            &runtime,
            workspace,
            &adapter,
            &owner,
            expected_workspace,
        ) {
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    })
}

pub(super) fn on_snap_intent_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    mut snap_settings: Signal<objects::LayoutSnapSettings>,
    workspace: Signal<&'static str>,
    expected_workspace: &'static str,
) -> EventHandler<objects::LayoutSnapIntent> {
    EventHandler::new(move |intent: objects::LayoutSnapIntent| {
        if !selection::workspace_owner_is_current(
            &runtime,
            workspace,
            &adapter,
            &owner,
            expected_workspace,
        ) {
            return;
        }
        let mut settings = snap_settings.read().clone();
        match intent {
            objects::LayoutSnapIntent::Fraction(value)
                if [0.0, 0.125, 0.25, 0.5, 1.0, -1.0, -0.5, -0.1].contains(&value) =>
            {
                settings.snap_fraction = value;
            }
            objects::LayoutSnapIntent::GeometrySnap(enabled) => {
                settings.geometry_snap = enabled;
            }
            objects::LayoutSnapIntent::GapSnap(enabled) => {
                settings.gap_snap = enabled;
            }
            objects::LayoutSnapIntent::GapOverride(value) => {
                settings.gap_override = value;
            }
            objects::LayoutSnapIntent::Fraction(_) => return,
        }
        snap_settings.set(settings);
    })
}

pub(super) struct OnKeymapSelectInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub workspace: Signal<&'static str>,
    pub keymap_layer_id: Signal<String>,
}

pub(super) fn on_keymap_select_handler(input: OnKeymapSelectInput) -> EventHandler<String> {
    let OnKeymapSelectInput {
        runtime,
        adapter,
        scope,
        generation,
        mut objects_open,
        mut inspect_open,
        workspace,
        keymap_layer_id,
    } = input;
    EventHandler::new(move |key_id: String| {
        if workspace() != "Keymap"
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) {
            return;
        }
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        if key_id.is_empty() {
            let mut selected_context = adapter.selected_context;
            selected_context.set(None);
            let mut anchor_scope = adapter.anchor_scope;
            anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            return;
        }
        let layer_id = keymap_layer_id();
        let Some(view) = keymap::project(snapshot, Some(&scope), &model.active_board_id, &layer_id)
        else {
            return;
        };
        if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
            return;
        }
        let Some(context) = objects::context_for_part(&model, &key_id) else {
            return;
        };
        if !selection::context_is_current(&model, &scope, &context) {
            return;
        }
        let Some(selected_ids) = selection::submit_canvas_selection(
            &runtime,
            &adapter,
            &scope,
            generation,
            context.clone(),
            SelectionMode::Replace,
            Vec::new(),
        ) else {
            return;
        };
        if selected_ids.is_empty()
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let current = runtime.model();
        if !active_board_scope_matches(&current, &scope)
            || !selection::context_is_current(&current, &scope, &context)
        {
            return;
        }
        let Some(snapshot) = current.accepted.as_ref() else {
            return;
        };
        let layer_id = keymap_layer_id();
        let Some(view) =
            keymap::project(snapshot, Some(&scope), &current.active_board_id, &layer_id)
        else {
            return;
        };
        if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
            return;
        }
        objects_open.set(false);
        inspect_open.set(true);
    })
}

pub(super) struct OnKeycapsSelectInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub workspace: Signal<&'static str>,
    pub instance_selection: InstanceSelection,
}

pub(super) fn on_keycaps_select_handler(input: OnKeycapsSelectInput) -> EventHandler<String> {
    let OnKeycapsSelectInput {
        runtime,
        adapter,
        scope,
        generation,
        mut objects_open,
        mut inspect_open,
        workspace,
        instance_selection,
    } = input;
    EventHandler::new(move |key_id: String| {
        if workspace() != "Keycaps"
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
            return;
        }
        if key_id.is_empty() {
            let mut selected_context = adapter.selected_context;
            selected_context.set(None);
            let mut anchor_scope = adapter.anchor_scope;
            anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            return;
        }
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        let Some(view) = keycaps_scene::project(snapshot, &scope, &model.active_board_id) else {
            return;
        };
        if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
            return;
        }
        let Some(context) = objects::context_for_part(&model, &key_id) else {
            return;
        };
        if !selection::context_is_current(&model, &scope, &context) {
            return;
        }
        let Some(selected_ids) = selection::submit_canvas_selection(
            &runtime,
            &adapter,
            &scope,
            generation,
            context.clone(),
            SelectionMode::Replace,
            Vec::new(),
        ) else {
            return;
        };
        if selected_ids.is_empty()
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let current = runtime.model();
        if !active_board_scope_matches(&current, &scope)
            || !selection::context_is_current(&current, &scope, &context)
            || !instance_selection.is_current(&current)
        {
            return;
        }
        let Some(snapshot) = current.accepted.as_ref() else {
            return;
        };
        let Some(view) = keycaps_scene::project(snapshot, &scope, &current.active_board_id) else {
            return;
        };
        if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
            return;
        }
        objects_open.set(false);
        inspect_open.set(true);
    })
}

pub(super) struct OnPcbPartHitInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub token: boardstudio_application::SnapshotToken,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub workspace: Signal<&'static str>,
    pub instance_selection: InstanceSelection,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
}

pub(super) fn on_pcb_part_hit_handler(
    input: OnPcbPartHitInput,
) -> EventHandler<pcb_scene::PcbPartHit> {
    let OnPcbPartHitInput {
        runtime,
        adapter,
        scope,
        generation,
        token,
        owner,
        tree_cell_anchor,
        workspace,
        instance_selection,
        layout_selection_kind,
    } = input;
    EventHandler::new(move |request: pcb_scene::PcbPartHit| {
        if workspace() != "PCB"
            || request.scope != scope
            || request.token != token
            || request.generation != generation
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
            return;
        }
        let Some(snapshot) = model
            .accepted
            .as_ref()
            .filter(|snapshot| snapshot.token == request.token)
        else {
            return;
        };
        let Some(board) = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
        else {
            return;
        };
        if !board.part_ids.contains(&request.part_id)
            || !snapshot
                .document
                .parts
                .iter()
                .any(|part| part.id == request.part_id)
        {
            return;
        }
        let Some(hit_context) = objects::context_for_part(&model, &request.part_id) else {
            return;
        };
        if !selection::context_is_current(&model, &scope, &hit_context) {
            return;
        }
        update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&hit_context));
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
        let context = objects::context_for_selection_kind(
            &model,
            &hit_context,
            layout_selection_kind(),
            retained.as_ref(),
        )
        .map(|projection| projection.context)
        .unwrap_or(hit_context);
        let mode = if request.range {
            SelectionMode::Range
        } else if request.additive {
            SelectionMode::Toggle
        } else {
            SelectionMode::Replace
        };
        let range_ids = if mode == SelectionMode::Range {
            selection::eligible_live_ids(&model)
                .into_iter()
                .filter(|id| board.part_ids.contains(id))
                .collect()
        } else {
            Vec::new()
        };
        selection::submit_canvas_selection(
            &runtime, &adapter, &scope, generation, context, mode, range_ids,
        );
    })
}

pub(super) struct SelHandlers {
    pub on_layout_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub on_layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub on_pcb_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub on_pcb_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub on_keymap_select: EventHandler<String>,
    pub on_keycaps_select: EventHandler<String>,
    pub on_pcb_part_hit: EventHandler<pcb_scene::PcbPartHit>,
    pub on_mounted_module_select: EventHandler<String>,
    pub on_pcb_wiring_edit_board: EventHandler<()>,
    pub on_case_action: EventHandler<case_workspace::TreeAction>,
    pub on_case_display: EventHandler<case_workspace::DisplayRequest>,
    pub on_keymap_layer: EventHandler<String>,
    pub on_keymap_export: EventHandler<()>,
    pub nudge_tree: EventHandler<objects::TreeNudgeRequest>,
    pub on_open_module_placement: EventHandler<String>,
    pub on_module_attached: EventHandler<parts::AttachedModuleNavigation>,
}

pub(super) fn build_selection_handlers(
    setup_a: &super::setup_bundles::SetupA,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
    workspace_routing: WorkspaceRoutingSlots,
) -> (
    SelHandlers,
    impl FnMut(objects::TreeSelectRequest) + 'static,
) {
    let runtime = &setup_a.runtime;
    let adapter = &setup_a.adapter;
    let workspace = setup_a.workspace;
    let render_generation = setup_a.render_generation;
    let inspect_open = setup_a.inspect_open;
    let tree_cell_anchor = &setup_a.tree_cell_anchor;
    let objects_open = setup_a.objects_open;
    let instance_selection = setup_a.instance_selection;
    let layout_owner = &setup_a.layout_owner;
    let inspector_panel_settings = setup_a.inspector_panel_settings;
    let parts_selection = setup_a.parts_selection;
    let layout_state = setup_a.layout_state;
    let keymap_layer_id = setup_a.keymap_layer_id;
    let outline_activation = setup_a.outline_activation;
    let layout_selection_kind = setup_a.layout_selection_kind;
    let layout_snap_settings = setup_a.layout_snap_settings;
    let objects_panel_settings = setup_a.objects_panel_settings;
    let parts_selection_generation = setup_a.parts_selection_generation;
    let parts_state = setup_a.parts_state;
    let case_workspace_state = setup_a.case_workspace_state;
    let keymap_state = setup_a.keymap_state;
    let select_tree = select_tree_handler(SelectTreeInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        owner: layout_owner.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        generation: render_generation,
        workspace,
        inspector_settings: inspector_panel_settings,
        objects_open,
        inspect_open,
        layout_state,
        outline_activation,
    });
    let selection_kind_handler = |expected_workspace| {
        on_selection_kind_handler(
            SelectionKindInput {
                runtime: runtime.clone(),
                adapter: adapter.clone(),
                owner: layout_owner.clone(),
                tree_cell_anchor: tree_cell_anchor.clone(),
                objects_open,
                inspect_open,
                selection_kind: layout_selection_kind,
                workspace,
                inspector_panel_settings,
            },
            expected_workspace,
        )
    };
    let snap_intent_handler = |expected_workspace| {
        on_snap_intent_handler(
            runtime.clone(),
            adapter.clone(),
            layout_owner.clone(),
            layout_snap_settings,
            workspace,
            expected_workspace,
        )
    };
    let on_layout_selection_kind = selection_kind_handler("Layout");
    let on_layout_snap_intent = snap_intent_handler("Layout");
    let on_pcb_selection_kind = selection_kind_handler("PCB");
    let on_pcb_snap_intent = snap_intent_handler("PCB");
    let on_keymap_select = on_keymap_select_handler(OnKeymapSelectInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        scope: render_scope.clone(),
        generation: render_generation,
        objects_open,
        inspect_open,
        workspace,
        keymap_layer_id,
    });
    let on_keycaps_select = on_keycaps_select_handler(OnKeycapsSelectInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        scope: render_scope.clone(),
        generation: render_generation,
        objects_open,
        inspect_open,
        workspace,
        instance_selection,
    });
    let on_pcb_part_hit = on_pcb_part_hit_handler(OnPcbPartHitInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        scope: render_scope.clone(),
        generation: render_generation,
        token: snapshot.token,
        owner: layout_owner.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        workspace,
        instance_selection,
        layout_selection_kind,
    });
    let on_mounted_module_select = super::action_handlers::on_mounted_module_select_handler(
        super::action_handlers::OnMountedModuleSelectInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            token: snapshot.token,
            revision: snapshot.document.revision,
            inspect_open,
            workspace,
            instance_selection,
            parts_state,
            objects_open,
            objects_panel_settings,
        },
    );
    let on_open_module_placement = super::action_handlers::on_open_module_placement_handler(
        super::action_handlers::OnOpenModulePlacementInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            token: snapshot.token,
            revision: snapshot.document.revision,
            workspace,
            objects_open,
            inspect_open,
            instance_selection,
            parts_selection,
        },
    );
    let on_module_attached = super::action_handlers::on_module_attached_handler(
        super::action_handlers::OnModuleAttachedInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            workspace,
            objects_open,
            inspect_open,
            parts_selection_generation,
            parts_selection,
            instance_selection,
        },
    );
    let on_pcb_wiring_edit_board = super::action_handlers::on_pcb_wiring_edit_board_handler(
        runtime.clone(),
        adapter.clone(),
        render_scope.clone(),
        render_generation,
        snapshot.token,
        workspace,
        instance_selection,
    );
    let case_admission = super::action_handlers::case_admission(
        runtime.clone(),
        adapter.clone(),
        case_workspace_state,
        instance_selection,
        render_scope.clone(),
        snapshot.token,
        render_generation,
    );
    let on_case_action = super::action_handlers::on_case_action_handler(
        case_admission.clone(),
        workspace_routing.navigate,
        workspace,
        case_workspace_state,
    );
    let on_case_display = super::action_handlers::on_case_display_handler(
        case_admission,
        workspace,
        case_workspace_state,
    );
    let on_keymap_layer = super::action_handlers::on_keymap_layer_handler(
        runtime.clone(),
        adapter.clone(),
        render_scope.clone(),
        render_generation,
        workspace,
        keymap_state,
    );
    let on_keymap_export = super::action_handlers::on_keymap_export_handler(
        super::action_handlers::OnKeymapExportInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation: render_generation,
            workspace,
            instance_selection,
        },
    );
    let nudge_tree = super::action_handlers::nudge_tree_handler(
        runtime.clone(),
        adapter.clone(),
        render_generation,
        inspect_open,
    );
    (
        SelHandlers {
            on_layout_selection_kind,
            on_layout_snap_intent,
            on_pcb_selection_kind,
            on_pcb_snap_intent,
            on_keymap_select,
            on_keycaps_select,
            on_pcb_part_hit,
            on_mounted_module_select,
            on_pcb_wiring_edit_board,
            on_case_action,
            on_case_display,
            on_keymap_layer,
            on_keymap_export,
            nudge_tree,
            on_open_module_placement,
            on_module_attached,
        },
        select_tree,
    )
}
