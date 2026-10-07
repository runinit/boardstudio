//! Editor-lifetime owner for automatic, board-scoped electrical plan resolution.
//! The selected part is intentionally absent from this controller's request identity.
use super::part_net_admission::{part_net_owner_context_is_current, use_part_net_owner_lifetime};
use super::{
    FirmwarePositionEditRequest, FirmwarePositionFeedback, FirmwarePositionFeedbackState,
    PcbWiringResolution, WiringPlanIdentity, firmware_position_projection,
};
use crate::firmware_position_projection::{
    FirmwarePositionAdmission, FirmwarePositionFeedbackTarget, admits_edit,
};
use crate::pcb_wiring_mode_operation::{ResolutionAdmission, begin_resolution};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, ProjectDoc};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::Future,
    pin::Pin,
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

use super::{
    PartNetActions, PartNetEditAction, PartNetEditIdentity, PartNetEditRequest, PartNetFeedback,
    PartNetFeedbackState,
    part_connections::{self, PartNetIntent},
};

#[derive(Clone, Copy)]
struct FirmwareTickets(Signal<Vec<(FirmwarePositionEditRequest, EditTicket)>>);

pub(crate) fn pending_binding(
    identity: &crate::firmware_position_projection::FirmwarePositionIdentity,
    key_id: &str,
) -> Option<String> {
    let tickets = try_consume_context::<FirmwareTickets>()?;
    tickets.0.read().iter().rev().find_map(|(request, ticket)| {
        (ticket.is_pending()
            && request.identity.ui_scope == identity.ui_scope
            && request.identity.scope_generation == identity.scope_generation
            && request.key_id == key_id)
            .then(|| request.binding.clone())
    })
}

pub struct FirmwarePositionActions {
    pub feedback: Option<FirmwarePositionFeedback>,
    pub editable: bool,
    pub on_edit: EventHandler<FirmwarePositionEditRequest>,
}

/// Editor-lifetime root owner for legacy SetKeyBinding admission and exact outcome settlement.
pub fn use_firmware_position_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    resolution: Signal<PcbWiringResolution>,
) -> FirmwarePositionActions {
    let tickets = use_signal(Vec::<(FirmwarePositionEditRequest, EditTicket)>::new);
    use_context_provider(|| FirmwareTickets(tickets));
    let latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let feedback = use_signal(|| None::<FirmwarePositionFeedback>);
    let generation = scope_generation();
    let observed_version = version();

    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let mut tickets = tickets;
        let mut feedback = feedback;
        move |_| {
            let mut entries = tickets.peek().clone();
            let before = entries.len();
            entries.retain(|(request, ticket)| {
                let live = workspace() == "PCB"
                    && scope_generation() == request.identity.scope_generation
                    && runtime.scope().as_ref() == Some(&request.identity.ui_scope);
                let state = match ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(FirmwarePositionFeedbackState::Saved),
                    Settlement::Failed { message } => {
                        Some(FirmwarePositionFeedbackState::Failed(message))
                    }
                    Settlement::Retired => None,
                };
                if *latest.peek() == Some(ticket.operation()) {
                    feedback.set(state.map(|state| FirmwarePositionFeedback {
                        target: feedback_target(request),
                        state,
                    }));
                }
                false
            });
            if before != entries.len() {
                tickets.set(entries);
            }
        }
    }));

    let on_edit = use_callback({
        let runtime = runtime.clone();
        let mut tickets = tickets;
        let mut latest = latest;
        let mut feedback = feedback;
        let instance_is_current = instance_is_current.clone();
        move |request: FirmwarePositionEditRequest| {
            if workspace() != "PCB"
                || !instance_is_current()
                || scope_generation() != generation
                || runtime.scope().as_ref() != Some(&request.identity.ui_scope)
            {
                return;
            }
            let model = runtime.model();
            if !matches!(
                model.lifecycle,
                Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
            ) || model.display_preview.is_some()
                || model.gesture.is_some()
                || !matches!(
                    model.durability,
                    Durability::Saved { .. } | Durability::Saving { .. }
                )
                || model.active_board_id != request.identity.ui_scope.board_id
                || model.active_instance_id != request.identity.ui_scope.instance_id
            {
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if snapshot.session_epoch != request.identity.ui_scope.session_epoch
                || snapshot.document.id != request.identity.ui_scope.document_id
                || snapshot.token != request.identity.plan.token
                || snapshot.document.revision != request.identity.plan.revision
                || runtime.electrical_preview_executor_epoch()
                    != request.identity.plan.executor_epoch
                || !snapshot
                    .document
                    .boards
                    .iter()
                    .any(|board| board.id == request.identity.ui_scope.board_id)
            {
                return;
            }
            let live_scope = runtime
                .scope()
                .unwrap_or_else(|| request.identity.ui_scope.clone());
            let plan_identity = WiringPlanIdentity {
                scope: Scope {
                    instance_id: None,
                    ..live_scope.clone()
                },
                token: snapshot.token,
                revision: snapshot.document.revision,
                executor_epoch: runtime.electrical_preview_executor_epoch(),
            };
            if plan_identity != request.identity.plan {
                return;
            }
            let PcbWiringResolution::Current { identity, plan } = &*resolution.read() else {
                return;
            };
            if identity != &request.identity.plan {
                return;
            }
            let current = firmware_position_projection::project(
                &snapshot.document,
                &plan_identity,
                &request.identity.ui_scope,
                generation,
                super::PlanLifecycle::Current(identity, plan),
            );
            if !admits_edit(
                &request.identity,
                &request.key_id,
                FirmwarePositionAdmission {
                    workspace: workspace(),
                    current_generation: scope_generation(),
                    instance_is_current: instance_is_current(),
                    runtime_scope: runtime.scope().as_ref(),
                    accepted: snapshot,
                    executor_epoch: runtime.electrical_preview_executor_epoch(),
                    current_plan: Some(identity),
                    current_projection: &current,
                },
            ) {
                return;
            }
            let ticket = EditTicket::begin(
                &runtime,
                "firmware-position",
                Some("firmware position".into()),
                firmware_position_resolver(request.clone()),
            );
            latest.set(Some(ticket.operation()));
            feedback.set(Some(FirmwarePositionFeedback {
                target: feedback_target(&request),
                state: FirmwarePositionFeedbackState::Pending,
            }));
            tickets.write().push((request, ticket));
        }
    });

    let editable = firmware_position_editable(
        &runtime,
        workspace(),
        scope_generation(),
        generation,
        instance_is_current(),
        &resolution.read(),
    );
    FirmwarePositionActions {
        feedback: feedback(),
        editable,
        on_edit,
    }
}

