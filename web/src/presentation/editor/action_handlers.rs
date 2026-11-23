use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct OnMountedModuleSelectInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub inspect_open: Signal<bool>,
    pub workspace: Signal<Workspace>,
    pub instance_selection: InstanceSelection,
    pub parts_state: parts_workspace::PartsWorkspaceState,
    pub objects_open: Signal<bool>,
    pub objects_panel_settings: Signal<PanelSettings>,
}

pub(super) fn on_mounted_module_select_handler(
    input: OnMountedModuleSelectInput,
) -> EventHandler<String> {
    let OnMountedModuleSelectInput {
        runtime,
        mut adapter,
        scope,
        generation,
        token,
        revision,
        mut inspect_open,
        workspace,
        instance_selection,
        parts_state,
        objects_open,
        objects_panel_settings,
    } = input;
    EventHandler::new(move |module_id: String| {
        if !matches!(workspace(), Workspace::Pcb | Workspace::Layout)
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
            return;
        }
        let Some(accepted) = model
            .accepted
            .as_ref()
            .filter(|snapshot| snapshot.token == token && snapshot.document.revision == revision)
        else {
            return;
        };
        let Some(module) = accepted
            .document
            .modules
            .iter()
            .find(|module| module.id == module_id && module.host_board_id == scope.board_id)
        else {
            return;
        };
        if !accepted
            .document
            .module_definitions
            .iter()
            .any(|definition| definition.id == module.definition_id)
        {
            return;
        }
        let module_definition_id = module.definition_id.clone();
        let context = objects::TreeContext::MountedModule {
            board_id: scope.board_id.clone(),
            module_id,
        };
        if !selection::context_is_current(&model, &scope, &context) {
            return;
        }
        adapter
            .selected_context
            .set(Some(objects::ScopedTreeContext {
                scope: scope.clone(),
                context,
            }));
        adapter.anchor_scope.set(None);
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
        parts_state.select_mounted_module(scope.clone(), &module_definition_id);
        browse_parts_workspace(
            workspace,
            objects_open,
            objects_panel_settings,
            is_compact_viewport(),
        );
        inspect_open.set(true);
    })
}

pub(super) struct OnOpenModulePlacementInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub workspace: Signal<Workspace>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub instance_selection: InstanceSelection,
    pub parts_selection: parts::PartsSelection,
}

pub(super) fn on_open_module_placement_handler(
    input: OnOpenModulePlacementInput,
) -> EventHandler<String> {
    let OnOpenModulePlacementInput {
        runtime,
        adapter,
        scope,
        generation,
        token,
        revision,
        mut workspace,
        mut objects_open,
        mut inspect_open,
        instance_selection,
        parts_selection,
    } = input;
    EventHandler::new(move |module_id: String| {
        if workspace() != Workspace::Parts
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
            return;
        }
        let current_context = adapter.selected_context.read().clone();
        let Some(current_context) = current_context.filter(|selected| {
            selected.scope == scope
                && matches!(
                    &selected.context,
                    objects::TreeContext::MountedModule { board_id, module_id: selected_id }
                        if board_id == &scope.board_id && selected_id == &module_id
                )
        }) else {
            return;
        };
        if !selection::context_is_current(&model, &scope, &current_context.context) {
            return;
        }
        let Some(accepted) = model
            .accepted
            .as_ref()
            .filter(|snapshot| snapshot.token == token && snapshot.document.revision == revision)
        else {
            return;
        };
        let selected_definition = parts_selection().and_then(|(selected_scope, id)| {
            (selected_scope == Some(scope.clone()))
                .then(|| id.strip_prefix("module:").map(str::to_owned))
                .flatten()
        });
        if !accepted.document.modules.iter().any(|module| {
            module.id == module_id
                && module.host_board_id == scope.board_id
                && Some(module.definition_id.clone()) == selected_definition
        }) {
            return;
        }
        workspace.set(Workspace::Pcb);
        objects_open.set(false);
        inspect_open.set(true);
    })
}

pub(super) struct OnModuleAttachedInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub workspace: Signal<Workspace>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub parts_selection_generation: Signal<u64>,
    pub parts_selection: parts::PartsSelection,
    pub instance_selection: InstanceSelection,
}

