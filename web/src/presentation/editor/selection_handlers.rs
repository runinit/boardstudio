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
        if request
            .outline_action
            .as_ref()
            .is_some_and(|action| !action.is_current(&runtime, generation))
            || (adapter.generation)() != generation
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

pub(super) struct OnLayoutSelectionKindInput {
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

pub(super) fn on_layout_selection_kind_handler(
    input: OnLayoutSelectionKindInput,
) -> impl FnMut(objects::LayoutSelectionKind) + 'static {
    let OnLayoutSelectionKindInput {
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
    move |kind: objects::LayoutSelectionKind| {
        if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
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
        if layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    }
}

pub(super) fn on_layout_snap_intent_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    mut snap_settings: Signal<objects::LayoutSnapSettings>,
    workspace: Signal<&'static str>,
) -> impl FnMut(objects::LayoutSnapIntent) + 'static {
    move |intent: objects::LayoutSnapIntent| {
        if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
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
    }
}

pub(super) struct OnPcbSelectionKindInput {
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

pub(super) fn on_pcb_selection_kind_handler(
    input: OnPcbSelectionKindInput,
) -> impl FnMut(objects::LayoutSelectionKind) + 'static {
    let OnPcbSelectionKindInput {
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
    move |kind: objects::LayoutSelectionKind| {
        if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner) {
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
        if pcb_owner_is_current(&runtime, workspace, &adapter, &owner) {
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    }
}

pub(super) fn on_pcb_snap_intent_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    mut snap_settings: Signal<objects::LayoutSnapSettings>,
    workspace: Signal<&'static str>,
) -> impl FnMut(objects::LayoutSnapIntent) + 'static {
    move |intent: objects::LayoutSnapIntent| {
        if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner) {
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
    }
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

pub(super) fn on_keymap_select_handler(
    input: OnKeymapSelectInput,
) -> impl FnMut(String) + Clone + 'static {
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
    move |key_id: String| {
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
    }
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

pub(super) fn on_keycaps_select_handler(
    input: OnKeycapsSelectInput,
) -> impl FnMut(String) + Clone + 'static {
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
    move |key_id: String| {
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
    }
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
) -> impl FnMut(pcb_scene::PcbPartHit) + 'static {
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
    move |request: pcb_scene::PcbPartHit| {
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
    }
}