fn firmware_position_resolver(request: FirmwarePositionEditRequest) -> EditResolver {
    EditResolver::new("firmware-position", move |accepted: &AcceptedSnapshot| {
        let scope = &request.identity.ui_scope;
        if accepted.session_epoch != scope.session_epoch
            || accepted.document.id != scope.document_id
        {
            return Resolution::Retire(boardstudio_application::DOCUMENT_SESSION_CHANGED.into());
        }
        let boardstudio_core::model::CoreRequest::ResolveElectrical {
            request: electrical_request,
            ..
        } = crate::pcb_wiring_mode_operation::electrical_preview_request(
            "firmware-position",
            &accepted.document,
            &scope.board_id,
        )
        else {
            unreachable!()
        };
        let plan = boardstudio_core::electrical::resolve(electrical_request);
        let identity = WiringPlanIdentity {
            token: accepted.token,
            revision: accepted.document.revision,
            ..request.identity.plan.clone()
        };
        let projection = firmware_position_projection::project(
            &accepted.document,
            &identity,
            scope,
            request.identity.scope_generation,
            super::PlanLifecycle::Current(&identity, &plan),
        );
        if !projection.keys.iter().any(|key| key.id == request.key_id) {
            return Resolution::Retire(
                "The firmware key was deleted or is no longer eligible on this board.".into(),
            );
        }
        if projection
            .bindings
            .get(&request.key_id)
            .map(String::as_str)
            .unwrap_or("&none")
            == request.binding
        {
            return Resolution::Unchanged;
        }
        Resolution::Submit(EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec![scope.board_id.clone(), request.key_id.clone()],
            operation: EditOperation::SetKeyBinding {
                board_id: scope.board_id.clone(),
                key_id: request.key_id.clone(),
                binding: request.binding.clone(),
            },
        })
    })
}

#[derive(Clone, Copy)]
struct PartNetTickets {
    tickets: Signal<Vec<(PartNetEditRequest, EditTicket, bool)>>,
    preparing_create: Signal<bool>,
}

pub(super) fn create_net_pending() -> bool {
    try_consume_context::<PartNetTickets>().is_some_and(|owner| {
        (owner.preparing_create)()
            || owner
                .tickets
                .read()
                .iter()
                .any(|(_, ticket, create)| *create && ticket.is_pending())
    })
}

pub(super) fn pending_net(
    identity: &PartNetEditIdentity,
    pad_ids: &[String],
) -> Option<Option<String>> {
    let owner = try_consume_context::<PartNetTickets>()?;
    owner
        .tickets
        .read()
        .iter()
        .rev()
        .find_map(|(request, ticket, _)| match &request.action {
            PartNetEditAction::AssignPads {
                pad_ids: target,
                net_id,
            } if ticket.is_pending()
                && same_part_net_identity(&request.identity, identity)
                && target == pad_ids =>
            {
                Some(net_id.clone())
            }
            _ => None,
        })
}

