//! Browser placement hook: owns the interaction lifetime, definition loading and
//! commit/cancel routing for the canvas. Identity and proposal logic live in
//! `super::owner` and `super::plan`.
use super::owner::{
    ActivePartPlacement, ComponentActionOwner, ControllerChooserOwner, KeyEditOwner,
    PlacementAdmission, PlacementCapture, PlacementOwner, PlacementPendingKey, PlacementWorkflow,
    accepted_snapshot_is_current, key_component_resolver, placement_resolver,
};
use super::plan::{
    PlacementIntent, component_part, controller_part, document_with_pending_definition,
    grid_snap_point, placement_source_definition, update_pending_part,
};
use crate::canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use crate::objects::{self, LayoutSnapSettings};
use crate::parts::{PartsQuery, PartsSelection};
use crate::runtime::Runtime;
use crate::selection::SelectionAdapter;
use crate::setup_guide::{SetupGuidePreferences, SetupGuideStage};
use boardstudio_application::{Durability, Event, Lifecycle, SelectionMode};
use boardstudio_model::model::{PartDefinition, PartKind, ProjectDoc, Vec2};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;

pub use super::owner::ComponentPlacementAction;

#[derive(Clone)]
pub struct PartPlacementMount {
    pub projection: Option<ActivePartPlacement>,
    pub busy: bool,
    pub error: Option<String>,
    pub on_choose_controller: EventHandler<()>,
    pub on_place_controller: EventHandler<String>,
    pub controller_placement_enabled: bool,
    pub controller_back: Option<EventHandler<()>>,
    pub on_place_component: EventHandler<ComponentPlacementAction>,
    pub on_move: EventHandler<Vec2>,
    pub on_commit: EventHandler<Vec2>,
    pub on_cancel: EventHandler<()>,
}

impl PartPlacementMount {
    #[cfg(test)]
    pub fn owns_canvas(&self) -> bool {
        self.busy || self.projection.is_some()
    }
}

pub struct PartPlacementHost {
    pub runtime: Rc<Runtime>,
    pub load_definition: DefinitionLoader,
    pub workspace: Signal<&'static str>,
    pub generation: Signal<u64>,
    pub version: Signal<u64>,
    pub adapter: SelectionAdapter,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub guide_preferences: Signal<Option<SetupGuidePreferences>>,
    pub parts_query: PartsQuery,
    pub parts_selection: PartsSelection,
    pub snap_settings: Signal<LayoutSnapSettings>,
    pub layout_target: Signal<Option<String>>,
    pub canvas_center: Vec2,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub canvas_interaction: CanvasInteractionArbiter,
}

