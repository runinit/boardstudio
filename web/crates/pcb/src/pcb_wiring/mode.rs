//! Accepted board-level mode edits for PCB wiring.
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::{
    pcb_wiring_mode_operation::{BoardWiringModeFeedbackTarget, BoardWiringModeIdentity},
    runtime::Runtime,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::{electrical::ElectricalMode, model::EditOperation};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeEditRequest {
    pub identity: BoardWiringModeIdentity,
    pub mode: ElectricalMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardWiringModeFeedback {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeFeedbackView {
    pub target: BoardWiringModeFeedbackTarget,
    pub request_plan: WiringPlanIdentity,
    pub state: BoardWiringModeFeedback,
}

#[derive(Clone, PartialEq)]
pub struct BoardWiringModeActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    pub feedback: Option<BoardWiringModeFeedbackView>,
    pub on_change: EventHandler<BoardWiringModeEditRequest>,
}

use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
#[derive(Clone, Copy)]
struct ModeTickets(Signal<Vec<(BoardWiringModeEditRequest, EditTicket)>>);

pub(super) fn pending_mode(
    identity: &BoardWiringModeIdentity,
) -> Option<boardstudio_core::electrical::ElectricalMode> {
    let tickets = try_consume_context::<ModeTickets>()?;
    tickets.0.read().iter().rev().find_map(|(request, ticket)| {
        if ticket.is_pending() && request.identity.feedback_target() == identity.feedback_target() {
            Some(request.mode)
        } else {
            None
        }
    })
}

pub fn use_board_wiring_mode_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
) -> BoardWiringModeActions {
    let tickets = use_signal(Vec::<(BoardWiringModeEditRequest, EditTicket)>::new);
    use_context_provider(|| ModeTickets(tickets));
    let latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let feedback = use_signal(|| None::<BoardWiringModeFeedbackView>);
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
                    && request.identity.feedback_target().is_visible(
                        runtime.scope().as_ref(),
                        runtime
                            .model()
                            .selected_part_ids
                            .first()
                            .map(String::as_str),
                        scope_generation(),
                    );
                let state = match ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(BoardWiringModeFeedback::Saved),
                    Settlement::Failed { message } => {
                        Some(BoardWiringModeFeedback::Failed(message))
                    }
                    Settlement::Retired => None,
                };
                if *latest.peek() == Some(ticket.operation()) {
                    feedback.set(state.map(|state| BoardWiringModeFeedbackView {
                        target: request.identity.feedback_target(),
                        request_plan: request.identity.plan.clone(),
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
    let on_change = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut tickets = tickets;
        let mut latest = latest;
        let mut feedback = feedback;
        move |request: BoardWiringModeEditRequest| {
            let Some(_snapshot) = current_edit_snapshot(
                &runtime,
                &request.identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let PcbWiringResolution::Current {
                identity,
                plan: _plan,
            } = &*resolution.read()
            else {
                return;
            };
            if identity != &request.identity.plan {
                return;
            };
            let ticket = EditTicket::begin(
                &runtime,
                "pcb-wiring-mode",
                Some("wiring mode".into()),
                mode_resolver(request.clone()),
            );
            latest.set(Some(ticket.operation()));
            feedback.set(Some(BoardWiringModeFeedbackView {
                target: request.identity.feedback_target(),
                request_plan: request.identity.plan.clone(),
                state: BoardWiringModeFeedback::Pending,
            }));
            tickets.write().push((request, ticket));
        }
    });
    let identity = source.as_ref().map(mode_identity);
    let editable = identity.as_ref().is_some_and(|identity| current_edit_snapshot(&runtime, identity, workspace(), scope_generation(), instance_is_current()).is_some()
        && matches!(&*resolution.read(), PcbWiringResolution::Current {identity: current, ..} if current == &identity.plan));
    let target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let feedback = feedback().filter(|item| target.as_ref() == Some(&item.target));
    BoardWiringModeActions {
        identity,
        editable,
        feedback,
        on_change,
    }
}

fn mode_resolver(request: BoardWiringModeEditRequest) -> EditResolver {
    EditResolver::new("pcb-wiring-mode", move |accepted: &AcceptedSnapshot| {
        let board_id = &request.identity.plan.scope.board_id;
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| &board.id == board_id)
        {
            return Resolution::Retire("The board was deleted.".into());
        }
        let current_mode = accepted
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .boards
                    .iter()
                    .find(|configuration| configuration.board_id == *board_id)
            })
            .map_or(ElectricalMode::Matrix, |configuration| configuration.mode);
        if current_mode == request.mode {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![board_id.clone()],
            EditOperation::SetWiringMode {
                board_id: board_id.clone(),
                mode: request.mode,
            },
        )
    })
}