/// Queue each connection intent against the accepted board at execution.
pub fn use_pcb_part_net_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
) -> PartNetActions {
    let tickets = use_signal(Vec::<(PartNetEditRequest, EditTicket, bool)>::new);
    let latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let feedback = use_signal(|| None::<PartNetFeedback>);
    let preparing_create = use_signal(|| false);
    use_context_provider(|| PartNetTickets {
        tickets,
        preparing_create,
    });
    let queue = use_hook(|| {
        Rc::new(RefCell::new(VecDeque::<(
            PartNetEditRequest,
            String,
            Option<String>,
            bool,
        )>::new()))
    });
    let preparing = use_hook(|| Rc::new(Cell::new(false)));
    let alive = use_part_net_owner_lifetime();
    let generation = scope_generation();
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let mut tickets = tickets;
        let mut feedback = feedback;
        move |_| {
            let mut entries = tickets.peek().clone();
            let before = entries.len();
            entries.retain(|(request, ticket, _)| {
                let identity = &request.identity;
                let live = workspace() == "PCB"
                    && scope_generation() == identity.generation
                    && runtime.scope().as_ref() == Some(&identity.ui_scope)
                    && runtime.model().selected_part_ids.as_slice() == [identity.part_id.as_str()];
                let result = match ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => {
                        Some(("Connection saved.".into(), PartNetFeedbackState::Saved))
                    }
                    Settlement::Failed { message } => Some((message, PartNetFeedbackState::Failed)),
                    Settlement::Retired => None,
                };
                if *latest.peek() == Some(ticket.operation()) {
                    feedback.set(result.map(|(message, state)| PartNetFeedback {
                        identity: identity.clone(),
                        message,
                        state,
                    }));
                }
                false
            });
            if before != entries.len() {
                tickets.set(entries);
            }
        }
    }));
    let on_edit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        move |request: PartNetEditRequest| {
            if workspace() != "PCB" || scope_generation() != generation || !instance_is_current() {
                return;
            }
            let Some(current) =
                current_part_net_identity(&runtime, generation, instance_is_current())
            else {
                return;
            };
            if !request_matches_current_part(&request.identity, &current) {
                return;
            }
            let create = matches!(&request.action, PartNetEditAction::CreateNet { .. });
            if create
                && (*preparing_create.peek()
                    || tickets
                        .peek()
                        .iter()
                        .any(|(_, ticket, create)| *create && ticket.is_pending()))
            {
                return;
            }
            let accepted = runtime.model().accepted.unwrap();
            let part = accepted
                .document
                .parts
                .iter()
                .find(|part| part.id == request.identity.part_id)
                .unwrap();
            let definition = accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .unwrap();
            let definition_id = definition.id.clone();
            let source = definition
                .generator
                .as_ref()
                .map(|generator| generator.source.clone());
            let mut preparing_create = preparing_create;
            if create {
                preparing_create.set(true);
            }
            let mut latest = latest;
            latest.set(None);
            let mut feedback = feedback;
            feedback.set(None);
            queue
                .borrow_mut()
                .push_back((request, definition_id, source, create));
            if preparing.replace(true) {
                return;
            }
            let queue = queue.clone();
            let preparing = preparing.clone();
            let runtime = runtime.clone();
            let alive = alive.clone();
            let instance_is_current = instance_is_current.clone();
            let mut tickets = tickets;
            let mut latest = latest;
            let mut feedback = feedback;
            spawn_local(async move {
                loop {
                    let item = queue.borrow_mut().pop_front();
                    let Some((request, definition_id, source, create)) = item else {
                        preparing.set(false);
                        break;
                    };
                    let classified = match source.as_deref() {
                        Some(source) => {
                            crate::presentation::parts::is_generator_source(source.to_owned()).await
                        }
                        None => Ok(false),
                    };
                    if !alive.get() {
                        return;
                    }
                    if create {
                        preparing_create.set(false);
                    }
                    if !part_net_owner_context_is_current(
                        &alive,
                        workspace,
                        scope_generation,
                        request.identity.generation,
                    ) || !instance_is_current()
                        || runtime.scope().as_ref() != Some(&request.identity.ui_scope)
                        || runtime.model().selected_part_ids.as_slice()
                            != [request.identity.part_id.as_str()]
                    {
                        continue;
                    }
                    let request_for_ui = request.clone();
                    let seed = runtime.operation().0;
                    let resolver = match classified {
                        Ok(is_generator_source) => part_net_resolver(
                            request,
                            definition_id,
                            source,
                            is_generator_source,
                            seed,
                        ),
                        Err(reason) => EditResolver::new("pcb-part-net", move |_| {
                            Resolution::Retire(reason.clone())
                        }),
                    };
                    let ticket = EditTicket::begin(
                        &runtime,
                        "pcb-part-net",
                        Some("connection".into()),
                        resolver,
                    );
                    latest.set(Some(ticket.operation()));
                    feedback.set(None);
                    tickets.write().push((request_for_ui, ticket, create));
                }
            });
        }
    });
    let identity = current_part_net_identity(&runtime, generation, instance_is_current());
    let editable = identity.is_some() && workspace() == "PCB";
    let visible_feedback = feedback().filter(|feedback| {
        identity
            .as_ref()
            .is_some_and(|identity| same_part_net_identity(&feedback.identity, identity))
    });
    PartNetActions {
        identity,
        editable,
        feedback: visible_feedback,
        on_edit,
    }
}