pub type DefinitionLoader = Rc<
    dyn Fn(
        ProjectDoc,
        String,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PartDefinition, String>>>>,
>;

pub fn use_controller_placement(host: PartPlacementHost) -> PartPlacementMount {
    let PartPlacementHost {
        runtime,
        load_definition,
        workspace,
        generation,
        version,
        adapter,
        mut layout_selection_kind,
        guide_preferences,
        parts_query,
        parts_selection,
        snap_settings,
        layout_target,
        canvas_center,
        mut objects_open,
        mut inspect_open,
        canvas_interaction,
    } = host;
    let chooser = use_signal(|| None::<ControllerChooserOwner>);
    let active = use_signal(|| None::<ActivePartPlacement>);
    let preparing = use_signal(|| None::<PlacementOwner>);
    let committing = use_signal(|| false);
    let key_edit_disabled = use_signal(|| false);
    let pending_edits = use_hook(PendingEditSignals::<PlacementPendingKey>::new);
    pending_edits.bind_one_shot(PlacementPendingKey::Placement, committing);
    pending_edits.bind_one_shot(PlacementPendingKey::KeyComponent, key_edit_disabled);
    let error = use_signal(|| None::<String>);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    let action_owner = ComponentActionOwner::capture(
        runtime.as_ref(),
        generation(),
        workspace(),
        (adapter.selected_context)(),
    );
    let on_choose_controller = {
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut query = parts_query;
        let mut selected = parts_selection;
        let guide = guide_preferences;
        let entry_owner = action_owner.clone();
        let mut chooser = chooser;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        let canvas_interaction = canvas_interaction.clone();
        move |_| {
            if canvas_interaction.current().is_some() {
                return;
            }
            let model = runtime.model();
            let Some(project_id) = model
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.id.clone())
            else {
                return;
            };
            let guided = guide().as_ref().is_some_and(|preferences| {
                preferences.project_id == project_id
                    && preferences.open
                    && preferences.current_stage == SetupGuideStage::Wiring
            });
            let Some(owner) = entry_owner.as_ref().filter(|owner| {
                owner.is_current(
                    runtime.as_ref(),
                    generation(),
                    workspace(),
                    selected_context(),
                    owner.workspace,
                )
            }) else {
                return;
            };
            if !guided && owner.workspace != "PCB" {
                return;
            }
            chooser.set((!guided).then(|| ControllerChooserOwner {
                accepted: owner.accepted.clone(),
                scope: owner.scope.clone(),
                generation: owner.generation,
            }));
            workspace.set(if guided { "Layout" } else { "Parts" });
            query.set("controller".into());
            selected.set(None);
            selected_context.set(None);
            anchor_scope.set(None);
            objects_open.set(true);
            inspect_open.set(!guided);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
        }
    };

    let start_placement = {
        let runtime = runtime.clone();
        let mut active = active;
        let query = parts_query;
        let load_definition = load_definition.clone();
        let mut preparing = preparing;
        let mut error = error;
        let alive = alive.clone();
        let adapter_for_async = adapter.clone();
        let canvas_interaction = canvas_interaction.clone();
        let pending_edits = pending_edits.clone();
        Rc::new(RefCell::new(
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
                        source_workspace != "Parts"
                            && !(source_workspace == "Layout" && guided_wiring)
                    }
                    PlacementWorkflow::PcbController => source_workspace != "Parts",
                    PlacementWorkflow::GeneralComponent => {
                        !matches!(source_workspace, "Parts" | "Layout" | "PCB")
                    }
                } {
                    return;
                }
                let current_model = runtime.model();
                let admission = PlacementAdmission::capture(
                    &runtime,
                    generation(),
                    workspace(),
                    "Layout",
                    guide_preferences(),
                );
                if active.read().as_ref().is_some_and(|placement| {
                    !admission.admits(&placement.owner, &runtime, &current_model)
                }) {
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
                        .map_or("Parts", |owner| owner.source_workspace),
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
                    || *key_edit_disabled.read()
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
                    let accepted = snapshot.clone();
                    let accepted_generation = generation();
                    let reversible_document = (*snapshot.document).clone();
                    let runtime = runtime.clone();
                    let load_definition = load_definition.clone();
                    let mut error = error;
                    let current_context = adapter_for_async.selected_context;
                    let workspace_for_load = workspace;
                    let generation_for_load = generation;
                    let pending_edits_for_key = pending_edits.clone();
                    let alive = alive.clone();
                    spawn_local(async move {
                        let definition =
                            match load_definition(reversible_document, definition_id).await {
                                Ok(definition) => definition,
                                Err(message) => {
                                    if alive.get()
                                        && accepted_snapshot_is_current(runtime.as_ref(), &accepted)
                                        && runtime.scope().as_ref() == Some(&scope)
                                        && workspace_for_load() == source_workspace
                                        && generation_for_load() == accepted_generation
                                        && current_context() == Some(selected.clone())
                                    {
                                        error.set(Some(message));
                                    }
                                    return;
                                }
                            };
                        if !alive.get() {
                            return;
                        }
                        if definition.kind != kind {
                            if accepted_snapshot_is_current(runtime.as_ref(), &accepted)
                                && runtime.scope().as_ref() == Some(&scope)
                                && workspace_for_load() == source_workspace
                                && generation_for_load() == accepted_generation
                                && current_context() == Some(selected.clone())
                            {
                                error.set(Some(
                                    "The selected component changed before it was applied.".into(),
                                ));
                            }
                            return;
                        }
                        if !alive.get()
                            || runtime.scope().as_ref() != Some(&scope)
                            || current_context() != Some(selected.clone())
                            || workspace_for_load() != source_workspace
                            || generation_for_load() != accepted_generation
                        {
                            return;
                        }
                        pending_edits_for_key.begin_one_shot(
                            &runtime,
                            PlacementPendingKey::KeyComponent,
                            "layout-key-component",
                            Some("component".into()),
                            key_component_resolver(scope.clone(), selected.clone(), definition),
                        );
                        let pending = KeyEditOwner {
                            scope: scope.clone(),
                            generation: accepted_generation,
                            source_workspace,
                            selection: selected.clone(),
                        };
                        error.set(None);
                        let runtime = runtime.clone();
                        let pending_edits = pending_edits_for_key.clone();
                        let current_context = current_context;
                        let workspace_for_key = workspace_for_load;
                        let generation_for_key = generation_for_load;
                        let alive = alive.clone();
                        spawn_local(async move {
                            loop {
                                if !alive.get() {
                                    return;
                                }
                                let still_current = runtime.scope().as_ref()
                                    == Some(&pending.scope)
                                    && workspace_for_key() == pending.source_workspace
                                    && generation_for_key() == pending.generation
                                    && current_context() == Some(pending.selection.clone());
                                let results =
                                    pending_edits.settle(still_current, |_| String::new());
                                if let Some(result) = results.into_iter().next() {
                                    if let PendingEditResult::Failed { message, .. } = result {
                                        error.set(Some(message));
                                    }
                                    break;
                                }
                                gloo_timers::future::TimeoutFuture::new(16).await;
                            }
                        });
                    });
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
                let layout_id =
                    layout_target().filter(|layout_id| {
                        snapshot.document.layouts.iter().any(|layout| {
                            layout.id == *layout_id && layout.board_id == scope.board_id
                        })
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
                    if workflow.is_controller() && !matches!(definition.kind, PartKind::Controller)
                    {
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
                    if !PlacementAdmission::capture(
                        &runtime,
                        generation_for_preparation(),
                        workspace_for_preparation(),
                        owner.source_workspace,
                        guide_for_preparation(),
                    )
                    .admits(&owner, &runtime, &model)
                    {
                        preparing.set(None);
                        return;
                    }
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
                    workspace.set("Layout");
                    adapter.selected_context.set(None);
                    adapter.anchor_scope.set(None);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: Vec::new(),
                        range_part_ids: Vec::new(),
                        mode: SelectionMode::Replace,
                    });
                });
            },
        ))
    };
    let ordinary_controller = chooser
        .read()
        .as_ref()
        .is_some_and(|owner| owner.is_current(runtime.as_ref(), generation(), workspace()));
    let guided_controller = guide_preferences().as_ref().is_some_and(|preferences| {
        preferences.open
            && preferences.current_stage == SetupGuideStage::Wiring
            && runtime
                .model()
                .accepted
                .as_ref()
                .is_some_and(|accepted| preferences.project_id == accepted.document.id)
    });
    let on_place_controller = {
        let start = start_placement.clone();
        let runtime = runtime.clone();
        let controller_action_owner = action_owner.clone();
        let selected_context = adapter.selected_context;
        let workflow = if ordinary_controller {
            PlacementWorkflow::PcbController
        } else {
            PlacementWorkflow::WiringController
        };
        move |definition_id: String| {
            if workflow == PlacementWorkflow::PcbController
                && !controller_action_owner.as_ref().is_some_and(|owner| {
                    owner.is_current(
                        runtime.as_ref(),
                        generation(),
                        workspace(),
                        selected_context(),
                        "Parts",
                    )
                })
            {
                return;
            }
            start.borrow_mut()(
                definition_id,
                PartKind::Controller,
                workflow,
                false,
                None,
                PlacementIntent::Existing,
            );
        }
    };
    let controller_back =
        ordinary_controller.then(|| {
            let runtime = runtime.clone();
            let mut workspace = workspace;
            let mut chooser = chooser;
            let canvas_interaction = canvas_interaction.clone();
            EventHandler::new(move |_| {
                if chooser.read().as_ref().is_some_and(|owner| {
                    owner.is_current(runtime.as_ref(), generation(), workspace())
                }) && canvas_interaction.current().is_none()
                {
                    chooser.set(None);
                    workspace.set("PCB");
                }
            })
        });
    let on_place_component = {
        let start = start_placement;
        let runtime = runtime.clone();
        let selected_context = adapter.selected_context;
        move |action: ComponentPlacementAction| {
            let expected_workspace = match action {
                ComponentPlacementAction::AddObject { .. }
                | ComponentPlacementAction::AddMirrorOnlyObject { .. }
                | ComponentPlacementAction::AddSourceObject { .. } => {
                    let Some(owner) = action_owner
                        .as_ref()
                        .filter(|owner| matches!(owner.workspace, "Layout" | "PCB"))
                    else {
                        return;
                    };
                    owner.workspace
                }
                ComponentPlacementAction::PartsInspector { .. } => "Parts",
            };
            if !action_owner.as_ref().is_some_and(|owner| {
                owner.is_current(
                    runtime.as_ref(),
                    generation(),
                    workspace(),
                    selected_context(),
                    expected_workspace,
                )
            }) {
                return;
            }
            match action {
                ComponentPlacementAction::AddObject {
                    definition_id,
                    kind,
                } => start.borrow_mut()(
                    definition_id,
                    kind,
                    PlacementWorkflow::GeneralComponent,
                    false,
                    None,
                    PlacementIntent::LinkedCatalogueDefault,
                ),
                ComponentPlacementAction::AddMirrorOnlyObject {
                    definition_id,
                    kind,
                } => start.borrow_mut()(
                    definition_id,
                    kind,
                    PlacementWorkflow::GeneralComponent,
                    false,
                    None,
                    PlacementIntent::MirrorOnly,
                ),
                ComponentPlacementAction::AddSourceObject {
                    module_definition_id,
                    definition,
                } => start.borrow_mut()(
                    definition.id.clone(),
                    definition.kind.clone(),
                    PlacementWorkflow::GeneralComponent,
                    false,
                    Some((module_definition_id, definition)),
                    PlacementIntent::Existing,
                ),
                ComponentPlacementAction::PartsInspector {
                    definition_id,
                    kind,
                } => start.borrow_mut()(
                    definition_id,
                    kind,
                    PlacementWorkflow::GeneralComponent,
                    true,
                    None,
                    PlacementIntent::Existing,
                ),
            }
        }
    };

    let observed_version = version();
    let observed_generation = generation();
    let observed_workspace = workspace();
    let observed_guide = guide_preferences();
    let cleanup_runtime = runtime.clone();
    let mut cleanup_active = active;
    let mut cleanup_preparing = preparing;
    let mut cleanup_chooser = chooser;
    use_effect(use_reactive!(
        |observed_version, observed_generation, observed_workspace, observed_guide| {
            let _ = observed_version;
            let stale_chooser = cleanup_chooser.read().as_ref().is_some_and(|owner| {
                !owner.is_current(
                    cleanup_runtime.as_ref(),
                    observed_generation,
                    observed_workspace,
                )
            });
            if stale_chooser {
                cleanup_chooser.set(None);
            }
            let model = cleanup_runtime.model();
            let stale_active = cleanup_active.read().as_ref().is_some_and(|placement| {
                !PlacementAdmission::capture(
                    &cleanup_runtime,
                    observed_generation,
                    observed_workspace,
                    "Layout",
                    observed_guide.clone(),
                )
                .admits(&placement.owner, &cleanup_runtime, &model)
            });
            if stale_active {
                cleanup_active.set(None);
            }
            let stale_preparing = cleanup_preparing.read().as_ref().is_some_and(|owner| {
                !PlacementAdmission::capture(
                    &cleanup_runtime,
                    observed_generation,
                    observed_workspace,
                    owner.source_workspace,
                    observed_guide.clone(),
                )
                .admits(owner, &cleanup_runtime, &model)
            });
            if stale_preparing {
                cleanup_preparing.set(None);
            }
        }
    ));
    let on_move = {
        let mut active = active;
        let canvas_interaction = canvas_interaction.clone();
        move |at: Vec2| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                return;
            }
            if let Some(mut placement) = active() {
                update_pending_part(&mut placement.pending, at);
                active.set(Some(placement));
            }
        }
    };
    let on_commit = {
        let runtime = runtime.clone();
        let mut active = active;
        let mut error = error;
        let alive = alive.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |at: Vec2| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                return;
            }
            let Some(mut placement) = active() else {
                return;
            };
            if *committing.read() {
                return;
            }
            update_pending_part(&mut placement.pending, at);
            placement
                .owner
                .record_committed_position(placement.pending.at);
            let model = runtime.model();
            if model.accepted.is_none() {
                return;
            }
            if !PlacementAdmission::capture(
                &runtime,
                generation(),
                workspace(),
                "Layout",
                guide_preferences(),
            )
            .admits(&placement.owner, &runtime, &model)
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: placement.owner.revision,
                    })
            {
                return;
            }
            let owner = placement.owner.clone();
            pending_edits.begin_one_shot(
                &runtime,
                PlacementPendingKey::Placement,
                "layout-component-placement",
                Some("placement".into()),
                placement_resolver(
                    owner.board_id.clone(),
                    placement.pending.definition.clone(),
                    placement.pending.part.clone(),
                    owner.layout_id.clone(),
                    owner.intent,
                ),
            );
            // The helper disables this canvas-owning one-shot until settlement.
            active.set(None);
            error.set(None);
            let runtime = runtime.clone();
            let pending_edits = pending_edits.clone();
            let mut error = error;
            let mut workspace = workspace;
            let guide_preferences = guide_preferences;
            let mut selected_context = adapter.selected_context;
            let mut anchor_scope = adapter.anchor_scope;
            let canvas_interaction = canvas_interaction.clone();
            let alive = alive.clone();
            spawn_local(async move {
                let settlement = loop {
                    if !alive.get() {
                        return;
                    }
                    let model = runtime.model();
                    let route_live = placement_route_is_current(
                        &runtime,
                        &model,
                        &owner,
                        generation(),
                        workspace(),
                        guide_preferences(),
                    );
                    if let Some(result) = pending_edits
                        .settle(route_live, |_| String::new())
                        .into_iter()
                        .next()
                    {
                        break result;
                    }
                    gloo_timers::future::TimeoutFuture::new(16).await;
                };
                let model = runtime.model();
                match settlement {
                    PendingEditResult::Landed { .. } => {
                        // Select from the accepted document at the landing; nothing when the
                        // placed part is no longer there.
                        let placed = model.accepted.as_ref().is_some_and(|accepted| {
                            accepted
                                .document
                                .parts
                                .iter()
                                .any(|part| part.id == owner.part_id)
                        });
                        let next_context =
                            if owner.workflow == PlacementWorkflow::GeneralComponent && placed {
                                objects::context_for_part(&model, &owner.part_id).map(|context| {
                                    ScopedTreeContext {
                                        scope: owner.scope.clone(),
                                        context,
                                    }
                                })
                            } else {
                                None
                            };
                        selected_context.set(next_context);
                        anchor_scope.set(None);
                        workspace.set(if owner.workflow == PlacementWorkflow::WiringController {
                            "Layout"
                        } else if owner.workflow.is_controller() {
                            "PCB"
                        } else {
                            "Layout"
                        });
                        if owner.workflow == PlacementWorkflow::GeneralComponent {
                            layout_selection_kind.set(objects::LayoutSelectionKind::Part);
                        }
                        runtime.submit(Event::SelectParts {
                            operation_id: runtime.operation(),
                            part_ids: if owner.workflow == PlacementWorkflow::GeneralComponent
                                && placed
                            {
                                vec![owner.part_id.clone()]
                            } else {
                                Vec::new()
                            },
                            range_part_ids: Vec::new(),
                            mode: SelectionMode::Replace,
                        });
                    }
                    PendingEditResult::Failed { message, .. } => {
                        workspace.set(owner.source_workspace);
                        error.set(Some(message));
                    }
                    PendingEditResult::Retired { .. } => {}
                }
                canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
            });
        }
    };
    let on_cancel = {
        let runtime = runtime.clone();
        let mut active = active;
        let mut preparing = preparing;
        let mut workspace = workspace;
        let guide = guide_preferences;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        let canvas_interaction = canvas_interaction.clone();
        move |_| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                return;
            }
            if *committing.read() {
                return;
            }
            let owner_and_workspace = active
                .read()
                .as_ref()
                .map(|placement| (placement.owner.clone(), "Layout"))
                .or_else(|| {
                    preparing
                        .read()
                        .clone()
                        .map(|owner| (owner.clone(), owner.source_workspace))
                });
            let Some((owner, owner_workspace)) = owner_and_workspace else {
                return;
            };
            let model = runtime.model();
            let current = model.accepted.as_ref().is_some_and(|_snapshot| {
                PlacementAdmission::capture(
                    &runtime,
                    generation(),
                    workspace(),
                    owner_workspace,
                    guide(),
                )
                .admits(&owner, &runtime, &model)
            });
            active.set(None);
            preparing.set(None);
            canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
            if current {
                selected_context.set(None);
                anchor_scope.set(None);
                workspace.set(if owner.workflow == PlacementWorkflow::PcbController {
                    "PCB"
                } else {
                    "Layout"
                });
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: Vec::new(),
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
            }
        }
    };
    let on_move = EventHandler::new(on_move);
    let on_commit = EventHandler::new(on_commit);
    let projection = active().filter(|placement| {
        let model = runtime.model();
        PlacementAdmission::capture(
            &runtime,
            generation(),
            workspace(),
            "Layout",
            guide_preferences(),
        )
        .admits(&placement.owner, &runtime, &model)
    });
    let busy = preparing.read().is_some() || *committing.read() || *key_edit_disabled.read();
    let owns_canvas = busy || projection.is_some();
    use_effect(use_reactive(
        (&version(), &workspace(), &generation(), &owns_canvas),
        {
            let canvas_interaction = canvas_interaction.clone();
            move |(_, _, _, owns_canvas)| {
                if !owns_canvas {
                    canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
                }
            }
        },
    ));
    PartPlacementMount {
        projection,
        busy,
        error: error(),
        on_choose_controller: EventHandler::new(on_choose_controller),
        on_place_controller: EventHandler::new(on_place_controller),
        controller_placement_enabled: ordinary_controller || guided_controller,
        controller_back,
        on_place_component: EventHandler::new(on_place_component),
        on_move,
        on_commit,
        on_cancel: EventHandler::new(on_cancel),
    }
}

