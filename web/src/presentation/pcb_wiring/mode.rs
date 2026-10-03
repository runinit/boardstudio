//! Accepted board-level mode edits for PCB wiring.
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::{
    pcb_wiring_mode_operation::{
        BoardWiringModeFeedbackTarget, BoardWiringModeIdentity, propose_mode,
    },
    runtime::Runtime,
};
use boardstudio_application::{Durability, Event, Lifecycle, Scope, TerminalOutcome};
use boardstudio_core::{
    electrical::ElectricalMode,
    model::{EditCommand, EditOperation, EditPhase, ProjectDoc},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct BoardWiringModeEditRequest {
    pub identity: BoardWiringModeIdentity,
    pub mode: ElectricalMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum BoardWiringModeFeedback {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct BoardWiringModeFeedbackView {
    pub target: BoardWiringModeFeedbackTarget,
    pub state: BoardWiringModeFeedback,
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct BoardWiringModeActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    pub feedback: Option<BoardWiringModeFeedbackView>,
    pub on_change: EventHandler<BoardWiringModeEditRequest>,
}

#[derive(Clone)]
struct PendingModeEdit {
    request: BoardWiringModeEditRequest,
    proposal: ProjectDoc,
    base_revision: u64,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

pub(in crate::presentation) fn use_board_wiring_mode_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
) -> BoardWiringModeActions {
    let pending = use_signal(|| None::<PendingModeEdit>);
    let feedback = use_signal(|| None::<BoardWiringModeFeedbackView>);
    let observed_version = version();

    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow_mut().take() else {
                return;
            };
            let model = runtime.model();
            let accepted = model.accepted.as_ref().filter(|accepted| {
                accepted.session_epoch == waiting.request.identity.ui_scope.session_epoch
                    && accepted.document.id == waiting.request.identity.ui_scope.document_id
            });
            let saved_proposal = accepted.is_some_and(|accepted| {
                let mut expected = waiting.proposal.clone();
                expected.revision = accepted.document.revision;
                model.lifecycle == Lifecycle::Ready
                    && model.durability
                        == (Durability::Saved {
                            revision: accepted.document.revision,
                        })
                    && accepted.document.revision > waiting.base_revision
                    && *accepted.document == expected
            });
            let target_current = waiting.request.identity.feedback_target().is_visible(
                runtime.scope().as_ref(),
                model.selected_part_ids.first().map(String::as_str),
                scope_generation(),
            ) && model.active_board_id
                == waiting.request.identity.ui_scope.board_id
                && model.active_instance_id == waiting.request.identity.ui_scope.instance_id
                && accepted.is_some_and(|accepted| {
                    saved_proposal
                        || (accepted.token == waiting.request.identity.plan.token
                            && accepted.document.revision == waiting.request.identity.plan.revision)
                });
            pending.set(None);
            if !target_current {
                feedback.set(None);
                return;
            }
            let state = match outcome {
                TerminalOutcome::Completed if saved_proposal => BoardWiringModeFeedback::Saved,
                TerminalOutcome::Completed => BoardWiringModeFeedback::Failed(
                    "The mode edit completed, but the requested mode is not the saved board state. Retry the change.".into(),
                ),
                TerminalOutcome::PersistenceFailed(message) => {
                    BoardWiringModeFeedback::Failed(format!("Could not save wiring mode: {message}"))
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::ExecutorFailed(message)
                | TerminalOutcome::BlockedByRecovery(message) => {
                    BoardWiringModeFeedback::Failed(format!("Wiring mode was rejected: {message}"))
                }
                TerminalOutcome::Cancelled
                | TerminalOutcome::Closed
                | TerminalOutcome::Superseded => BoardWiringModeFeedback::Failed(
                    "The wiring mode change was cancelled before it could be saved.".into(),
                ),
            };
            feedback.set(Some(BoardWiringModeFeedbackView {
                target: waiting.request.identity.feedback_target(),
                state,
            }));
        }
    }));

    let on_change = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: BoardWiringModeEditRequest| {
            if pending.peek().is_some() {
                return;
            }
            let Some(snapshot) = current_snapshot(
                &runtime,
                &request.identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let PcbWiringResolution::Current { identity, .. } = &*resolution.read() else {
                return;
            };
            if identity != &request.identity.plan {
                return;
            }
            let Some(proposal) = propose_mode(
                &snapshot.document,
                &request.identity.plan.scope.board_id,
                request.mode,
            ) else {
                return;
            };
            if proposal == *snapshot.document {
                return;
            }
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let base_revision = snapshot.document.revision;
            pending.set(Some(PendingModeEdit {
                request: request.clone(),
                proposal: proposal.clone(),
                base_revision,
                outcome,
            }));
            feedback.set(Some(BoardWiringModeFeedbackView {
                target: request.identity.feedback_target(),
                state: BoardWiringModeFeedback::Pending,
            }));
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision,
                    transaction_id: format!("pcb-wiring-mode-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![request.identity.plan.scope.board_id],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(proposal),
                    },
                },
            });
        }
    });

    let identity = source.as_ref().map(mode_identity);
    let editable = identity.as_ref().is_some_and(|identity| {
        workspace() == "PCB"
            && pending().is_none()
            && current_snapshot(
                &runtime,
                identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            )
            .is_some()
            && matches!(
                &*resolution.read(),
                PcbWiringResolution::Current { identity: current, .. } if current == &identity.plan
            )
    });
    let feedback_target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let feedback = feedback().filter(|item| feedback_target.as_ref() == Some(&item.target));
    BoardWiringModeActions {
        identity,
        editable,
        feedback,
        on_change,
    }
}

fn mode_identity(source: &PcbWiringSource) -> BoardWiringModeIdentity {
    BoardWiringModeIdentity {
        plan: source.identity.clone(),
        ui_scope: source.ui_scope.clone(),
        selected_part_id: source.active_part_id.clone(),
        scope_generation: source.scope_generation,
    }
}

fn current_snapshot(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: &str,
    scope_generation: u64,
    instance_is_current: bool,
) -> Option<boardstudio_application::AcceptedSnapshot> {
    if workspace != "PCB" || !instance_is_current {
        return None;
    }
    let model = runtime.model();
    let snapshot = model.accepted?;
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
    if model.lifecycle != Lifecycle::Ready
        || model.display_preview.is_some()
        || model.gesture.is_some()
        || model.durability
            != (Durability::Saved {
                revision: snapshot.document.revision,
            })
        || model.active_board_id != identity.ui_scope.board_id
        || model.active_instance_id != identity.ui_scope.instance_id
        || snapshot.session_epoch != identity.ui_scope.session_epoch
        || snapshot.document.id != identity.ui_scope.document_id
        || !identity.matches_action_context(
            &current_plan,
            current_scope.as_ref(),
            model.selected_part_ids.first().map(String::as_str),
            scope_generation,
        )
        || !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == identity.plan.scope.board_id)
    {
        return None;
    }
    Some(snapshot)
}
