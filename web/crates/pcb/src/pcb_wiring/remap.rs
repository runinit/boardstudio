//! Deliberate release of a board's protected electrical handoff before remapping.
use super::mode::{BoardWiringModeFeedbackTarget, BoardWiringModeIdentity};
use super::{
    PcbWiringSource,
    mode::{current_edit_snapshot, mode_identity},
};
use crate::runtime::Runtime;
use boardstudio_application::{Event, TerminalOutcome};
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedRemapIdentity {
    pub wiring: BoardWiringModeIdentity,
    pub expected_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtectedRemapFeedback {
    Applied,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedRemapFeedbackView {
    pub target: BoardWiringModeFeedbackTarget,
    pub request_fingerprint: String,
    pub state: ProtectedRemapFeedback,
}

#[derive(Clone, PartialEq)]
pub struct ProtectedRemapActions {
    pub identity: Option<ProtectedRemapIdentity>,
    pub handoff_revision: Option<u64>,
    pub editable: bool,
    pub feedback: Option<ProtectedRemapFeedbackView>,
    pub on_review: EventHandler<ProtectedRemapIdentity>,
}

pub fn use_protected_remap_review(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    source: Option<PcbWiringSource>,
) -> ProtectedRemapActions {
    let _ = version();
    let mut feedback = use_signal(|| None::<ProtectedRemapFeedbackView>);
    let on_review = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        move |request: ProtectedRemapIdentity| {
            let Some(snapshot) = current_edit_snapshot(
                &runtime,
                &request.wiring,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let target = request.wiring.feedback_target();
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            runtime.submit(Event::ReviewElectricalRemap {
                operation_id,
                base_revision: snapshot.document.revision,
                board_id: request.wiring.plan.scope.board_id,
                expected_fingerprint: request.expected_fingerprint.clone(),
            });
            let state = match outcome.borrow_mut().take() {
                Some(TerminalOutcome::Completed) => ProtectedRemapFeedback::Applied,
                Some(
                    TerminalOutcome::Rejected(message)
                    | TerminalOutcome::ExecutorFailed(message)
                    | TerminalOutcome::PersistenceFailed(message),
                ) => ProtectedRemapFeedback::Failed(message),
                _ => ProtectedRemapFeedback::Failed(
                    "The new PCB revision could not be started.".into(),
                ),
            };
            feedback.set(Some(ProtectedRemapFeedbackView {
                target,
                request_fingerprint: request.expected_fingerprint,
                state,
            }));
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
        workspace() == Workspace::Pcb
            && current_edit_snapshot(
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
            && (matches!(&item.state, ProtectedRemapFeedback::Applied)
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
) -> Option<boardstudio_model::model::ElectricalHandoffBaseline> {
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