fn placement_route_is_current(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    owner: &PlacementOwner,
    generation: u64,
    workspace: &'static str,
    preferences: Option<SetupGuidePreferences>,
) -> bool {
    model.accepted.as_ref().is_some_and(|accepted| {
        accepted.session_epoch == owner.session_epoch
            && accepted.document.id == owner.project_id
            && model.active_board_id == owner.board_id
            && model.active_instance_id == owner.scope.instance_id
            && runtime.scope().as_ref() == Some(&owner.scope)
            && generation == owner.generation
            && workspace == "Layout"
    }) && (owner.workflow != PlacementWorkflow::WiringController
        || preferences.as_ref().is_some_and(|preferences| {
            preferences.open
                && preferences.project_id == owner.project_id
                && preferences.current_stage == SetupGuideStage::Wiring
        }))
}

fn browser_uuid() -> Result<String, String> {
    let crypto = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("crypto"))
        .map_err(|error| format!("Could not access browser identity service: {error:?}"))?;
    let random_uuid = js_sys::Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
        .map_err(|error| format!("Could not access browser identity service: {error:?}"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "The browser cannot generate a safe component identity.".to_string())?;
    random_uuid
        .call0(&crypto)
        .map_err(|error| format!("Could not generate a component identity: {error:?}"))?
        .as_string()
        .ok_or_else(|| "Could not generate a component identity.".into())
}

