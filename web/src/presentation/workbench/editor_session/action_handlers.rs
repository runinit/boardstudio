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
