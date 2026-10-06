//! Accepted selected-board assignment pin and lock edits.
use super::mode::{current_snapshot, mode_identity};
use super::{PcbWiringResolution, PcbWiringSource};
use crate::{
    pcb_wiring_mode_operation::{BoardWiringModeFeedbackTarget, BoardWiringModeIdentity},
    runtime::Runtime,
};
use boardstudio_application::{Durability, Event, Lifecycle, TerminalOutcome};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan},
    model::{EditCommand, EditOperation, EditPhase, ProjectDoc},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcbWiringPinAssignment {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
    pub value: Option<String>,
    pub locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcbWiringPinEditRequest {
    pub identity: BoardWiringModeIdentity,
    pub assignment_id: String,
    /// The selected value is written into the existing Core lock map. `None` removes it.
    pub pin: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcbWiringPinFeedback {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcbWiringPinFeedbackView {
    pub target: BoardWiringModeFeedbackTarget,
    pub request_plan: super::WiringPlanIdentity,
    pub state: PcbWiringPinFeedback,
}

#[derive(Clone, PartialEq)]
pub struct PcbWiringPinActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    pub feedback: Option<PcbWiringPinFeedbackView>,
    pub on_change: EventHandler<PcbWiringPinEditRequest>,
}

#[derive(Clone)]
struct PendingPinEdit {
    request: PcbWiringPinEditRequest,
    proposal: ProjectDoc,
    base_revision: u64,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

pub fn use_pcb_wiring_pin_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
) -> PcbWiringPinActions {
    let pending = use_signal(|| None::<PendingPinEdit>);
    let feedback = use_signal(|| None::<PcbWiringPinFeedbackView>);
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
                TerminalOutcome::Completed if saved_proposal => PcbWiringPinFeedback::Saved,
                TerminalOutcome::Completed => PcbWiringPinFeedback::Failed(
                    "The pin edit completed, but the requested lock is not the saved board state. Resolve again before retrying.".into(),
                ),
                TerminalOutcome::PersistenceFailed(message) => {
                    PcbWiringPinFeedback::Failed(format!("Could not save wiring pin: {message}"))
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::ExecutorFailed(message)
                | TerminalOutcome::BlockedByRecovery(message) => {
                    PcbWiringPinFeedback::Failed(format!("Wiring pin edit was rejected: {message}"))
                }
                TerminalOutcome::Cancelled
                | TerminalOutcome::Closed
                | TerminalOutcome::Superseded => PcbWiringPinFeedback::Failed(
                    "The wiring pin edit was cancelled before it could be saved.".into(),
                ),
            };
            feedback.set(Some(PcbWiringPinFeedbackView {
                target: waiting.request.identity.feedback_target(),
                request_plan: waiting.request.identity.plan.clone(),
                state,
            }));
        }
    }));

    let on_change = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: PcbWiringPinEditRequest| {
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
            let PcbWiringResolution::Current { identity, plan } = &*resolution.read() else {
                return;
            };
            if identity != &request.identity.plan
                || !pin_edit_is_available(
                    &snapshot.document,
                    plan,
                    &request.identity.plan.scope.board_id,
                    &request.assignment_id,
                    request.pin.as_deref(),
                )
            {
                return;
            }
            let Some(proposal) = propose_pin_lock(
                &snapshot.document,
                &request.identity.plan.scope.board_id,
                &request.assignment_id,
                request.pin.as_deref(),
            ) else {
                return;
            };
            if proposal == *snapshot.document {
                return;
            }
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let base_revision = snapshot.document.revision;
            pending.set(Some(PendingPinEdit {
                request: request.clone(),
                proposal: proposal.clone(),
                base_revision,
                outcome,
            }));
            feedback.set(Some(PcbWiringPinFeedbackView {
                target: request.identity.feedback_target(),
                request_plan: request.identity.plan.clone(),
                state: PcbWiringPinFeedback::Pending,
            }));
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision,
                    transaction_id: format!("pcb-wiring-pin-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![request.identity.plan.scope.board_id.clone()],
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
            .is_some_and(|_| {
                matches!(
                    &*resolution.read(),
                    PcbWiringResolution::Current { identity: current, .. }
                        if current == &identity.plan
                )
            })
    });
    let feedback_target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let feedback = feedback().filter(|item| {
        feedback_target.as_ref() == Some(&item.target)
            && (matches!(&item.state, PcbWiringPinFeedback::Saved)
                || identity
                    .as_ref()
                    .is_some_and(|identity| identity.plan == item.request_plan))
    });
    PcbWiringPinActions {
        identity,
        editable,
        feedback,
        on_change,
    }
}

