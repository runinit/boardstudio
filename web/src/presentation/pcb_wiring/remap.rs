//! Deliberate release of a board's protected electrical handoff before remapping.
use super::{
    PcbWiringSource,
    mode::{current_snapshot, mode_identity},
};
use crate::pcb_wiring_mode_operation::{BoardWiringModeFeedbackTarget, BoardWiringModeIdentity};
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Event, Lifecycle, TerminalOutcome};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, ProjectDoc};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct ProtectedRemapIdentity {
    pub wiring: BoardWiringModeIdentity,
    pub expected_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum ProtectedRemapFeedback {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct ProtectedRemapFeedbackView {
    pub target: BoardWiringModeFeedbackTarget,
    pub request_fingerprint: String,
    pub state: ProtectedRemapFeedback,
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct ProtectedRemapActions {
    pub identity: Option<ProtectedRemapIdentity>,
    pub handoff_revision: Option<u64>,
    pub editable: bool,
    pub feedback: Option<ProtectedRemapFeedbackView>,
    pub on_review: EventHandler<ProtectedRemapIdentity>,
}

#[derive(Clone)]
struct PendingProtectedRemap {
    request: ProtectedRemapIdentity,
    proposal: ProjectDoc,
    base_revision: u64,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

pub(in crate::presentation) fn use_protected_remap_review(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    source: Option<PcbWiringSource>,
) -> ProtectedRemapActions {
    let pending = use_signal(|| None::<PendingProtectedRemap>);
    let feedback = use_signal(|| None::<ProtectedRemapFeedbackView>);
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
                accepted.session_epoch == waiting.request.wiring.ui_scope.session_epoch
                    && accepted.document.id == waiting.request.wiring.ui_scope.document_id
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
            let target = waiting.request.wiring.feedback_target();
            let target_current = target.is_visible(
                runtime.scope().as_ref(),
                model.selected_part_ids.first().map(String::as_str),
                scope_generation(),
            ) && model.active_board_id
                == waiting.request.wiring.ui_scope.board_id
                && model.active_instance_id == waiting.request.wiring.ui_scope.instance_id
                && accepted.is_some_and(|accepted| {
                    saved_proposal
                        || (accepted.token == waiting.request.wiring.plan.token
                            && accepted.document.revision == waiting.request.wiring.plan.revision)
                });
            pending.set(None);
            if !target_current {
                feedback.set(None);
                return;
            }
            let state = match outcome {
                TerminalOutcome::Completed if saved_proposal => ProtectedRemapFeedback::Saved,
                TerminalOutcome::Completed => ProtectedRemapFeedback::Failed(
                    "The remap review completed, but the new revision was not saved. Review the current board and retry.".into(),
                ),
                TerminalOutcome::PersistenceFailed(message) => {
                    ProtectedRemapFeedback::Failed(format!("Could not save the new PCB revision: {message}"))
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::ExecutorFailed(message)
                | TerminalOutcome::BlockedByRecovery(message) => {
                    ProtectedRemapFeedback::Failed(format!("The remap review was rejected: {message}"))
                }
                TerminalOutcome::Cancelled
                | TerminalOutcome::Closed
                | TerminalOutcome::Superseded => ProtectedRemapFeedback::Failed(
                    "The remap review was cancelled before the new revision was saved.".into(),
                ),
            };
            feedback.set(Some(ProtectedRemapFeedbackView {
                target,
                request_fingerprint: waiting.request.expected_fingerprint,
                state,
            }));
        }
    }));

    let on_review = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: ProtectedRemapIdentity| {
            if pending.peek().is_some() {
                return;
            }
            let Some(snapshot) = current_snapshot(
                &runtime,
                &request.wiring,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let Some(proposal) = crate::pcb_wiring_remap_operation::propose_review_remap(
                &snapshot.document,
                &request.wiring.plan.scope.board_id,
                &request.expected_fingerprint,
            ) else {
                return;
            };
            if proposal == *snapshot.document {
                return;
            }
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let base_revision = snapshot.document.revision;
            let target = request.wiring.feedback_target();
            pending.set(Some(PendingProtectedRemap {
                request: request.clone(),
                proposal: proposal.clone(),
                base_revision,
                outcome,
            }));
            feedback.set(Some(ProtectedRemapFeedbackView {
                target,
                request_fingerprint: request.expected_fingerprint.clone(),
                state: ProtectedRemapFeedback::Pending,
            }));
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision,
                    transaction_id: format!("pcb-remap-review-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![request.wiring.plan.scope.board_id],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(proposal),
                    },
                },
            });
        }
    });

    let base_identity = source.as_ref().map(mode_identity);
    let handoff = source.as_ref().and_then(protected_handoff);
    let identity = base_identity
        .clone()
        .zip(handoff.as_ref())
        .map(|(wiring, handoff)| ProtectedRemapIdentity {
            wiring,
            expected_fingerprint: handoff.fingerprint.clone(),
        });
    let editable = identity.as_ref().is_some_and(|identity| {
        workspace() == "PCB"
            && pending().is_none()
            && current_snapshot(
                &runtime,
                &identity.wiring,
                workspace(),
                scope_generation(),
                instance_is_current(),
            )
            .is_some()
    });
    let feedback_target = base_identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let feedback = feedback().filter(|item| {
        feedback_target.as_ref() == Some(&item.target)
            && (matches!(&item.state, ProtectedRemapFeedback::Saved)
                || identity.as_ref().is_some_and(|identity| {
                    identity.expected_fingerprint == item.request_fingerprint
                }))
    });
    ProtectedRemapActions {
        identity,
        handoff_revision: handoff.map(|handoff| handoff.revision),
        editable,
        feedback,
        on_review,
    }
}

fn protected_handoff(
    source: &PcbWiringSource,
) -> Option<boardstudio_core::model::ElectricalHandoffBaseline> {
    source
        .document
        .hardware
        .as_ref()?
        .boards
        .iter()
        .find(|configuration| configuration.board_id == source.identity.scope.board_id)?
        .protected_handoff
        .clone()
}
