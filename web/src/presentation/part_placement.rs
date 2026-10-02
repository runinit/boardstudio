//! Private project-scoped placement planning for catalogue components.
//!
//! The browser controller owns the interaction lifetime; this module keeps the
//! identity checks and atomic document proposal independent of the canvas DOM.
use super::{
    objects::LayoutSnapSettings,
    parts::{PartsQuery, PartsSelection},
    selection::SelectionAdapter,
    setup_guide::{SetupGuidePreferences, SetupGuideStage},
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, SelectionMode, SnapshotToken,
    TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, OutlineFeature, Part, PartDefinition, PartKind, Pose2,
    ProjectDoc, Side, Vec2,
};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PlacementOwner {
    pub(super) token: SnapshotToken,
    pub(super) session_epoch: boardstudio_application::SessionEpoch,
    pub(super) project_id: String,
    pub(super) revision: u64,
    pub(super) scope: Scope,
    pub(super) generation: u64,
    pub(super) board_id: String,
    pub(super) part_id: String,
    pub(super) definition_id: String,
    pub(super) reference: String,
    pub(super) layout_id: Option<String>,
    pub(super) at: Vec2,
}

impl PlacementOwner {
    fn record_committed_position(&mut self, at: Vec2) {
        self.at = at;
    }

    pub(super) fn capture(
        snapshot: &AcceptedSnapshot,
        scope: Scope,
        generation: u64,
        part_id: String,
        definition_id: String,
        layout_id: Option<String>,
        at: Vec2,
    ) -> Option<Self> {
        if snapshot.document.id != scope.document_id
            || snapshot.session_epoch != scope.session_epoch
            || !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
        {
            return None;
        }
        let reference = next_controller_reference(&snapshot.document);
        let board_id = scope.board_id.clone();
        Some(Self {
            token: snapshot.token,
            session_epoch: snapshot.session_epoch,
            project_id: snapshot.document.id.clone(),
            revision: snapshot.document.revision,
            scope,
            generation,
            board_id,
            part_id,
            definition_id,
            reference,
            layout_id,
            at,
        })
    }