pub fn assignments(
    source: &PcbWiringSource,
    plan: &ElectricalPlan,
) -> Vec<PcbWiringPinAssignment> {
    let board_id = &source.identity.scope.board_id;
    let configuration = source.document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|item| &item.board_id == board_id)
    });
    let locked = |id: &str| {
        configuration
            .and_then(|item| item.locks.get(id))
            .is_some_and(|pin| !pin.is_empty())
    };
    let mut rows = Vec::new();
    match plan.mode {
        ElectricalMode::Matrix => {
            rows.extend(plan.row_pins.iter().enumerate().map(|(index, pin)| {
                let id = format!("row/{index}");
                PcbWiringPinAssignment {
                    label: format!("Row {}", index + 1),
                    locked: locked(&id),
                    id,
                    detail: None,
                    value: (!pin.is_empty()).then(|| pin.clone()),
                }
            }));
            rows.extend(plan.column_pins.iter().enumerate().map(|(index, pin)| {
                let id = format!("column/{index}");
                PcbWiringPinAssignment {
                    label: format!("Column {}", index + 1),
                    locked: locked(&id),
                    id,
                    detail: None,
                    value: (!pin.is_empty()).then(|| pin.clone()),
                }
            }));
        }
        ElectricalMode::Direct => {
            rows.extend(plan.assignments.iter().map(|assignment| {
                let id = assignment.key_id.clone();
                PcbWiringPinAssignment {
                    label: source
                        .document
                        .parts
                        .iter()
                        .find(|part| part.id == id)
                        .map_or_else(|| id.clone(), |part| part.reference.clone()),
                    locked: locked(&id),
                    id,
                    detail: Some(
                        assignment
                            .direct_gpio
                            .clone()
                            .unwrap_or_else(|| "Unresolved".into()),
                    ),
                    value: (!assignment.column_pin.is_empty())
                        .then(|| assignment.column_pin.clone()),
                }
            }));
        }
    }
    rows.extend(
        plan.peripheral_terminals
            .iter()
            .map(|(id, pin)| PcbWiringPinAssignment {
                label: peripheral_label(id),
                detail: plan.peripheral_pins.get(id).cloned(),
                value: (!pin.is_empty()).then(|| pin.clone()),
                locked: locked(id),
                id: id.clone(),
            }),
    );
    rows
}

pub fn pin_choices(row: &PcbWiringPinAssignment, plan: &ElectricalPlan) -> Vec<String> {
    let mut pins = Vec::new();
    if let Some(value) = row.value.as_ref() {
        pins.push(value.clone());
    }
    for value in &plan.free_pins {
        if !pins.contains(value) {
            pins.push(value.clone());
        }
    }
    pins
}

pub fn pin_edit_is_available(
    document: &ProjectDoc,
    plan: &ElectricalPlan,
    board_id: &str,
    assignment_id: &str,
    pin: Option<&str>,
) -> bool {
    if pin.is_none() {
        return document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .boards
                    .iter()
                    .find(|configuration| configuration.board_id == board_id)
            })
            .is_some_and(|configuration| configuration.locks.contains_key(assignment_id));
    }
    let current = match assignment_id {
        value if value.starts_with("row/") && plan.mode == ElectricalMode::Matrix => value[4..]
            .parse::<usize>()
            .ok()
            .and_then(|index| plan.row_pins.get(index)),
        value if value.starts_with("column/") && plan.mode == ElectricalMode::Matrix => value[7..]
            .parse::<usize>()
            .ok()
            .and_then(|index| plan.column_pins.get(index)),
        _ if plan.mode == ElectricalMode::Direct => plan
            .assignments
            .iter()
            .find(|assignment| assignment.key_id == assignment_id)
            .map(|assignment| &assignment.column_pin),
        _ => plan.peripheral_terminals.get(assignment_id),
    };
    let Some(current) = current else {
        return false;
    };
    match pin {
        Some("") => true,
        Some(value) => value == current || plan.free_pins.iter().any(|free| free == value),
        None => false,
    }
}

pub fn propose_pin_lock(
    document: &ProjectDoc,
    board_id: &str,
    assignment_id: &str,
    pin: Option<&str>,
) -> Option<ProjectDoc> {
    document
        .boards
        .iter()
        .any(|board| board.id == board_id)
        .then_some(())?;
    let mut proposal = document.clone();
    let hardware = proposal.hardware.get_or_insert_with(Default::default);
    let index = hardware
        .boards
        .iter()
        .position(|configuration| configuration.board_id == board_id);
    let configuration = if let Some(index) = index {
        &mut hardware.boards[index]
    } else {
        hardware.boards.push(Default::default());
        let configuration = hardware.boards.last_mut()?;
        configuration.board_id = board_id.to_owned();
        configuration
    };
    match pin {
        Some(pin) => {
            configuration
                .locks
                .insert(assignment_id.to_owned(), pin.to_owned());
        }
        None => {
            configuration.locks.remove(assignment_id)?;
        }
    }
    Some(proposal)
}

fn peripheral_label(id: &str) -> String {
    id.strip_prefix("peripheral/")
        .map(|name| {
            name.rsplit('/')
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_else(|| id.to_owned())
}
