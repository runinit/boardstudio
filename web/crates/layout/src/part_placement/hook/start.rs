//! Admit and prepare a new catalogue placement.
use super::*;
use boardstudio_application::AcceptedSnapshot;

pub(super) fn start_placement(
    host: &PartPlacementHost,
    state: PlacementState,
    alive: Rc<Cell<bool>>,
) -> impl FnMut(
    String,
    PartKind,
    PlacementWorkflow,
    bool,
    Option<(String, PartDefinition)>,
    PlacementIntent,
) + use<> {
    let runtime = host.runtime.clone();
    let PartPlacementHost {
        workspace,
        generation,
        guide_preferences,
        snap_settings,
        layout_target,
        canvas_center,
        ..
    } = *host;
    let PlacementState {
        chooser,
        mut active,
        mut preparing,
        committing,
        mut error,
    } = state;
    let query = host.parts_query;
    let load_definition = host.load_definition.clone();
    let adapter_for_async = host.adapter.clone();
    let canvas_interaction = host.canvas_interaction.clone();
    move |definition_id: String,
          kind: PartKind,
          workflow: PlacementWorkflow,
          apply_to_key: bool,
          source_definition: Option<(String, PartDefinition)>,
          intent: PlacementIntent| {
        let source_workspace = workspace();
        let guided_wiring = workflow == PlacementWorkflow::WiringController
            && guide_preferences().as_ref().is_some_and(|preferences| {
                preferences.open && preferences.current_stage == SetupGuideStage::Wiring
            });
        if match workflow {
            PlacementWorkflow::WiringController => {
                source_workspace != Workspace::Parts
                    && !(source_workspace == Workspace::Layout && guided_wiring)
            }
            PlacementWorkflow::PcbController => source_workspace != Workspace::Parts,
            PlacementWorkflow::GeneralComponent => !matches!(
                source_workspace,
                Workspace::Parts | Workspace::Layout | Workspace::Pcb
            ),
        } {
            return;
        }
        let current_model = runtime.model();
        let admission = PlacementAdmission::capture(
            &runtime,
            generation(),
            workspace(),
            Workspace::Layout,
            guide_preferences(),
        );
        if active
            .read()
            .as_ref()
            .is_some_and(|placement| !admission.admits(&placement.owner, &runtime, &current_model))
        {
            active.set(None);
        }
        let current_model = runtime.model();
        let admission = PlacementAdmission::capture(
            &runtime,
            generation(),
            workspace(),
            preparing
                .read()
                .as_ref()
                .map_or(Workspace::Parts, |owner| owner.source_workspace),
            guide_preferences(),
        );
        if preparing
            .read()
            .as_ref()
            .is_some_and(|owner| !admission.admits(owner, &runtime, &current_model))
        {
            preparing.set(None);
        }
        if active.read().is_some()
            || preparing.read().is_some()
            || *committing.read()
            || workspace() != source_workspace
        {
            return;
        }
        let model = runtime.model();
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        let Some(scope) = runtime.scope() else {
            return;
        };
        if apply_to_key && source_definition.is_some() {
            error.set(Some(
                "A module source component cannot be applied to a matrix key.".into(),
            ));
            return;
        }
        let source_definition = source_definition.map(|(module_id, definition)| {
            placement_source_definition(&snapshot.document, &module_id, definition)
        });
        let definition_id = source_definition
            .as_ref()
            .map(|definition| definition.id.clone())
            .unwrap_or(definition_id);
        let kind = source_definition
            .as_ref()
            .map(|definition| definition.kind.clone())
            .unwrap_or(kind);
        let guide_is_live = guide_preferences().as_ref().is_some_and(|preferences| {
            preferences.open
                && preferences.current_stage == SetupGuideStage::Wiring
                && preferences.project_id == snapshot.document.id
        });
        let pcb_chooser_is_live = chooser.read().as_ref().is_some_and(|owner| {
            owner.is_current(runtime.as_ref(), generation(), source_workspace)
        });
        if (workflow == PlacementWorkflow::WiringController && !guide_is_live)
            || (workflow == PlacementWorkflow::PcbController && !pcb_chooser_is_live)
            || model.lifecycle != Lifecycle::Ready
            || model.durability
                != (Durability::Saved {
                    revision: snapshot.document.revision,
                })
            || model.display_preview.is_some()
            || model.gesture.is_some()
            || model.active_board_id != scope.board_id
        {
            return;
        }
        let selected_context = (adapter_for_async.selected_context)();
        if apply_to_key
            && selected_context
                .as_ref()
                .is_some_and(|selected| matches!(selected.context, TreeContext::Key { .. }))
        {
            let Some(selected) = selected_context else {
                return;
            };
            if selected.scope != scope
                || !crate::selection::context_is_current(&model, &scope, &selected.context)
            {
                return;
            }
            spawn_local(apply_key_component(
                runtime.clone(),
                load_definition.clone(),
                alive.clone(),
                error,
                KeyComponentLoad {
                    accepted: snapshot.clone(),
                    generation,
                    accepted_generation: generation(),
                    workspace,
                    source_workspace,
                    current_context: adapter_for_async.selected_context,
                    selected,
                    definition_id,
                    kind,
                },
            ));
            return;
        }
        let part_id = match browser_uuid() {
            Ok(id) => format!("ui-{id}"),
            Err(message) => {
                error.set(Some(message));
                return;
            }
        };
        // Ticket 22: the initial placement grid follows the selected matrix
        // pitch (19.05 mm only when the selection names no matrix).
        let pitch = (adapter_for_async.selected_context)()
            .and_then(|selected| match &selected.context {
                TreeContext::Matrix { matrix_id }
                | TreeContext::Row { matrix_id, .. }
                | TreeContext::Column { matrix_id, .. }
                | TreeContext::Key { matrix_id, .. } => Some(matrix_id.clone()),
                TreeContext::Component { matrix_id, .. } => matrix_id.clone(),
                _ => None,
            })
            .and_then(|matrix_id| {
                snapshot
                    .document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == matrix_id)
            })
            .map(|matrix| matrix.pitch);
        let at = grid_snap_point(canvas_center, snap_settings.read().snap_fraction, pitch);
        let layout_id = layout_target().filter(|layout_id| {
            snapshot
                .document
                .layouts
                .iter()
                .any(|layout| layout.id == *layout_id && layout.board_id == scope.board_id)
        });
        let Some(owner) = PlacementOwner::capture(
            snapshot,
            PlacementCapture {
                scope: scope.clone(),
                generation: generation(),
                part_id,
                definition_id: definition_id.clone(),
                kind: kind.clone(),
                workflow,
                intent,
                source_workspace,
                layout_id,
                at,
            },
        ) else {
            return;
        };
        if !canvas_interaction.try_acquire(CanvasInteractionOwner::PartPlacement) {
            return;
        }
        preparing.set(Some(owner.clone()));
        error.set(None);
        let accepted = snapshot.clone();
        let reversible_document = (*snapshot.document).clone();
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut active = active;
        let mut error = error;
        let alive = alive.clone();
        let mut adapter = adapter_for_async.clone();
        let guide = guide_preferences;
        let mut query = query;
        let mut preparing = preparing;
        let workspace_for_preparation = workspace;
        let generation_for_preparation = generation;
        let guide_for_preparation = guide;
        let load_definition = load_definition.clone();
        let source_definition = source_definition.clone();
        spawn_local(async move {
            let definition = match source_definition {
                Some(definition) => Ok(definition),
                None => load_definition(reversible_document, definition_id).await,
            };
            if !alive.get() {
                return;
            }
            if preparing.read().as_ref() != Some(&owner) {
                return;
            }
            let model = runtime.model();
            let still_owned = PlacementAdmission::capture(
                &runtime,
                generation_for_preparation(),
                workspace_for_preparation(),
                owner.source_workspace,
                guide_for_preparation(),
            )
            .admits(&owner, &runtime, &model);
            if !still_owned
                || workspace_for_preparation() != owner.source_workspace
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: accepted.document.revision,
                    })
            {
                preparing.set(None);
                return;
            }
            let definition = match definition {
                Ok(definition) => definition,
                Err(message) => {
                    preparing.set(None);
                    error.set(Some(message));
                    return;
                }
            };
            if definition.kind != kind {
                preparing.set(None);
                error.set(Some(
                    "The selected component changed before placement.".into(),
                ));
                return;
            }
            if workflow.is_controller() && !matches!(definition.kind, PartKind::Controller) {
                preparing.set(None);
                error.set(Some(
                    "The selected catalogue item is not a controller.".into(),
                ));
                return;
            }
            let pending = if workflow.is_controller() {
                controller_part(
                    definition,
                    owner.part_id.clone(),
                    owner.reference.clone(),
                    owner.at,
                )
            } else {
                Some(component_part(
                    definition,
                    owner.part_id.clone(),
                    owner.reference.clone(),
                    owner.at,
                ))
            };
            let Some(pending) = pending else {
                preparing.set(None);
                error.set(Some(
                    "The selected catalogue item is not a controller.".into(),
                ));
                return;
            };
            active.set(Some(ActivePartPlacement {
                owner,
                snap_document: Rc::new(document_with_pending_definition(
                    &accepted.document,
                    &pending.definition,
                )),
                pending,
            }));
            preparing.set(None);
            if workflow.is_controller() {
                query.set("controller".into());
            }
            workspace.set(Workspace::Layout);
            adapter.selected_context.set(None);
            adapter.anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
        });
    }
}