pub(super) fn on_module_attached_handler(
    input: OnModuleAttachedInput,
) -> EventHandler<parts::AttachedModuleNavigation> {
    let OnModuleAttachedInput {
        runtime,
        mut adapter,
        mut workspace,
        mut objects_open,
        mut inspect_open,
        parts_selection_generation,
        parts_selection,
        instance_selection,
    } = input;
    EventHandler::new(move |request: parts::AttachedModuleNavigation| {
        if workspace() != Workspace::Parts
            || runtime.scope().as_ref() != Some(&request.scope)
            || parts_selection_generation() != request.selection_generation
            || parts_selection()
                != Some((
                    Some(request.scope.clone()),
                    format!("module:{}", request.definition_id),
                ))
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &request.scope)
            || !instance_selection.is_current(&model)
        {
            return;
        }
        let Some(accepted) = model.accepted.as_ref().filter(|snapshot| {
            snapshot.token == request.snapshot_token
                && snapshot.document.revision == request.revision
        }) else {
            return;
        };
        if !accepted.document.modules.iter().any(|module| {
            module.id == request.module_id
                && module.definition_id == request.definition_id
                && module.host_board_id == request.scope.board_id
        }) {
            return;
        }
        let context = objects::TreeContext::MountedModule {
            board_id: request.scope.board_id.clone(),
            module_id: request.module_id,
        };
        if !selection::context_is_current(&model, &request.scope, &context) {
            return;
        }
        adapter
            .selected_context
            .set(Some(objects::ScopedTreeContext {
                scope: request.scope,
                context,
            }));
        adapter.anchor_scope.set(None);
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
        workspace.set(Workspace::Pcb);
        objects_open.set(false);
        inspect_open.set(true);
    })
}

pub(in crate::presentation) fn on_pcb_wiring_edit_board_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    scope: Scope,
    generation: u64,
    token: boardstudio_application::SnapshotToken,
    workspace: Signal<Workspace>,
    instance_selection: InstanceSelection,
) -> EventHandler<()> {
    EventHandler::new(move |()| {
        if workspace() != Workspace::Pcb
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
            .filter(|snapshot| snapshot.token == token)
        else {
            return;
        };
        if !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
        {
            return;
        }
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
    })
}

pub(super) fn case_admission(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    case_state: case_workspace::CaseWorkspaceState,
    instance_selection: InstanceSelection,
    owner_scope: Scope,
    owner_token: boardstudio_application::SnapshotToken,
    owner_generation: u64,
) -> case_workspace::Admission {
    case_workspace::Admission {
        runtime,
        adapter,
        case_state,
        instance_selection,
        owner_scope,
        owner_token,
        owner_generation,
    }
}

pub(super) fn on_case_action_handler(
    admission: case_workspace::Admission,
    navigate: EventHandler<(Scope, String, Option<String>)>,
    workspace: Signal<Workspace>,
    case_workspace_state: case_workspace::CaseWorkspaceState,
) -> EventHandler<case_workspace::TreeAction> {
    EventHandler::new(move |action| {
        if workspace() != Workspace::Case {
            return;
        }
        case_workspace_state.apply_tree_action(action, &admission, navigate);
    })
}

pub(super) fn on_case_display_handler(
    admission: case_workspace::Admission,
    workspace: Signal<Workspace>,
    case_workspace_state: case_workspace::CaseWorkspaceState,
) -> EventHandler<case_workspace::DisplayRequest> {
    EventHandler::new(move |request| {
        if workspace() != Workspace::Case {
            return;
        }
        case_workspace_state.apply_display_request(request, &admission);
    })
}

pub(in crate::presentation) fn on_keymap_layer_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    scope: Scope,
    generation: u64,
    workspace: Signal<Workspace>,
    mut active_layer: Signal<String>,
) -> EventHandler<String> {
    EventHandler::new(move |layer_id: String| {
        if workspace() != Workspace::Keymap
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
        let exists = snapshot
            .document
            .keymap
            .as_ref()
            .filter(|map| !map.layers.is_empty())
            .map_or(layer_id == "base", |map| {
                map.layers.iter().any(|layer| layer.id == layer_id)
            });
        if exists {
            active_layer.set(layer_id);
        }
    })
}

pub(in crate::presentation) struct OnKeymapExportInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub generation: u64,
    pub workspace: Signal<Workspace>,
    pub instance_selection: InstanceSelection,
}