fn part_net_resolver(
    request: PartNetEditRequest,
    definition_id: String,
    source: Option<String>,
    is_generator_source: bool,
    seed: u64,
) -> EditResolver {
    EditResolver::new("pcb-part-net", move |accepted: &AcceptedSnapshot| {
        if accepted.session_epoch != request.identity.ui_scope.session_epoch
            || accepted.document.id != request.identity.ui_scope.document_id
        {
            return Resolution::Retire(boardstudio_application::DOCUMENT_SESSION_CHANGED.into());
        }
        let part = accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == request.identity.part_id);
        let definition = part.and_then(|part| {
            accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
        });
        if definition.is_none_or(|definition| {
            definition.id != definition_id
                || definition
                    .generator
                    .as_ref()
                    .map(|generator| &generator.source)
                    != source.as_ref()
        }) {
            return Resolution::Retire("The part was deleted or its definition changed.".into());
        }
        let intent = match &request.action {
            PartNetEditAction::AssignPads { pad_ids, net_id } => PartNetIntent::AssignPads {
                pad_ids: pad_ids.clone(),
                net_id: net_id.clone(),
            },
            PartNetEditAction::CreateNet { name } => PartNetIntent::CreateNet {
                net_id: fresh_net_id(&accepted.document, seed),
                name: name.clone(),
            },
        };
        let proposal = match part_connections::propose(
            &accepted.document,
            &request.identity.board_id,
            &request.identity.part_id,
            is_generator_source,
            intent,
        ) {
            Ok(proposal) => proposal,
            Err(reason) => return Resolution::Retire(reason),
        };
        if proposal == *accepted.document {
            return Resolution::Unchanged;
        }
        Resolution::Submit(EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec![
                request.identity.board_id.clone(),
                request.identity.part_id.clone(),
            ],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(proposal),
            },
        })
    })
}

fn current_part_net_identity(
    runtime: &Runtime,
    generation: u64,
    instance_is_current: bool,
) -> Option<PartNetEditIdentity> {
    if !instance_is_current {
        return None;
    }
    let model = runtime.model();
    let scope = runtime.scope()?;
    let accepted = model.accepted.as_ref()?;
    let [part_id] = model.selected_part_ids.as_slice() else {
        return None;
    };
    let board = accepted
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)?;
    if !matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) || model.display_preview.is_some()
        || model.gesture.is_some()
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || accepted.session_epoch != scope.session_epoch
        || accepted.document.id != scope.document_id
        || accepted.scene.revision != accepted.document.revision
        || !board.part_ids.contains(part_id)
    {
        return None;
    }
    let part = accepted
        .document
        .parts
        .iter()
        .find(|part| part.id == *part_id)?;
    let definition = accepted
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)?;
    if matches!(
        &definition.kind,
        boardstudio_core::model::PartKind::Switch | boardstudio_core::model::PartKind::Controller
    ) {
        return None;
    }
    Some(PartNetEditIdentity {
        board_id: scope.board_id.clone(),
        ui_scope: scope,
        part_id: part_id.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        generation,
    })
}

fn same_part_net_identity(left: &PartNetEditIdentity, right: &PartNetEditIdentity) -> bool {
    left.ui_scope == right.ui_scope
        && left.generation == right.generation
        && left.part_id == right.part_id
        && left.board_id == right.board_id
}

fn request_matches_current_part(
    rendered: &PartNetEditIdentity,
    current: &PartNetEditIdentity,
) -> bool {
    rendered == current
}

fn fresh_net_id(document: &ProjectDoc, operation: u64) -> String {
    let mut suffix = 0u64;
    loop {
        let id = format!("pcb-net-{operation}-{suffix}");
        if !document.nets.iter().any(|net| net.id == id) {
            return id;
        }
        suffix = suffix.checked_add(1).expect("net identity space exhausted");
    }
}

#[cfg(test)]
mod part_net_identity_tests {
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use wasm_bindgen_test::wasm_bindgen_test;

    fn identity(
        board_id: &str,
        part_id: &str,
        token: u64,
        revision: u64,
        generation: u64,
    ) -> PartNetEditIdentity {
        PartNetEditIdentity {
            board_id: board_id.into(),
            ui_scope: Scope {
                session_epoch: SessionEpoch(3),
                document_id: "project".into(),
                board_id: board_id.into(),
                instance_id: None,
            },
            part_id: part_id.into(),
            token: SnapshotToken(token),
            revision,
            generation,
        }
    }