    pub(super) fn is_current(
        &self,
        snapshot: &AcceptedSnapshot,
        scope: Option<&Scope>,
        generation: u64,
        guide_open: bool,
        guide_project_id: Option<&str>,
        guide_stage_is_wiring: bool,
    ) -> bool {
        scope == Some(&self.scope)
            && generation == self.generation
            && snapshot.token == self.token
            && snapshot.session_epoch == self.session_epoch
            && snapshot.document.id == self.project_id
            && snapshot.document.revision == self.revision
            && self.board_id == self.scope.board_id
            && guide_open
            && guide_project_id == Some(self.project_id.as_str())
            && guide_stage_is_wiring
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PendingPart {
    pub(super) definition: PartDefinition,
    pub(super) part: Part,
    pub(super) at: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ActivePartPlacement {
    pub(super) owner: PlacementOwner,
    pub(super) pending: PendingPart,
}

#[derive(Clone)]
struct PendingCommit {
    owner: PlacementOwner,
    operation_id: boardstudio_application::OperationId,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

#[derive(Clone)]
pub(super) struct PartPlacementMount {
    pub(super) projection: Option<ActivePartPlacement>,
    pub(super) busy: bool,
    pub(super) error: Option<String>,
    pub(super) on_choose_controller: EventHandler<()>,
    pub(super) on_place_controller: EventHandler<String>,
    pub(super) on_move: EventHandler<Vec2>,
    pub(super) on_commit: EventHandler<Vec2>,
    pub(super) on_cancel: EventHandler<()>,
}

pub(super) struct PartPlacementHost {
    pub(super) runtime: Rc<Runtime>,
    pub(super) workspace: Signal<&'static str>,
    pub(super) generation: Signal<u64>,
    pub(super) adapter: SelectionAdapter,
    pub(super) guide_preferences: Signal<Option<SetupGuidePreferences>>,
    pub(super) parts_query: PartsQuery,
    pub(super) parts_selection: PartsSelection,
    pub(super) snap_settings: Signal<LayoutSnapSettings>,
    pub(super) objects_open: Signal<bool>,
    pub(super) inspect_open: Signal<bool>,
}

struct PlacementAdmission {
    scope: Option<Scope>,
    generation: u64,
    workspace: &'static str,
    expected_workspace: &'static str,
    preferences: Option<SetupGuidePreferences>,
}

impl PlacementAdmission {
    fn capture(
        runtime: &Runtime,
        generation: u64,
        workspace: &'static str,
        expected_workspace: &'static str,
        preferences: Option<SetupGuidePreferences>,
    ) -> Self {
        Self {
            scope: runtime.scope(),
            generation,
            workspace,
            expected_workspace,
            preferences,
        }
    }
}

pub(super) fn use_controller_placement(host: PartPlacementHost) -> PartPlacementMount {
    let PartPlacementHost {
        runtime,
        workspace,
        generation,
        adapter,
        guide_preferences,
        parts_query,
        parts_selection,
        snap_settings,
        mut objects_open,
        mut inspect_open,
    } = host;
    let active = use_signal(|| None::<ActivePartPlacement>);
    let preparing = use_signal(|| None::<PlacementOwner>);
    let committing = use_signal(|| None::<PendingCommit>);
    let error = use_signal(|| None::<String>);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    let on_choose_controller = {
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut query = parts_query;
        let mut selected = parts_selection;
        let guide = guide_preferences;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        move |_| {
            let model = runtime.model();
            let Some(project_id) = model
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.id.clone())
            else {
                return;
            };
            if !guide().as_ref().is_some_and(|preferences| {
                preferences.project_id == project_id
                    && preferences.open
                    && preferences.current_stage == SetupGuideStage::Wiring
            }) {
                return;
            }
            workspace.set("Parts");
            query.set("controller".into());
            selected.set(None);
            selected_context.set(None);
            anchor_scope.set(None);
            objects_open.set(true);
            inspect_open.set(false);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
        }
    };

    let on_place_controller = {
        let runtime = runtime.clone();
        let query = parts_query;
        let mut preparing = preparing;
        let mut error = error;
        let alive = alive.clone();
        let adapter_for_async = adapter.clone();
        move |definition_id: String| {
            if active.read().is_some()
                || preparing.read().is_some()
                || committing.read().is_some()
                || workspace() != "Parts"
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
            let guide_is_live = guide_preferences().as_ref().is_some_and(|preferences| {
                preferences.open
                    && preferences.current_stage == SetupGuideStage::Wiring
                    && preferences.project_id == snapshot.document.id
            });
            if !guide_is_live
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
            let part_id = match browser_uuid() {
                Ok(id) => format!("ui-{id}"),
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let at = grid_snap_point(model.camera.center, snap_settings.read().snap_fraction);
            let Some(owner) = PlacementOwner::capture(
                snapshot,
                scope.clone(),
                generation(),
                part_id,
                definition_id.clone(),
                None,
                at,
            ) else {
                return;
            };
            preparing.set(Some(owner.clone()));
            error.set(None);
            let accepted = snapshot.clone();
            let reversible_document = snapshot.document.clone();
            let runtime = runtime.clone();
            let mut workspace = workspace;
            let mut active = active;
            let mut error = error;
            let alive = alive.clone();
            let mut adapter = adapter_for_async.clone();
            let guide = guide_preferences;
            let mut query = query;
            let mut preparing = preparing;
            spawn_local(async move {
                let definition = crate::presentation::parts::load_controller_definition(
                    &reversible_document,
                    &definition_id,
                )
                .await;
                if !alive.get() {
                    return;
                }
                if preparing.read().as_ref() != Some(&owner) {
                    return;
                }
                let model = runtime.model();
                let still_owned = owner_is_live(
                    &owner,
                    &runtime,
                    &model,
                    PlacementAdmission::capture(
                        &runtime,
                        generation(),
                        workspace(),
                        "Parts",
                        guide(),
                    ),
                );
                if !still_owned
                    || workspace() != "Parts"
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
                let Some(pending) = controller_part(
                    definition,
                    owner.part_id.clone(),
                    owner.reference.clone(),
                    owner.at,
                ) else {
                    preparing.set(None);
                    error.set(Some(
                        "The selected catalogue item is not a controller.".into(),
                    ));
                    return;
                };
                if !owner_is_live(
                    &owner,
                    &runtime,
                    &model,
                    PlacementAdmission::capture(
                        &runtime,
                        generation(),
                        workspace(),
                        "Parts",
                        guide(),
                    ),
                ) {
                    preparing.set(None);
                    return;
                }
                active.set(Some(ActivePartPlacement { owner, pending }));
                preparing.set(None);
                query.set("controller".into());
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
        }
    };

    let on_move = {
        let mut active = active;
        move |at: Vec2| {
            if let Some(mut placement) = active() {
                update_pending_part(&mut placement.pending, at);
                active.set(Some(placement));
            }
        }
    };
    let on_commit = {
        let runtime = runtime.clone();
        let mut active = active;
        let mut committing = committing;
        let mut error = error;
        let alive = alive.clone();
        move |at: Vec2| {
            let Some(mut placement) = active() else {
                return;
            };
            if committing.read().is_some() {
                return;
            }
            update_pending_part(&mut placement.pending, at);
            placement
                .owner
                .record_committed_position(placement.pending.at);
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !owner_is_live(
                &placement.owner,
                &runtime,
                &model,
                PlacementAdmission::capture(
                    &runtime,
                    generation(),
                    workspace(),
                    "Layout",
                    guide_preferences(),
                ),
            ) || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: placement.owner.revision,
                    })
            {
                return;
            }
            let operation = match placement_operation(
                &snapshot.document,
                &placement.owner.board_id,
                &placement.pending.definition,
                &placement.pending.part,
                placement.owner.layout_id.as_deref(),
            ) {
                Ok(operation) => operation,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let owner = placement.owner.clone();
            let observed = outcome.clone();
            committing.set(Some(PendingCommit {
                owner: owner.clone(),
                operation_id,
                outcome,
            }));
            active.set(None);
            error.set(None);
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id: format!("controller-place-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![owner.part_id.clone()],
                    operation,
                },
            });
            let runtime = runtime.clone();
            let mut committing = committing;
            let mut error = error;
            let mut workspace = workspace;
            let guide_preferences = guide_preferences;
            let mut selected_context = adapter.selected_context;
            let mut anchor_scope = adapter.anchor_scope;
            let alive = alive.clone();
            spawn_local(async move {
                while observed.borrow().is_none() {
                    gloo_timers::future::TimeoutFuture::new(16).await;
                }
                let terminal = observed.borrow().clone();
                if !alive.get() {
                    return;
                }
                let operation_matches = committing.read().as_ref().is_some_and(|pending| {
                    pending.operation_id == operation_id
                        && pending.owner == owner
                        && Rc::ptr_eq(&pending.outcome, &observed)
                });
                if terminal == Some(TerminalOutcome::Completed) {
                    for _ in 0..500 {
                        let model = runtime.model();
                        if let Some(accepted) = model.accepted.as_ref()
                            && completion_is_accepted(
                                operation_matches,
                                terminal.as_ref(),
                                &model.lifecycle,
                                &model.durability,
                                Some(accepted),
                                &owner,
                                &owner.part_id,
                            )
                        {
                            break;
                        }
                        if !alive.get() {
                            break;
                        }
                        gloo_timers::future::TimeoutFuture::new(16).await;
                    }
                }
                if !alive.get() {
                    return;
                }
                let model = runtime.model();
                let success = model.accepted.as_ref().is_some_and(|accepted| {
                    completion_is_accepted(
                        operation_matches,
                        terminal.as_ref(),
                        &model.lifecycle,
                        &model.durability,
                        Some(accepted),
                        &owner,
                        &owner.part_id,
                    )
                });
                let route_live = model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.document.id == owner.project_id
                        && model.active_board_id == owner.board_id
                        && model.active_instance_id == owner.scope.instance_id
                        && runtime.scope().as_ref() == Some(&owner.scope)
                        && generation() == owner.generation
                        && workspace() == "Layout"
                }) && guide_preferences().as_ref().is_some_and(|preferences| {
                    preferences.open
                        && preferences.project_id == owner.project_id
                        && preferences.current_stage == SetupGuideStage::Wiring
                });
                committing.set(None);
                if success && route_live {
                    selected_context.set(None);
                    anchor_scope.set(None);
                    workspace.set("PCB");
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: Vec::new(),
                        range_part_ids: Vec::new(),
                        mode: SelectionMode::Replace,
                    });
                } else if route_live {
                    workspace.set("Parts");
                    error.set(Some(match terminal {
                        Some(
                            TerminalOutcome::Rejected(message)
                            | TerminalOutcome::PersistenceFailed(message)
                            | TerminalOutcome::ExecutorFailed(message)
                            | TerminalOutcome::BlockedByRecovery(message),
                        ) => message,
                        Some(TerminalOutcome::Completed) => {
                            "The controller was not accepted and saved in the active project."
                                .into()
                        }
                        _ => "Controller placement was cancelled or superseded.".into(),
                    }));
                }
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
        move |_| {
            if committing.read().is_some() {
                return;
            }
            let owner_and_workspace = active
                .read()
                .as_ref()
                .map(|placement| (placement.owner.clone(), "Layout"))
                .or_else(|| preparing.read().clone().map(|owner| (owner, "Parts")));
            let Some((owner, owner_workspace)) = owner_and_workspace else {
                return;
            };
            let model = runtime.model();
            let current = model.accepted.as_ref().is_some_and(|_snapshot| {
                owner_is_live(
                    &owner,
                    &runtime,
                    &model,
                    PlacementAdmission::capture(
                        &runtime,
                        generation(),
                        workspace(),
                        owner_workspace,
                        guide(),
                    ),
                )
            });
            active.set(None);
            preparing.set(None);
            if current {
                selected_context.set(None);
                anchor_scope.set(None);
                workspace.set("PCB");
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
        owner_is_live(
            &placement.owner,
            &runtime,
            &model,
            PlacementAdmission::capture(
                &runtime,
                generation(),
                workspace(),
                "Layout",
                guide_preferences(),
            ),
        )
    });
    PartPlacementMount {
        projection,
        busy: preparing.read().is_some() || committing.read().is_some(),
        error: error(),
        on_choose_controller: EventHandler::new(on_choose_controller),
        on_place_controller: EventHandler::new(on_place_controller),
        on_move,
        on_commit,
        on_cancel: EventHandler::new(on_cancel),
    }
}

fn owner_is_live(
    owner: &PlacementOwner,
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    admission: PlacementAdmission,
) -> bool {
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    owner.is_current(
        snapshot,
        admission.scope.as_ref(),
        admission.generation,
        admission
            .preferences
            .as_ref()
            .is_some_and(|preferences| preferences.open),
        admission
            .preferences
            .as_ref()
            .map(|preferences| preferences.project_id.as_str()),
        admission
            .preferences
            .as_ref()
            .is_some_and(|preferences| preferences.current_stage == SetupGuideStage::Wiring),
    ) && model.active_board_id == owner.board_id
        && runtime.scope().as_ref() == Some(&owner.scope)
        && admission.workspace == admission.expected_workspace
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

pub(super) fn controller_part(
    definition: PartDefinition,
    part_id: String,
    reference: String,
    at: Vec2,
) -> Option<PendingPart> {
    matches!(definition.kind, PartKind::Controller).then(|| PendingPart {
        part: Part {
            keycap: None,
            outline: None,
            id: part_id,
            definition_id: definition.id.clone(),
            reference,
            pose: Pose2 { at, rotation: 0.0 },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: definition
                .generator
                .as_ref()
                .map(|generator| generator.parameters.clone()),
        },
        definition,
        at,
    })
}

pub(super) fn update_pending_part(pending: &mut PendingPart, at: Vec2) {
    pending.at = at;
    pending.part.pose.at = at;
}

fn grid_snap_point(at: Vec2, snap_fraction: f64) -> Vec2 {
    if snap_fraction == 0.0 {
        return at;
    }
    let step = if snap_fraction.is_finite() {
        if snap_fraction < 0.0 {
            -snap_fraction
        } else {
            19.05 * snap_fraction
        }
    } else {
        19.05 * 0.25
    }
    .max(0.001);
    Vec2 {
        x: (at.x / step).round() * step,
        y: (at.y / step).round() * step,
    }
}

pub(super) struct PlacementSnapOptions {
    pub(super) snap_fraction: f64,
    pub(super) geometry_snap: bool,
    pub(super) gap_snap: bool,
    pub(super) free: bool,
}

pub(super) fn snap_placement_at(
    document: &ProjectDoc,
    board_id: &str,
    pending: &PendingPart,
    at: Vec2,
    options: PlacementSnapOptions,
) -> Vec2 {
    if options.free {
        return at;
    }
    let grid = grid_snap_point(at, options.snap_fraction);
    if !options.geometry_snap {
        return grid;
    }
    let mut moving = pending.part.clone();
    moving.pose.at = at;
    boardstudio_application::interactions::snap_part_in_board(
        document,
        board_id,
        &moving,
        2.0,
        options.gap_snap.then_some(1.0),
    )
    .map_or(grid, |guide| guide.at)
}

pub(super) fn placement_operation(
    accepted: &ProjectDoc,
    board_id: &str,
    definition: &PartDefinition,
    part: &Part,
    layout_id: Option<&str>,
) -> Result<EditOperation, String> {
    if !matches!(definition.kind, PartKind::Controller) {
        return Err("Only controller definitions can be placed from the setup guide.".into());
    }
    if part.definition_id != definition.id || part.id.is_empty() || part.reference.is_empty() {
        return Err("The prepared controller placement is incomplete.".into());
    }
    if accepted.parts.iter().any(|existing| existing.id == part.id) {
        return Err("The controller placement identity already exists.".into());
    }
    let board_index = accepted
        .boards
        .iter()
        .position(|board| board.id == board_id)
        .ok_or_else(|| "The selected board is no longer available.".to_string())?;
    if accepted
        .parts
        .iter()
        .any(|existing| existing.reference == part.reference)
    {
        return Err("The controller reference is already in use.".into());
    }

    let mut proposed = accepted.clone();
    match proposed
        .definitions
        .iter()
        .find(|existing| existing.id == definition.id)
    {
        Some(existing) if existing != definition => {
            return Err("The selected controller definition changed before placement.".into());
        }
        Some(_) => {}
        None => proposed.definitions.push(definition.clone()),
    }
    proposed.parts.push(part.clone());
    let board = &mut proposed.boards[board_index];
    if !board.part_ids.contains(&part.id) {
        board.part_ids.push(part.id.clone());
    }
    let board_outlines = board
        .outline_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for outline in &mut proposed.outline {
        if let OutlineFeature::PartEnvelope { id, part_ids, .. } = outline
            && board_outlines.contains(id)
            && !part_ids.contains(&part.id)
        {
            part_ids.push(part.id.clone());
        }
    }
    if let Some(layout_id) = layout_id {
        let layout = proposed
            .layouts
            .iter_mut()
            .find(|layout| layout.id == layout_id && layout.board_id == board_id)
            .ok_or_else(|| {
                "The selected layout is no longer available on this board.".to_string()
            })?;
        if !layout.part_ids.contains(&part.id) {
            layout.part_ids.push(part.id.clone());
        }
    }
    Ok(EditOperation::ReplaceDocument {
        document: Box::new(proposed),
    })
}

pub(super) fn next_controller_reference(document: &ProjectDoc) -> String {
    let occupied = document
        .parts
        .iter()
        .filter_map(|part| part.reference.strip_prefix('U'))
        .filter_map(|number| number.parse::<u32>().ok())
        .collect::<std::collections::BTreeSet<_>>();
    let next = (1..=u32::MAX)
        .find(|candidate| !occupied.contains(candidate))
        .unwrap_or(1);
    format!("U{next}")
}

pub(super) fn completion_is_accepted(
    operation_matches: bool,
    outcome: Option<&TerminalOutcome>,
    model_lifecycle: &Lifecycle,
    durability: &Durability,
    accepted: Option<&AcceptedSnapshot>,
    owner: &PlacementOwner,
    part_id: &str,
) -> bool {
    if !operation_matches || outcome != Some(&TerminalOutcome::Completed) {
        return false;
    }
    let Some(snapshot) = accepted else {
        return false;
    };
    let Some(board) = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == owner.board_id)
    else {
        return false;
    };
    let envelope_membership_matches = snapshot.document.outline.iter().all(|outline| {
        let OutlineFeature::PartEnvelope { id, part_ids, .. } = outline else {
            return true;
        };
        !board.outline_ids.contains(id) || part_ids.iter().any(|id| id == part_id)
    });
    let layout_membership_matches = owner.layout_id.as_ref().is_none_or(|layout_id| {
        snapshot.document.layouts.iter().any(|layout| {
            layout.id == *layout_id
                && layout.board_id == owner.board_id
                && layout.part_ids.iter().any(|id| id == part_id)
        })
    });
    *model_lifecycle == Lifecycle::Ready
        && *durability
            == (Durability::Saved {
                revision: snapshot.document.revision,
            })
        && snapshot.document.id == owner.project_id
        && snapshot.session_epoch == owner.session_epoch
        && snapshot
            .document
            .definitions
            .iter()
            .any(|definition| definition.id == owner.definition_id)
        && board.part_ids.iter().any(|id| id == part_id)
        && envelope_membership_matches
        && layout_membership_matches
        && snapshot.document.parts.iter().any(|part| {
            part.id == part_id
                && part.definition_id == owner.definition_id
                && part.reference == owner.reference
                && part.pose.at == owner.at
                && part.pose.rotation == 0.0
                && part.side == Side::Front
        })
}

#[cfg(test)]
mod tests {
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{Board, OutlineSettings};
    use std::sync::Arc;

    fn controller_definition(id: &str) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": "Controller",
            "kind": "controller",
            "courtyard": [{"x": -5.0, "y": -5.0}, {"x": 5.0, "y": -5.0}, {"x": 5.0, "y": 5.0}],
            "pads": []
        }))
        .unwrap()
    }

    fn fixture() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Keyboard");
        document.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: OutlineSettings::default(),
            id: "envelope-main".into(),
            part_ids: vec![],
            margin: 4.0,
            operation: boardstudio_core::model::Operation::Add,
        });
        document.boards.push(Board {
            id: "board-main".into(),
            name: "Main".into(),
            outline_ids: vec!["envelope-main".into()],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    fn accepted(document: ProjectDoc, token: u64) -> AcceptedSnapshot {
        let revision = document.revision;
        AcceptedSnapshot {
            token: SnapshotToken(token),
            session_epoch: SessionEpoch(7),
            document: Arc::new(document),
            scene: Arc::new(boardstudio_core::model::SceneDelta {
                module_scenes: vec![],
                revision,
                transaction_id: "controller-placement-test".into(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: boardstudio_core::model::Readiness {
                    layout: false,
                    outline: false,
                    pcb: false,
                    case_ready: false,
                },
            }),
        }
    }

    fn part(definition_id: &str, id: &str, reference: &str) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: definition_id.into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x: 12.0, y: -8.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn replacement(operation: EditOperation) -> ProjectDoc {
        let EditOperation::ReplaceDocument { document } = operation else {
            panic!("placement must be one ReplaceDocument edit");
        };
        *document
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn controller_creation_is_one_document_edit_with_board_and_envelope_membership() {
        let accepted = fixture();
        let definition = controller_definition("catalog:controller");
        let placed = part(&definition.id, "part-controller", "U1");

        let proposed = replacement(
            placement_operation(&accepted, "board-main", &definition, &placed, None).unwrap(),
        );

        assert_eq!(proposed.revision, accepted.revision);
        assert_eq!(proposed.definitions, vec![definition]);
        assert_eq!(proposed.parts, vec![placed]);
        assert_eq!(proposed.boards[0].part_ids, vec!["part-controller"]);
        let OutlineFeature::PartEnvelope { part_ids, .. } = &proposed.outline[0] else {
            panic!("fixture outline is an envelope");
        };
        assert_eq!(part_ids, &["part-controller"]);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn controller_creation_adds_selected_layout_membership_and_keeps_other_layouts_untouched() {
        let mut accepted = fixture();
        accepted.layouts = vec![
            boardstudio_core::model::Layout {
                id: "layout-main".into(),
                name: "Main".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-main".into(),
                part_ids: vec![],
                mirror_link: None,
            },
            boardstudio_core::model::Layout {
                id: "layout-other".into(),
                name: "Other".into(),
                board_id: "board-main".into(),
                matrix_id: "matrix-other".into(),
                part_ids: vec![],
                mirror_link: None,
            },
        ];
        let definition = controller_definition("catalog:controller");
        let placed = part(&definition.id, "part-controller", "U1");

        let proposed = replacement(
            placement_operation(
                &accepted,
                "board-main",
                &definition,
                &placed,
                Some("layout-main"),
            )
            .unwrap(),
        );

        assert_eq!(proposed.layouts[0].part_ids, vec!["part-controller"]);
        assert!(proposed.layouts[1].part_ids.is_empty());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn conflicting_definitions_and_missing_layout_targets_reject_without_mutating_source() {
        let mut accepted = fixture();
        let original = accepted.clone();
        let definition = controller_definition("catalog:controller");
        let placed = part(&definition.id, "part-controller", "U1");
        accepted.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "catalog:controller",
                "name": "Conflicting controller",
                "kind": "controller",
                "courtyard": [],
                "pads": []
            }))
            .unwrap(),
        );
        let conflict = placement_operation(&accepted, "board-main", &definition, &placed, None);
        assert!(conflict.is_err());
        assert_eq!(accepted.revision, original.revision);
        assert!(accepted.parts.is_empty());

        let missing_layout = placement_operation(
            &original,
            "board-main",
            &definition,
            &placed,
            Some("missing-layout"),
        );
        assert!(missing_layout.is_err());
        assert!(original.parts.is_empty());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn controller_references_follow_existing_u_sequence_without_reusing_occupied_numbers() {
        let mut document = fixture();
        document.parts = vec![
            part("existing", "one", "U1"),
            part("existing", "two", "R3"),
            part("existing", "three", "U3"),
        ];
        assert_eq!(next_controller_reference(&document), "U2");
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn cursor_placement_uses_project_grid_and_alt_free_position_policy() {
        let definition = controller_definition("catalog:controller");
        let pending = controller_part(
            definition,
            "part-controller".into(),
            "U1".into(),
            Vec2::default(),
        )
        .unwrap();
        let point = Vec2 { x: 4.6, y: -4.6 };

        assert_eq!(
            grid_snap_point(point, 0.25),
            Vec2 {
                x: 4.7625,
                y: -4.7625
            }
        );
        assert_eq!(grid_snap_point(point, -0.5), Vec2 { x: 4.5, y: -4.5 });
        assert_eq!(grid_snap_point(point, 0.0), point);
        assert_eq!(
            snap_placement_at(
                &fixture(),
                "board-main",
                &pending,
                point,
                PlacementSnapOptions {
                    snap_fraction: 0.25,
                    geometry_snap: false,
                    gap_snap: true,
                    free: false,
                }
            ),
            Vec2 {
                x: 4.7625,
                y: -4.7625
            }
        );
        assert_eq!(
            snap_placement_at(
                &fixture(),
                "board-main",
                &pending,
                point,
                PlacementSnapOptions {
                    snap_fraction: 0.25,
                    geometry_snap: true,
                    gap_snap: true,
                    free: true,
                }
            ),
            point
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn captured_owner_rejects_changed_snapshot_scope_generation_or_guide_stage() {
        let initial = accepted(fixture(), 11);
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "project".into(),
            board_id: "board-main".into(),
            instance_id: None,
        };
        let owner = PlacementOwner::capture(
            &initial,
            scope.clone(),
            4,
            "part-controller".into(),
            "catalog:controller".into(),
            None,
            Vec2 { x: 3.0, y: 9.0 },
        )
        .unwrap();

        assert!(owner.is_current(&initial, Some(&scope), 4, true, Some("project"), true));
        assert!(!owner.is_current(
            &accepted(fixture(), 12),
            Some(&scope),
            4,
            true,
            Some("project"),
            true
        ));
        assert!(!owner.is_current(&initial, Some(&scope), 5, true, Some("project"), true));
        assert!(!owner.is_current(&initial, Some(&scope), 4, false, Some("project"), true));
        assert!(!owner.is_current(&initial, Some(&scope), 4, true, Some("other"), true));
        assert!(!owner.is_current(&initial, Some(&scope), 4, true, Some("project"), false));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn only_the_exact_completed_operation_with_matching_ready_saved_document_can_return_to_wiring()
    {
        let initial = accepted(fixture(), 11);
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "project".into(),
            board_id: "board-main".into(),
            instance_id: None,
        };
        let definition = controller_definition("catalog:controller");
        let mut placed = part(&definition.id, "part-controller", "U1");
        let mut owner = PlacementOwner::capture(
            &initial,
            scope,
            4,
            placed.id.clone(),
            definition.id.clone(),
            None,
            Vec2::default(),
        )
        .unwrap();
        placed.pose.at = Vec2 { x: 14.0, y: -3.0 };
        owner.record_committed_position(placed.pose.at);
        let mut proposed = replacement(
            placement_operation(&initial.document, "board-main", &definition, &placed, None)
                .unwrap(),
        );
        proposed.revision += 1;
        let saved = accepted(proposed, 12);

        assert!(completion_is_accepted(
            true,
            Some(&TerminalOutcome::Completed),
            &Lifecycle::Ready,
            &Durability::Saved { revision: 1 },
            Some(&saved),
            &owner,
            "part-controller",
        ));
        let mut wrong_position_document = (*saved.document).clone();
        wrong_position_document
            .parts
            .iter_mut()
            .find(|part| part.id == "part-controller")
            .unwrap()
            .pose
            .at
            .x += 1.0;
        let wrong_position = accepted(wrong_position_document, 12);
        assert!(!completion_is_accepted(
            true,
            Some(&TerminalOutcome::Completed),
            &Lifecycle::Ready,
            &Durability::Saved { revision: 1 },
            Some(&wrong_position),
            &owner,
            "part-controller",
        ));
        assert!(!completion_is_accepted(
            false,
            Some(&TerminalOutcome::Completed),
            &Lifecycle::Ready,
            &Durability::Saved { revision: 1 },
            Some(&saved),
            &owner,
            "part-controller",
        ));
        assert!(!completion_is_accepted(
            true,
            Some(&TerminalOutcome::Rejected("stale".into())),
            &Lifecycle::Ready,
            &Durability::Saved { revision: 1 },
            Some(&saved),
            &owner,
            "part-controller",
        ));
        assert!(!completion_is_accepted(
            true,
            Some(&TerminalOutcome::Completed),
            &Lifecycle::Saving,
            &Durability::Saving { revision: 1 },
            Some(&saved),
            &owner,
            "part-controller",
        ));
    }
}
