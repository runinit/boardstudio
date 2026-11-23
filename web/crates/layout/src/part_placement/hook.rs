//! Browser placement hook: owns the interaction lifetime, definition loading and
//! commit/cancel routing for the canvas. Identity and proposal logic live in
//! `super::owner` and `super::plan`.
use super::owner::{
    ActivePartPlacement, ComponentActionOwner, ControllerChooserOwner, PlacementAdmission,
    PlacementCapture, PlacementOwner, PlacementWorkflow, accepted_snapshot_is_current,
    key_component_resolver, placement_resolver,
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
use boardstudio_web_runtime::operation_outcomes::Settlement;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
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
    pub canvas_interaction: CanvasInteractionArbiter,
}

pub type DefinitionLoader = Rc<
    dyn Fn(
        ProjectDoc,
        String,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PartDefinition, String>>>>,
>;

mod commit;
mod start;

#[derive(Clone, Copy)]
struct PlacementState {
    chooser: Signal<Option<ControllerChooserOwner>>,
    active: Signal<Option<ActivePartPlacement>>,
    preparing: Signal<Option<PlacementOwner>>,
    committing: Signal<bool>,
    error: Signal<Option<String>>,
}

pub fn use_controller_placement(host: PartPlacementHost) -> PartPlacementMount {
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
    let start_placement = Rc::new(RefCell::new(start::start_placement(&host, state, alive)));
    let on_commit = commit::commit_placement(&host, state);
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
        canvas_interaction,
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
            let canvas_interaction = canvas_interaction.clone();
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
            canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
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