    #[wasm_bindgen_test]
    fn rendered_part_action_is_rejected_after_board_selection_or_snapshot_changes() {
        let rendered = identity("left", "left-J2", 7, 12, 4);
        for current in [
            identity("right", "left-J2", 7, 12, 4),
            identity("left", "left-SW1", 7, 12, 4),
            identity("left", "left-J2", 8, 12, 4),
            identity("left", "left-J2", 7, 13, 4),
            identity("left", "left-J2", 7, 12, 5),
        ] {
            assert!(!request_matches_current_part(&rendered, &current));
        }
        assert!(request_matches_current_part(&rendered, &rendered));
    }
}

fn firmware_position_editable(
    runtime: &Runtime,
    workspace: &str,
    current_generation: u64,
    captured_generation: u64,
    instance_is_current: bool,
    resolution: &PcbWiringResolution,
) -> bool {
    if workspace != "PCB" || !instance_is_current || current_generation != captured_generation {
        return false;
    }
    let model = runtime.model();
    if !matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) || model.display_preview.is_some()
        || model.gesture.is_some()
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
    {
        return false;
    }
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    let Some(ui_scope) = runtime.scope() else {
        return false;
    };
    if model.active_board_id != ui_scope.board_id
        || model.active_instance_id != ui_scope.instance_id
        || snapshot.session_epoch != ui_scope.session_epoch
        || snapshot.document.id != ui_scope.document_id
        || snapshot.scene.revision != snapshot.document.revision
    {
        return false;
    }
    let mut board_scope = ui_scope.clone();
    board_scope.instance_id = None;
    let plan_identity = WiringPlanIdentity {
        scope: board_scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    let PcbWiringResolution::Current { identity, plan } = resolution else {
        return false;
    };
    let projection = firmware_position_projection::project(
        &snapshot.document,
        &plan_identity,
        &ui_scope,
        current_generation,
        super::PlanLifecycle::Current(identity, plan),
    );
    identity == &plan_identity && projection.identity.is_some() && !projection.keys.is_empty()
}

fn feedback_target(request: &FirmwarePositionEditRequest) -> FirmwarePositionFeedbackTarget {
    FirmwarePositionFeedbackTarget {
        ui_scope: request.identity.ui_scope.clone(),
        scope_generation: request.identity.scope_generation,
        key_id: request.key_id.clone(),
    }
}

#[derive(Clone)]
pub struct PcbWiringMount {
    pub resolution: PcbWiringResolution,
    pub resolution_signal: Signal<PcbWiringResolution>,
    pub on_resolve: EventHandler<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WiringResolutionNotice {
    Pending,
    Failed(String),
    Waiting,
}

impl WiringResolutionNotice {
    pub fn role(&self) -> &'static str {
        match self {
            Self::Pending | Self::Waiting => "status",
            Self::Failed(_) => "alert",
        }
    }
}

pub fn wiring_resolution_notice(
    resolution: &PcbWiringResolution,
    identity: &WiringPlanIdentity,
) -> WiringResolutionNotice {
    match resolution {
        PcbWiringResolution::Pending { identity: owner } if owner == identity => {
            WiringResolutionNotice::Pending
        }
        PcbWiringResolution::Failed {
            identity: owner,
            message,
        } if owner == identity => WiringResolutionNotice::Failed(message.clone()),
        PcbWiringResolution::Current {
            identity: owner, ..
        } if owner == identity => WiringResolutionNotice::Waiting,
        _ => WiringResolutionNotice::Waiting,
    }
}

/// Keep the board-plan query alive at Editor lifetime, regardless of the selected component
/// or which workspace is currently visible. Call this hook unconditionally in the page parent.
pub fn use_pcb_wiring_controller(runtime: Rc<Runtime>, version: Signal<u64>) -> PcbWiringMount {
    let current: Rc<dyn Fn() -> Option<(WiringPlanIdentity, AcceptedSnapshot, Scope)>> = {
        let runtime = runtime.clone();
        Rc::new(move || current_input(&runtime))
    };
    let resolve: WiringResolver = Rc::new(move |accepted, scope| {
        let runtime = runtime.clone();
        Box::pin(async move { runtime.resolve_electrical_preview(accepted, scope).await })
    });
    use_pcb_wiring_resolution_owner(version, current, resolve)
}

type WiringInput = (WiringPlanIdentity, AcceptedSnapshot, Scope);
type WiringResolver = Rc<
    dyn Fn(
        AcceptedSnapshot,
        Scope,
    ) -> Pin<
        Box<dyn Future<Output = Result<boardstudio_core::electrical::ElectricalPlan, String>>>,
    >,
>;