pub struct LayoutPlacementCancellation {
    pub parts: PartPlacementMount,
    pub matrices: crate::objects::MatrixPlacementMount,
    pub interactions: CanvasInteractionArbiter,
}

pub fn layout_view_mode_handler(
    is_owner_current: impl Fn() -> bool + 'static,
    is_assembly_3d: Signal<bool>,
    mut set_assembly_3d: impl FnMut(bool) + 'static,
    placements: LayoutPlacementCancellation,
    before_placement_cancel: impl Fn() + 'static,
    mut after_placement_cancel: impl FnMut() + 'static,
) -> EventHandler<bool> {
    EventHandler::new(move |assembly_3d| {
        if !is_owner_current() {
            return;
        }
        if assembly_3d && !is_assembly_3d() {
            before_placement_cancel();
            if placements.parts.busy || placements.parts.projection.is_some() {
                placements.parts.on_cancel.call(());
            }
            after_placement_cancel();
            match placements.interactions.current() {
                Some(CanvasInteractionOwner::PartPlacement) => {
                    placements
                        .interactions
                        .release(CanvasInteractionOwner::PartPlacement);
                }
                Some(CanvasInteractionOwner::OutlinePerimeter) => {
                    placements
                        .interactions
                        .release(CanvasInteractionOwner::OutlinePerimeter);
                }
                Some(CanvasInteractionOwner::MatrixPlacement) => {
                    if let Some(owner) = placements.matrices.cancel_owner.clone() {
                        placements.matrices.on_cancel.call(owner);
                    }
                }
                Some(CanvasInteractionOwner::MatrixTransform) => {
                    placements
                        .interactions
                        .release(CanvasInteractionOwner::MatrixTransform);
                }
                Some(CanvasInteractionOwner::MirroredPair) | None => {}
            }
        }
        set_assembly_3d(assembly_3d);
    })
}
