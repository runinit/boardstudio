//! Apply the exact current accepted board plan through the normal Session edit owner.
use super::mode::{current_snapshot, mode_identity};
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::pcb_wiring_mode_operation::BoardWiringModeIdentity;
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Event, Lifecycle, TerminalOutcome};
use boardstudio_core::{
    electrical::{self, ElectricalPlan},
    model::{EditCommand, EditOperation, EditPhase, ProjectDoc},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardWiringApplyFeedback {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringApplyFeedbackView {
    pub target: crate::pcb_wiring_mode_operation::BoardWiringModeFeedbackTarget,
    pub request_plan: WiringPlanIdentity,
    pub state: BoardWiringApplyFeedback,
}

#[derive(Clone, PartialEq)]
pub struct BoardWiringApplyActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    pub feedback: Option<BoardWiringApplyFeedbackView>,
    pub on_apply: EventHandler<BoardWiringModeIdentity>,
    pub on_release_reviewed_connections: EventHandler<BoardWiringModeIdentity>,
}

#[derive(Clone)]
struct PendingApply {
    identity: BoardWiringModeIdentity,
    proposal: ProjectDoc,
    base_revision: u64,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

pub fn use_board_wiring_apply(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
) -> BoardWiringApplyActions {
    let pending = use_signal(|| None::<PendingApply>);
    let feedback = use_signal(|| None::<BoardWiringApplyFeedbackView>);
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
                accepted.session_epoch == waiting.identity.ui_scope.session_epoch
                    && accepted.document.id == waiting.identity.ui_scope.document_id
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
            let target_current = waiting.identity.feedback_target().is_visible(
                runtime.scope().as_ref(),
                model.selected_part_ids.first().map(String::as_str),
                scope_generation(),
            ) && model.active_board_id == waiting.identity.ui_scope.board_id
                && model.active_instance_id == waiting.identity.ui_scope.instance_id
                && accepted.is_some_and(|accepted| {
                    saved_proposal
                        || (accepted.token == waiting.identity.plan.token
                            && accepted.document.revision == waiting.identity.plan.revision)
                });
            pending.set(None);
            if !target_current {
                feedback.set(None);
                return;
            }
            let state = match outcome {
                TerminalOutcome::Completed if saved_proposal => BoardWiringApplyFeedback::Saved,
                TerminalOutcome::Completed => BoardWiringApplyFeedback::Failed(
                    "The plan edit completed, but the requested wiring is not the saved board state. Resolve again before applying.".into(),
                ),
                TerminalOutcome::PersistenceFailed(message) => {
                    BoardWiringApplyFeedback::Failed(format!("Could not save wiring plan: {message}"))
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::ExecutorFailed(message)
                | TerminalOutcome::BlockedByRecovery(message) => {
                    BoardWiringApplyFeedback::Failed(format!("Wiring plan was rejected: {message}"))
                }
                TerminalOutcome::Cancelled
                | TerminalOutcome::Closed
                | TerminalOutcome::Superseded => BoardWiringApplyFeedback::Failed(
                    "Applying the wiring plan was cancelled before it could be saved.".into(),
                ),
            };
            feedback.set(Some(BoardWiringApplyFeedbackView {
                target: waiting.identity.feedback_target(),
                request_plan: waiting.identity.plan.clone(),
                state,
            }));
        }
    }));

    let on_apply = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |identity: BoardWiringModeIdentity| {
            if pending.peek().is_some() {
                return;
            }
            let Some(snapshot) = current_snapshot(
                &runtime,
                &identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let Some(plan) = current_plan(&identity.plan, &resolution.read(), &snapshot.document)
            else {
                return;
            };
            let mut proposal = (*snapshot.document).clone();
            if let Err(message) = electrical::materialize(&mut proposal, &plan) {
                feedback.set(Some(BoardWiringApplyFeedbackView {
                    target: identity.feedback_target(),
                    request_plan: identity.plan.clone(),
                    state: BoardWiringApplyFeedback::Failed(message),
                }));
                return;
            }

            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let base_revision = snapshot.document.revision;
            pending.set(Some(PendingApply {
                identity: identity.clone(),
                proposal: proposal.clone(),
                base_revision,
                outcome,
            }));
            feedback.set(Some(BoardWiringApplyFeedbackView {
                target: identity.feedback_target(),
                request_plan: identity.plan.clone(),
                state: BoardWiringApplyFeedback::Pending,
            }));
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision,
                    transaction_id: format!("pcb-wiring-apply-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![identity.plan.scope.board_id.clone()],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(proposal),
                    },
                },
            });
        }
    });

    let on_release_reviewed_connections = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        move |identity: BoardWiringModeIdentity| {
            let Some(snapshot) = current_snapshot(
                &runtime,
                &identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let Some(plan) =
                current_review_plan(&identity.plan, &resolution.read(), &snapshot.document)
            else {
                return;
            };
            let Some(review) =
                super::connections::existing_connection_review(&snapshot.document, &plan)
            else {
                return;
            };
            let proposal =
                super::connections::release_reviewed_connections(&snapshot.document, &review);
            if proposal == *snapshot.document {
                return;
            }
            let operation_id = runtime.operation();
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id: format!("pcb-review-connections-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![identity.plan.scope.board_id.clone()],
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
            .is_some_and(|snapshot| {
                current_plan(&identity.plan, &resolution.read(), &snapshot.document).is_some()
            })
    });
    let feedback_target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let feedback = feedback().filter(|item| {
        feedback_target.as_ref() == Some(&item.target)
            && (matches!(&item.state, BoardWiringApplyFeedback::Saved)
                || identity
                    .as_ref()
                    .is_some_and(|identity| identity.plan == item.request_plan))
    });
    BoardWiringApplyActions {
        identity,
        editable,
        feedback,
        on_apply,
        on_release_reviewed_connections,
    }
}

fn current_plan(
    identity: &WiringPlanIdentity,
    resolution: &PcbWiringResolution,
    document: &ProjectDoc,
) -> Option<Rc<ElectricalPlan>> {
    current_review_plan(identity, resolution, document).filter(|plan| {
        !plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error")
    })
}

fn current_review_plan(
    identity: &WiringPlanIdentity,
    resolution: &PcbWiringResolution,
    document: &ProjectDoc,
) -> Option<Rc<ElectricalPlan>> {
    let PcbWiringResolution::Current {
        identity: current,
        plan,
    } = resolution
    else {
        return None;
    };
    if current != identity
        || plan.revision != identity.revision
        || plan.board_id.as_deref() != Some(identity.scope.board_id.as_str())
        || plan.instance_id.is_some()
        || plan.mode != board_mode(document, &identity.scope.board_id)
    {
        return None;
    }
    Some(plan.clone())
}

fn board_mode(
    document: &ProjectDoc,
    board_id: &str,
) -> boardstudio_core::electrical::ElectricalMode {
    document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == board_id)
        })
        .map_or(
            boardstudio_core::electrical::ElectricalMode::Matrix,
            |configuration| configuration.mode,
        )
}