pub fn mode_identity(source: &PcbWiringSource) -> BoardWiringModeIdentity {
    BoardWiringModeIdentity {
        plan: source.identity.clone(),
        ui_scope: source.ui_scope.clone(),
        selected_part_id: source.active_part_id.clone(),
        scope_generation: source.scope_generation,
    }
}

pub fn current_snapshot(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: &str,
    scope_generation: u64,
    instance_is_current: bool,
) -> Option<boardstudio_application::AcceptedSnapshot> {
    current_snapshot_probe(
        runtime,
        identity,
        workspace,
        scope_generation,
        instance_is_current,
    )
    .ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurrentSnapshotBlocker {
    Workspace,
    InstanceSelection,
    MissingAcceptedSnapshot,
    Lifecycle,
    Preview,
    Gesture,
    Durability,
    ActiveBoard,
    ActiveInstance,
    SessionEpoch,
    DocumentId,
    ActionContext,
    BoardMissing,
}

/// Private diagnostic seam used by the mounted owner regression probe. Production admission
/// uses this same predicate and discards the reason; no runtime state or debug API is exposed.
pub fn current_snapshot_probe(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: &str,
    scope_generation: u64,
    instance_is_current: bool,
) -> Result<boardstudio_application::AcceptedSnapshot, CurrentSnapshotBlocker> {
    snapshot_probe(
        runtime,
        identity,
        workspace,
        scope_generation,
        instance_is_current,
        false,
    )
}

pub(super) fn current_edit_snapshot(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: &str,
    scope_generation: u64,
    instance_is_current: bool,
) -> Option<boardstudio_application::AcceptedSnapshot> {
    snapshot_probe(
        runtime,
        identity,
        workspace,
        scope_generation,
        instance_is_current,
        true,
    )
    .ok()
}

fn snapshot_probe(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: &str,
    scope_generation: u64,
    instance_is_current: bool,
    queued: bool,
) -> Result<boardstudio_application::AcceptedSnapshot, CurrentSnapshotBlocker> {
    if workspace != "PCB" {
        return Err(CurrentSnapshotBlocker::Workspace);
    }
    if !instance_is_current {
        return Err(CurrentSnapshotBlocker::InstanceSelection);
    }
    let model = runtime.model();
    let snapshot = model
        .accepted
        .ok_or(CurrentSnapshotBlocker::MissingAcceptedSnapshot)?;
    let current_scope = runtime.scope();
    let current_plan = WiringPlanIdentity {
        scope: Scope {
            instance_id: None,
            ..identity.ui_scope.clone()
        },
        token: snapshot.token,
        revision: snapshot.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    if !(model.lifecycle == Lifecycle::Ready
        || queued && matches!(model.lifecycle, Lifecycle::Applying | Lifecycle::Saving))
    {
        return Err(CurrentSnapshotBlocker::Lifecycle);
    }
    if model.display_preview.is_some() {
        return Err(CurrentSnapshotBlocker::Preview);
    }
    if model.gesture.is_some() {
        return Err(CurrentSnapshotBlocker::Gesture);
    }
    if !(model.durability
        == (Durability::Saved {
            revision: snapshot.document.revision,
        })
        || queued && matches!(model.durability, Durability::Saving { .. }))
    {
        return Err(CurrentSnapshotBlocker::Durability);
    }
    if model.active_board_id != identity.ui_scope.board_id {
        return Err(CurrentSnapshotBlocker::ActiveBoard);
    }
    if model.active_instance_id != identity.ui_scope.instance_id {
        return Err(CurrentSnapshotBlocker::ActiveInstance);
    }
    if snapshot.session_epoch != identity.ui_scope.session_epoch {
        return Err(CurrentSnapshotBlocker::SessionEpoch);
    }
    if snapshot.document.id != identity.ui_scope.document_id {
        return Err(CurrentSnapshotBlocker::DocumentId);
    }
    if !identity.matches_action_context(
        &current_plan,
        current_scope.as_ref(),
        model.selected_part_ids.first().map(String::as_str),
        scope_generation,
    ) {
        return Err(CurrentSnapshotBlocker::ActionContext);
    }
    if !snapshot
        .document
        .boards
        .iter()
        .any(|board| board.id == identity.plan.scope.board_id)
    {
        return Err(CurrentSnapshotBlocker::BoardMissing);
    }
    Ok(snapshot)
}