struct KeyComponentLoad {
    accepted: AcceptedSnapshot,
    generation: Signal<u64>,
    accepted_generation: u64,
    workspace: Signal<Workspace>,
    source_workspace: Workspace,
    current_context: Signal<Option<ScopedTreeContext>>,
    selected: ScopedTreeContext,
    definition_id: String,
    kind: PartKind,
}

async fn apply_key_component(
    runtime: Rc<Runtime>,
    load_definition: DefinitionLoader,
    alive: Rc<Cell<bool>>,
    mut error: Signal<Option<String>>,
    request: KeyComponentLoad,
) {
    let KeyComponentLoad {
        accepted,
        generation,
        accepted_generation,
        workspace,
        source_workspace,
        current_context,
        selected,
        definition_id,
        kind,
    } = request;
    let scope = &selected.scope;
    let loaded = load_definition((*accepted.document).clone(), definition_id).await;
    if !alive.get() {
        return;
    }
    let route_is_current = || {
        runtime.scope().as_ref() == Some(scope)
            && current_context().as_ref() == Some(&selected)
            && workspace() == source_workspace
            && generation() == accepted_generation
    };
    let definition = match loaded {
        Ok(definition) if definition.kind == kind => definition,
        invalid => {
            if accepted_snapshot_is_current(&runtime, &accepted) && route_is_current() {
                error.set(Some(invalid.err().unwrap_or_else(|| {
                    "The selected component changed before it was applied.".into()
                })));
            }
            return;
        }
    };
    // The resolver checks the key in the document it actually edits. Only load feedback
    // depends on the original accepted snapshot still being current.
    if !route_is_current() {
        return;
    }
    let outcome = runtime.resolve_edit(
        "layout-key-component",
        Some("component".into()),
        key_component_resolver(scope.clone(), selected, definition),
    );
    error.set(None);
    if let Settlement::Failed { message } = outcome.settlement(true) {
        error.set(Some(message));
    }
}