pub(in crate::presentation) fn on_keymap_export_handler(
    input: OnKeymapExportInput,
) -> EventHandler<()> {
    let OnKeymapExportInput {
        runtime,
        adapter,
        scope,
        token,
        revision,
        generation,
        workspace,
        instance_selection,
    } = input;
    EventHandler::new(move |_| {
        if workspace() != Workspace::Keymap
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
            return;
        }
        if model.accepted.as_ref().is_none_or(|accepted| {
            accepted.token != token
                || accepted.document.revision != revision
                || accepted.scene.revision != revision
        }) {
            return;
        }
        runtime.export_firmware();
    })
}

pub(super) fn nudge_tree_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    generation: u64,
    mut inspect_open: Signal<bool>,
) -> EventHandler<objects::TreeNudgeRequest> {
    EventHandler::new(move |request: objects::TreeNudgeRequest| {
        if runtime.scope().as_ref() != Some(&request.scope)
            || (adapter.generation)() != generation
            || !((request.dx == 0 && request.dy.abs() == 1)
                || (request.dy == 0 && request.dx.abs() == 1))
        {
            return;
        }
        let model = runtime.model();
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        let Some(part) = snapshot
            .document
            .parts
            .iter()
            .find(|part| part.id == request.part_id)
        else {
            return;
        };
        if part.locked == Some(true) {
            return;
        }
        let Some(board) = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == model.active_board_id)
        else {
            return;
        };
        if !board.part_ids.contains(&part.id) {
            return;
        }
        let Some(context) = objects::context_for_part(&model, &part.id) else {
            return;
        };
        if !matches!(
            context,
            objects::TreeContext::Key { .. }
                | objects::TreeContext::Component {
                    matrix_id: None,
                    ..
                }
        ) {
            return;
        }
        let selected_ids = if model.selected_part_ids.contains(&part.id) {
            model.selected_part_ids.clone()
        } else {
            vec![part.id.clone()]
        };
        let eligible = selection::eligible_live_ids(&model);
        let moving: Vec<_> = selected_ids
            .iter()
            .filter(|id| eligible.iter().any(|candidate| candidate == *id))
            .filter_map(|id| {
                snapshot
                    .document
                    .parts
                    .iter()
                    .find(|candidate| candidate.id == *id && candidate.locked != Some(true))
            })
            .collect();
        if moving.is_empty() {
            return;
        }
        let step = if request.large_step { 1.0 } else { 0.1 };
        let moving_ids: Vec<_> = moving.iter().map(|part| part.id.clone()).collect();
        let mut selected_context = adapter.selected_context;
        selected_context.set(Some(objects::ScopedTreeContext {
            scope: request.scope.clone(),
            context,
        }));
        let mut anchor_scope = adapter.anchor_scope;
        anchor_scope.set(None);
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: moving_ids.clone(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
        let current = runtime.model();
        if runtime.scope().as_ref() != Some(&request.scope) {
            return;
        }
        if current.selection_anchor_id.as_ref() == moving_ids.first()
            && current
                .selection_anchor_id
                .as_ref()
                .is_some_and(|anchor| selection::eligible_live_ids(&current).contains(anchor))
        {
            anchor_scope.set(Some(request.scope));
        }
        inspect_open.set(true);
        // Each press is a delta intent: it moves each part by one step from the
        // position accepted when it runs, so held-key repeats all land.
        runtime.submit(Event::ResolveEdit {
            operation_id: runtime.operation(),
            label: "layout-nudge".into(),
            resolver: layout_component_edits::nudge_resolver(
                moving_ids,
                model.active_board_id.clone(),
                Vec2 {
                    x: f64::from(request.dx) * step,
                    y: f64::from(request.dy) * step,
                },
            ),
        });
        let direction = match (request.dx, request.dy) {
            (-1, 0) => "left",
            (1, 0) => "right",
            (0, 1) => "up",
            (0, -1) => "down",
            _ => return,
        };
        runtime.report(format!(
            "{} moved {direction} {step:.1} mm. Position X {:.1}, Y {:.1} mm.",
            part.reference,
            part.pose.at.x + f64::from(request.dx) * step,
            part.pose.at.y + f64::from(request.dy) * step,
        ));
    })
}