/// The Runtime-facing hook delegates through this private seam so the mounted owner can be
/// qualified with controlled replies without changing production admission or settlement.
fn use_pcb_wiring_resolution_owner(
    version: Signal<u64>,
    current_input: Rc<dyn Fn() -> Option<WiringInput>>,
    resolver: WiringResolver,
) -> PcbWiringMount {
    let resolution = use_signal(|| PcbWiringResolution::Idle);
    let latest_request = use_hook(|| Rc::new(Cell::new(0_u64)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    // Runtime is deliberately not a Dioxus signal. The page's existing version signal wakes
    // this projection after Runtime notifications; identical board identity is deduplicated.
    let _runtime_version = version();
    let identity = current_input().map(|(identity, _, _)| identity);
    use_effect(use_reactive((&identity, &_runtime_version), {
        let current_input = current_input.clone();
        let resolver = resolver.clone();
        let mut resolution = resolution;
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |(identity, _version)| {
            if identity.is_some() {
                start_resolution(
                    current_input.clone(),
                    resolver.clone(),
                    resolution,
                    latest_request.clone(),
                    alive.clone(),
                    false,
                );
            } else {
                invalidate_pending(latest_request.as_ref());
                resolution.set(PcbWiringResolution::Idle);
            }
        }
    }));

    let on_resolve = use_callback({
        let current_input = current_input.clone();
        let resolver = resolver.clone();
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |()| {
            start_resolution(
                current_input.clone(),
                resolver.clone(),
                resolution,
                latest_request.clone(),
                alive.clone(),
                true,
            );
        }
    });
    PcbWiringMount {
        resolution: resolution(),
        resolution_signal: resolution,
        on_resolve,
    }
}

fn current_input(runtime: &Runtime) -> Option<(WiringPlanIdentity, AcceptedSnapshot, Scope)> {
    let model = runtime.model();
    if matches!(
        model.lifecycle,
        Lifecycle::Empty | Lifecycle::Opening | Lifecycle::Closing | Lifecycle::Closed
    ) {
        return None;
    }
    let accepted = model.accepted?;
    let mut scope = runtime.scope()?;
    // The electrical preview is a board plan with React's instanceId=null. Normalize away a
    // physical-instance selection so Case/Layout instance changes don't churn the board query.
    scope.instance_id = None;
    if scope.session_epoch != accepted.session_epoch
        || scope.document_id != accepted.document.id
        || model.active_board_id != scope.board_id
        || accepted.scene.revision != accepted.document.revision
        || !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return None;
    }
    let identity = WiringPlanIdentity {
        scope: scope.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    Some((identity, accepted, scope))
}

fn start_resolution(
    current_input: Rc<dyn Fn() -> Option<WiringInput>>,
    resolver: WiringResolver,
    mut resolution: Signal<PcbWiringResolution>,
    latest_request: Rc<Cell<u64>>,
    alive: Rc<Cell<bool>>,
    force: bool,
) {
    if !alive.get() {
        return;
    }
    let Some((identity, accepted, scope)) = current_input() else {
        invalidate_pending(&latest_request);
        resolution.set(PcbWiringResolution::Idle);
        return;
    };
    let mut admission = match &*resolution.read() {
        PcbWiringResolution::Idle => ResolutionAdmission::Idle,
        PcbWiringResolution::Pending { identity } => ResolutionAdmission::Pending(identity.clone()),
        PcbWiringResolution::Current { identity, .. } => {
            ResolutionAdmission::Current(identity.clone())
        }
        PcbWiringResolution::Failed { identity, .. } => {
            ResolutionAdmission::Failed(identity.clone())
        }
    };
    if !begin_resolution(&mut admission, &identity, force) {
        return;
    }
    let request_generation = next_request(latest_request.as_ref());
    resolution.set(PcbWiringResolution::Pending {
        identity: identity.clone(),
    });
    spawn_local(async move {
        let mut resolution = resolution;
        let result = resolver(accepted, scope).await;
        if !alive.get() || latest_request.get() != request_generation {
            return;
        }
        let Some((current, _, _)) = current_input() else {
            resolution.set(PcbWiringResolution::Idle);
            return;
        };
        if current != identity {
            return;
        }
        resolution.set(match result {
            Ok(plan) => PcbWiringResolution::Current {
                identity,
                plan: Rc::new(plan),
            },
            Err(message) => PcbWiringResolution::Failed { identity, message },
        });
    });
}

fn next_request(sequence: &Cell<u64>) -> u64 {
    let request = sequence
        .get()
        .checked_add(1)
        .expect("PCB wiring request sequence exhausted");
    sequence.set(request);
    request
}

fn invalidate_pending(sequence: &Cell<u64>) {
    let _ = next_request(sequence);
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_resolution_tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::electrical::{ElectricalMode, ElectricalPlan, ElectricalPlanRequest};
    use boardstudio_core::model::{Readiness, SceneDelta};
    use futures_channel::oneshot;
    use std::{cell::RefCell, collections::VecDeque, sync::Arc};
    use wasm_bindgen_test::wasm_bindgen_test;

    type Reply = oneshot::Sender<Result<ElectricalPlan, String>>;

    struct Probe {
        version: Cell<u64>,
        input: RefCell<Option<WiringInput>>,
        replies: RefCell<VecDeque<Reply>>,
        calls: Cell<usize>,
        latest: RefCell<Option<PcbWiringMount>>,
    }

    fn board_input(board_id: &str, token: u64) -> WiringInput {
        board_input_with_controller(board_id, token, 0, None)
    }

    fn board_input_with_controller(
        board_id: &str,
        token: u64,
        revision: u64,
        controller_part_id: Option<&str>,
    ) -> WiringInput {
        let mut document = ProjectDoc::empty("resolver-test", "Resolver test");
        document.revision = revision;
        document.boards.push(boardstudio_core::model::Board {
            id: board_id.into(),
            name: board_id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.hardware = Some(boardstudio_core::model::HardwareConfiguration {
            boards: vec![boardstudio_core::model::ElectricalBoardConfiguration {
                board_id: board_id.into(),
                controller_part_id: controller_part_id.map(str::to_owned),
                ..Default::default()
            }],
            ..Default::default()
        });
        let identity = WiringPlanIdentity {
            scope: Scope {
                session_epoch: SessionEpoch(1),
                document_id: document.id.clone(),
                board_id: board_id.into(),
                instance_id: None,
            },
            token: SnapshotToken(token),
            revision,
            executor_epoch: 1,
        };
        let accepted = AcceptedSnapshot {
            token: SnapshotToken(token),
            session_epoch: SessionEpoch(1),
            document: Arc::new(document.clone()),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision: document.revision,
                transaction_id: "resolver-test".into(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: Readiness {
                    layout: true,
                    outline: true,
                    pcb: true,
                    case_ready: false,
                },
            }),
        };
        let scope = identity.scope.clone();
        (identity, accepted, scope)
    }

    fn test_plan(board_id: &str, revision: u64) -> ElectricalPlan {
        let mut plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
            document: ProjectDoc::empty("resolver-test", "Resolver test"),
            instance_id: None,
            mode: ElectricalMode::Matrix,
            locks: Default::default(),
            controller_profile: None,
            board_id: Some(board_id.into()),
            controller_part_id: None,
        });
        plan.revision = revision;
        plan
    }

    fn host() -> Element {
        let probe = use_context::<Rc<Probe>>();
        let mut version = use_signal(|| probe.version.get());
        if *version.peek() != probe.version.get() {
            version.set(probe.version.get());
        }
        let current_probe = probe.clone();
        let current: Rc<dyn Fn() -> Option<WiringInput>> =
            Rc::new(move || current_probe.input.borrow().clone());
        let resolver_probe = probe.clone();
        let resolver: WiringResolver = Rc::new(move |_, _| {
            resolver_probe.calls.set(resolver_probe.calls.get() + 1);
            let (sender, receiver) = oneshot::channel();
            resolver_probe.replies.borrow_mut().push_back(sender);
            Box::pin(async move {
                receiver
                    .await
                    .unwrap_or_else(|_| Err("test resolver reply dropped".into()))
            })
        });
        let mount = use_pcb_wiring_resolution_owner(version, current, resolver);
        *probe.latest.borrow_mut() = Some(mount.clone());
        let identity = probe
            .input
            .borrow()
            .as_ref()
            .map(|(identity, _, _)| identity.clone());
        let notice = identity
            .as_ref()
            .map(|identity| wiring_resolution_notice(&mount.resolution, identity));
        rsx! {
            div {
                match notice {
                    Some(WiringResolutionNotice::Pending) => rsx! { p { role: "status", "Resolving wiring…" } },
                    Some(WiringResolutionNotice::Failed(message)) => rsx! { p { role: "alert", "{message}" } },
                    _ => rsx! {},
                }
            }
        }
    }

    fn flush(dom: &mut VirtualDom) {
        dom.mark_all_dirty();
        for _ in 0..5 {
            dom.render_immediate_to_vec();
            let mut work = std::pin::pin!(dom.wait_for_work());
            let _ = work
                .as_mut()
                .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        }
    }

    async fn tick(dom: &mut VirtualDom) {
        gloo_timers::future::TimeoutFuture::new(20).await;
        flush(dom);
    }

    async fn until_calls(probe: &Probe, dom: &mut VirtualDom, expected: usize) {
        for _ in 0..20 {
            if probe.calls.get() >= expected {
                return;
            }
            tick(dom).await;
        }
        panic!(
            "resolver was called {} times, expected {expected}",
            probe.calls.get()
        );
    }

    fn reply(probe: &Probe, result: Result<ElectricalPlan, String>) {
        reply_at(probe, 0, result);
    }

    fn reply_at(probe: &Probe, index: usize, result: Result<ElectricalPlan, String>) {
        probe
            .replies
            .borrow_mut()
            .remove(index)
            .expect("resolver call should have a pending reply")
            .send(result)
            .expect("mounted owner should still be awaiting the reply");
    }

    #[wasm_bindgen_test]
    async fn mounted_resolution_failure_is_visible_retryable_and_ignores_old_board_reply() {
        let probe = Rc::new(Probe {
            version: Cell::new(0),
            input: RefCell::new(Some(board_input("left", 1))),
            replies: RefCell::default(),
            calls: Cell::new(0),
            latest: RefCell::default(),
        });
        let mut dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dom.rebuild_to_vec();
        flush(&mut dom);

        until_calls(&probe, &mut dom, 1).await;
        assert_eq!(
            probe.latest.borrow().as_ref().map(|mount| {
                wiring_resolution_notice(
                    &mount.resolution,
                    &probe.input.borrow().as_ref().unwrap().0,
                )
            }),
            Some(WiringResolutionNotice::Pending)
        );
        reply(&probe, Err("injected Core resolution failure".into()));
        tick(&mut dom).await;
        assert_eq!(
            probe.latest.borrow().as_ref().map(|mount| {
                wiring_resolution_notice(
                    &mount.resolution,
                    &probe.input.borrow().as_ref().unwrap().0,
                )
            }),
            Some(WiringResolutionNotice::Failed(
                "injected Core resolution failure".into()
            ))
        );
        assert_eq!(
            WiringResolutionNotice::Failed("injected Core resolution failure".into()).role(),
            "alert"
        );

        probe
            .latest
            .borrow()
            .as_ref()
            .expect("mounted owner should publish retry action")
            .on_resolve
            .call(());
        until_calls(&probe, &mut dom, 2).await;
        reply(&probe, Ok(test_plan("left", 0)));
        tick(&mut dom).await;
        assert!(matches!(
            probe.latest.borrow().as_ref().unwrap().resolution,
            PcbWiringResolution::Current { .. }
        ));

        // Start a forced Left request, then let the accepted input advance to Right. The Right
        // request succeeds first; the delayed old-board reply must not replace that result.
        probe.latest.borrow().as_ref().unwrap().on_resolve.call(());
        until_calls(&probe, &mut dom, 3).await;
        *probe.input.borrow_mut() = Some(board_input("right", 2));
        probe.version.set(1);
        tick(&mut dom).await;
        until_calls(&probe, &mut dom, 4).await;
        reply_at(&probe, 1, Ok(test_plan("right", 0)));
        tick(&mut dom).await;
        assert!(matches!(
            probe.latest.borrow().as_ref().unwrap().resolution,
            PcbWiringResolution::Current { ref identity, .. }
                if identity.scope.board_id == "right"
        ));
        reply(&probe, Ok(test_plan("left", 0)));
        tick(&mut dom).await;
        assert!(matches!(
            probe.latest.borrow().as_ref().unwrap().resolution,
            PcbWiringResolution::Current { ref identity, .. }
                if identity.scope.board_id == "right"
        ));
    }

    #[wasm_bindgen_test]
    async fn mounted_resolution_ignores_old_same_board_reply_after_controller_revision_changes() {
        let probe = Rc::new(Probe {
            version: Cell::new(0),
            input: RefCell::new(Some(board_input_with_controller(
                "left",
                10,
                10,
                Some("left/U1"),
            ))),
            replies: RefCell::default(),
            calls: Cell::new(0),
            latest: RefCell::default(),
        });
        let mut dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dom.rebuild_to_vec();
        flush(&mut dom);

        until_calls(&probe, &mut dom, 1).await;

        // Model a controller selection edit accepted on the same board: both the snapshot token
        // and document revision advance, while the controller ID in the saved board config changes.
        *probe.input.borrow_mut() =
            Some(board_input_with_controller("left", 11, 11, Some("left/U2")));
        probe.version.set(1);
        tick(&mut dom).await;
        until_calls(&probe, &mut dom, 2).await;

        let mut new_plan = test_plan("left", 11);
        new_plan.controller_part_id = Some("left/U2".into());
        reply_at(&probe, 1, Ok(new_plan));
        tick(&mut dom).await;
        assert!(matches!(
            probe.latest.borrow().as_ref().unwrap().resolution,
            PcbWiringResolution::Current { ref identity, ref plan }
                if identity.scope.board_id == "left"
                    && identity.token == SnapshotToken(11)
                    && identity.revision == 11
                    && plan.revision == 11
                    && plan.controller_part_id.as_deref() == Some("left/U2")
        ));

        let mut old_plan = test_plan("left", 10);
        old_plan.controller_part_id = Some("left/U1".into());
        reply(&probe, Ok(old_plan));
        tick(&mut dom).await;
        assert!(matches!(
            probe.latest.borrow().as_ref().unwrap().resolution,
            PcbWiringResolution::Current { ref identity, ref plan }
                if identity.scope.board_id == "left"
                    && identity.token == SnapshotToken(11)
                    && identity.revision == 11
                    && plan.revision == 11
                    && plan.controller_part_id.as_deref() == Some("left/U2")
        ));
    }
}
