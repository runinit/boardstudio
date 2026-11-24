//! Catalogue component placement: the browser session owns the interaction
//! lifetime, definition loading and commit/cancel routing for the canvas.
//! `session` keeps the identity checks, `plan` keeps the atomic document
//! proposal independent of the canvas DOM. `plan` also compiles natively so
//! its unit tests run under `cargo test`.

pub mod plan;

#[cfg(any(target_arch = "wasm32", test))]
mod session;

pub use plan::{
    PendingPart, PlacementSnapOptions, canvas_world_center, component_part, controller_part,
    next_component_reference, placement_gap, placement_operation, pointer_release_commits,
    snap_placement_at, update_pending_part,
};
#[cfg(target_arch = "wasm32")]
pub use session::ComponentPlacementAction;
#[cfg(any(target_arch = "wasm32", test))]
pub use session::{
    ActivePartPlacement, PlacementOwner, PlacementWorkflow, selected_key_component_operation,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use plan::PlacementIntent;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use session::PlacementCapture;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod key_component_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod plan_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;

#[cfg(target_arch = "wasm32")]
use self::plan::{
    PlacementIntent, document_with_pending_definition, grid_snap_point, placement_source_definition,
};
#[cfg(target_arch = "wasm32")]
use self::session::{
    ComponentActionOwner, ControllerChooserOwner, PlacementAdmission, PlacementCapture,
    accepted_snapshot_is_current, key_component_resolver, placement_resolver,
};
#[cfg(target_arch = "wasm32")]
use crate::canvas_interaction::{CanvasMode, LayoutInteraction, PartPlacementPayload};
#[cfg(target_arch = "wasm32")]
use crate::objects::{self, LayoutSnapSettings};
#[cfg(target_arch = "wasm32")]
use crate::parts::{PartsQuery, PartsSelection};
#[cfg(target_arch = "wasm32")]
use crate::runtime::Runtime;
#[cfg(target_arch = "wasm32")]
use crate::selection::SelectionAdapter;
#[cfg(target_arch = "wasm32")]
use crate::setup_guide::{SetupGuidePreferences, SetupGuideStage};
#[cfg(target_arch = "wasm32")]
use boardstudio_application::AcceptedSnapshot;
#[cfg(target_arch = "wasm32")]
use boardstudio_application::{Durability, Event, Lifecycle, SelectionMode};
#[cfg(target_arch = "wasm32")]
use boardstudio_model::model::{PartDefinition, PartKind, ProjectDoc, Vec2};
#[cfg(target_arch = "wasm32")]
#[cfg(target_arch = "wasm32")]
use boardstudio_web_runtime::operation_outcomes::Settlement;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::Workspace;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
#[cfg(target_arch = "wasm32")]
pub struct PartPlacementSession {
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

#[cfg(target_arch = "wasm32")]
pub struct PartPlacementHost {
    pub runtime: Rc<Runtime>,
    pub load_definition: DefinitionLoader,
    pub workspace: Signal<Workspace>,
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
}

#[cfg(target_arch = "wasm32")]
pub type DefinitionLoader = Rc<
    dyn Fn(
        ProjectDoc,
        String,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PartDefinition, String>>>>,
>;

#[derive(Clone, Copy)]
#[cfg(target_arch = "wasm32")]
pub(crate) struct PlacementState {
    pub(crate) chooser: Signal<Option<ControllerChooserOwner>>,
    pub(crate) active: Signal<Option<ActivePartPlacement>>,
    pub(crate) preparing: Signal<Option<PlacementOwner>>,
    pub(crate) committing: Signal<bool>,
    pub(crate) error: Signal<Option<String>>,
}

#[cfg(target_arch = "wasm32")]
pub fn use_part_placement(host: PartPlacementHost) -> PartPlacementSession {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let chooser = use_signal(|| None::<ControllerChooserOwner>);
    let active = use_signal(|| None::<ActivePartPlacement>);
    let preparing = use_signal(|| None::<PlacementOwner>);
    let committing = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    let state = PlacementState {
        chooser,
        active,
        preparing,
        committing,
        error,
    };
    let start_placement = Rc::new(RefCell::new(start_placement(&host, state, alive)));
    let on_commit = commit_placement(&host, state);
    let PartPlacementHost {
        runtime,
        workspace,
        generation,
        version,
        adapter,
        guide_preferences,
        parts_query,
        parts_selection,
        mut objects_open,
        mut inspect_open,
        ..
    } = host;

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
            if !guided && owner.workspace != Workspace::Pcb {
                return;
            }
            chooser.set((!guided).then(|| ControllerChooserOwner {
                accepted: owner.accepted.clone(),
                scope: owner.scope.clone(),
                generation: owner.generation,
            }));
            workspace.set(if guided {
                Workspace::Layout
            } else {
                Workspace::Parts
            });
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
                        Workspace::Parts,
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

            EventHandler::new(move |_| {
                if chooser.read().as_ref().is_some_and(|owner| {
                    owner.is_current(runtime.as_ref(), generation(), workspace())
                }) && canvas_interaction.current().is_none()
                {
                    chooser.set(None);
                    workspace.set(Workspace::Pcb);
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
                    let Some(owner) = action_owner.as_ref().filter(|owner| {
                        matches!(owner.workspace, Workspace::Layout | Workspace::Pcb)
                    }) else {
                        return;
                    };
                    owner.workspace
                }
                ComponentPlacementAction::PartsInspector { .. } => Workspace::Parts,
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
                    Workspace::Layout,
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

        move |at: Vec2| {
            if canvas_interaction.current() != Some(CanvasMode::PlacingPart) {
                return;
            }
            if let Some(mut placement) = active() {
                update_pending_part(&mut placement.pending, at);
                active.set(Some(placement));
            }
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

        move |_| {
            if *committing.read() {
                return;
            }
            let owner_and_workspace = active
                .read()
                .as_ref()
                .map(|placement| (placement.owner.clone(), Workspace::Layout))
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
            canvas_interaction.end(CanvasMode::PlacingPart);
            if current {
                selected_context.set(None);
                anchor_scope.set(None);
                workspace.set(if owner.workflow == PlacementWorkflow::PcbController {
                    Workspace::Pcb
                } else {
                    Workspace::Layout
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
            Workspace::Layout,
            guide_preferences(),
        )
        .admits(&placement.owner, &runtime, &model)
    });
    let busy = preparing.read().is_some() || *committing.read();
    let owns_canvas = busy || projection.is_some();
    use_effect(use_reactive(
        (&version(), &workspace(), &generation(), &owns_canvas),
        {
            move |(_, _, _, owns_canvas)| {
                if !owns_canvas {
                    canvas_interaction.end(CanvasMode::PlacingPart);
                }
            }
        },
    ));
    PartPlacementSession {
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

#[cfg(target_arch = "wasm32")]
pub(crate) fn placement_route_is_current(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    owner: &PlacementOwner,
    generation: u64,
    workspace: Workspace,
    preferences: Option<SetupGuidePreferences>,
) -> bool {
    model.accepted.as_ref().is_some_and(|accepted| {
        accepted.session_epoch == owner.session_epoch
            && accepted.document.id == owner.project_id
            && model.active_board_id == owner.board_id
            && model.active_instance_id == owner.scope.instance_id
            && runtime.scope().as_ref() == Some(&owner.scope)
            && generation == owner.generation
            && workspace == Workspace::Layout
    }) && (owner.workflow != PlacementWorkflow::WiringController
        || preferences.as_ref().is_some_and(|preferences| {
            preferences.open
                && preferences.project_id == owner.project_id
                && preferences.current_stage == SetupGuideStage::Wiring
        }))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn browser_uuid() -> Result<String, String> {
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

/// Admit and prepare a new catalogue placement.
#[cfg(target_arch = "wasm32")]
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
    let canvas_interaction = use_context::<LayoutInteraction>();
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
            .and_then(|selected| {
                crate::objects::matrix_id_for_context(&selected.context).map(str::to_owned)
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
        if !canvas_interaction.begin_part_placement(PartPlacementPayload {
            part_id: owner.part_id.clone(),
            definition_id: owner.definition_id.clone(),
        }) {
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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

/// Commit the captured placement and apply its workspace follow-up.
#[cfg(target_arch = "wasm32")]
pub(super) fn commit_placement(
    host: &PartPlacementHost,
    state: PlacementState,
) -> impl FnMut(Vec2) + use<> {
    let runtime = host.runtime.clone();
    let PartPlacementHost {
        mut workspace,
        generation,
        mut layout_selection_kind,
        guide_preferences,
        ..
    } = *host;
    let PlacementState {
        mut active,
        mut committing,
        mut error,
        ..
    } = state;
    let mut adapter = host.adapter.clone();
    let canvas_interaction = use_context::<LayoutInteraction>();
    move |at: Vec2| {
        if canvas_interaction.current() != Some(CanvasMode::PlacingPart) {
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
            Workspace::Layout,
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
        committing.set(true);
        let outcome = runtime.resolve_edit(
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
        active.set(None);
        error.set(None);
        let route_live = runtime.with_model(|model| {
            placement_route_is_current(
                &runtime,
                model,
                &owner,
                generation(),
                workspace(),
                guide_preferences(),
            )
        });
        let settlement = outcome.settlement(route_live);
        committing.set(false);
        let model = runtime.model();
        match settlement {
            Settlement::Landed { .. } => {
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
                adapter.selected_context.set(next_context);
                adapter.anchor_scope.set(None);
                workspace.set(if owner.workflow == PlacementWorkflow::WiringController {
                    Workspace::Layout
                } else if owner.workflow.is_controller() {
                    Workspace::Pcb
                } else {
                    Workspace::Layout
                });
                if owner.workflow == PlacementWorkflow::GeneralComponent {
                    layout_selection_kind.set(objects::LayoutSelectionKind::Part);
                }
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: if owner.workflow == PlacementWorkflow::GeneralComponent && placed {
                        vec![owner.part_id.clone()]
                    } else {
                        Vec::new()
                    },
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
            }
            Settlement::Failed { message, .. } => {
                workspace.set(owner.source_workspace);
                error.set(Some(message));
            }
            Settlement::Retired => {}
        }
        canvas_interaction.end(CanvasMode::PlacingPart);
    }
}
