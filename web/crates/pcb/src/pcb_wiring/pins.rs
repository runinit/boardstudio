//! Accepted selected-board assignment pin and lock edits.
use super::mode::{current_edit_snapshot, mode_identity};
use super::{PcbWiringResolution, PcbWiringSource};
use crate::{
    pcb_wiring_mode_operation::{BoardWiringModeFeedbackTarget, BoardWiringModeIdentity},
    runtime::Runtime,
};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan},
    model::{EditOperation, ProjectDoc},
};
use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

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

#[derive(Clone, PartialEq)]
pub struct PcbWiringPinActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    /// A pin edit for the current board target is still pending.
    pub pending: bool,
    /// The latest pin edit's inline failure; landing and retirement are silent.
    pub failure: Signal<Option<String>>,
    pub on_change: EventHandler<PcbWiringPinEditRequest>,
}

/// One pin control's bounded logical key: the rendered board target plus its assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PcbWiringPinKey {
    target: BoardWiringModeFeedbackTarget,
    assignment_id: String,
}

impl PcbWiringPinKey {
    fn new(identity: &BoardWiringModeIdentity, assignment_id: &str) -> Self {
        Self {
            target: identity.feedback_target(),
            assignment_id: assignment_id.to_owned(),
        }
    }
}

#[derive(Clone)]
struct PinOwner {
    edits: Rc<RefCell<PendingEdits<PcbWiringPinKey>>>,
    /// The latest submitted pin per key: the domain projection the pending select shows.
    submissions: Signal<Vec<(PcbWiringPinKey, Option<String>)>>,
    /// The newest submitted key, so only its outcome places the panel's message.
    latest: Signal<Option<PcbWiringPinKey>>,
    failure: Signal<Option<String>>,
}

pub(super) fn pending_pin(
    identity: &BoardWiringModeIdentity,
    assignment_id: &str,
) -> Option<Option<String>> {
    let owner = try_consume_context::<PinOwner>()?;
    let key = PcbWiringPinKey::new(identity, assignment_id);
    if !owner.edits.borrow().is_pending(&key) {
        return None;
    }
    owner
        .submissions
        .read()
        .iter()
        .rev()
        .find_map(|(existing, pin)| (existing == &key).then(|| pin.clone()))
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
    let edits = use_hook(|| Rc::new(RefCell::new(PendingEdits::<PcbWiringPinKey>::default())));
    let submissions = use_signal(Vec::<(PcbWiringPinKey, Option<String>)>::new);
    let latest = use_signal(|| None::<PcbWiringPinKey>);
    let failure = use_signal(|| None::<String>);
    let settlement_tick = use_signal(|| 0u64);
    let _ = settlement_tick();
    use_context_provider(|| PinOwner {
        edits: edits.clone(),
        submissions,
        latest,
        failure,
    });
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let edits = edits.clone();
        let mut submissions = submissions;
        let latest = latest;
        let mut failure = failure;
        let mut settlement_tick = settlement_tick;
        move |_| {
            // Keyed submissions are the pending select projection; the collection owns the
            // ticket lifetime, and each ticket retires itself when its Scope departs.
            let results = edits.borrow_mut().settle(workspace() == "PCB");
            if results.is_empty() {
                return;
            }
            for result in results {
                match result {
                    PendingEditResult::Failed { key, message } => {
                        submissions.write().retain(|(existing, _)| existing != &key);
                        if latest.peek().as_ref() == Some(&key) {
                            failure.set(Some(message));
                        }
                    }
                    PendingEditResult::Landed { key, .. } | PendingEditResult::Retired { key } => {
                        submissions.write().retain(|(existing, _)| existing != &key);
                        if latest.peek().as_ref() == Some(&key) {
                            failure.set(None);
                        }
                    }
                }
            }
            settlement_tick.set(settlement_tick().wrapping_add(1));
        }
    }));
    let on_change = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let edits = edits.clone();
        let mut submissions = submissions;
        let mut latest = latest;
        let mut failure = failure;
        move |request: PcbWiringPinEditRequest| {
            let Some(snapshot) = current_edit_snapshot(
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
            }
            if !pin_edit_is_available(
                &snapshot.document,
                _plan,
                &request.identity.plan.scope.board_id,
                &request.assignment_id,
                request.pin.as_deref(),
            ) {
                return;
            }
            let key = PcbWiringPinKey::new(&request.identity, &request.assignment_id);
            latest.set(Some(key.clone()));
            failure.set(None);
            let mut entries = submissions.write();
            match entries.iter_mut().find(|(existing, _)| existing == &key) {
                Some(entry) => entry.1 = request.pin.clone(),
                None => entries.push((key.clone(), request.pin.clone())),
            }
            drop(entries);
            edits.borrow_mut().begin(
                &runtime,
                key,
                "pcb-wiring-pins",
                Some("wiring pin".into()),
                pins_resolver(request),
            );
        }
    });
    let identity = source.as_ref().map(mode_identity);
    let editable = identity.as_ref().is_some_and(|identity| current_edit_snapshot(&runtime, identity, workspace(), scope_generation(), instance_is_current()).is_some()
        && matches!(&*resolution.read(), PcbWiringResolution::Current {identity: current, ..} if current == &identity.plan));
    let target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let pending = target.as_ref().is_some_and(|target| {
        submissions.read().iter().any(|(key, _)| {
            &key.target == target && edits.borrow().is_pending(key)
        })
    });
    PcbWiringPinActions {
        identity,
        editable,
        pending,
        failure,
        on_change,
    }
}

fn pins_resolver(request: PcbWiringPinEditRequest) -> EditResolver {
    EditResolver::new("pcb-wiring-pins", move |accepted: &AcceptedSnapshot| {
        let board_id = &request.identity.plan.scope.board_id;
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| &board.id == board_id)
        {
            return Resolution::Retire("The board was deleted.".into());
        }
        let boardstudio_core::model::CoreRequest::ResolveElectrical {
            request: electrical_request,
            ..
        } = crate::pcb_wiring_mode_operation::electrical_preview_request(
            "pin-edit",
            &accepted.document,
            board_id,
        )
        else {
            unreachable!()
        };
        let plan = boardstudio_core::electrical::resolve(electrical_request);
        if !pin_edit_is_available(
            &accepted.document,
            &plan,
            board_id,
            &request.assignment_id,
            request.pin.as_deref(),
        ) {
            if request.pin.is_none()
                && !accepted.document.hardware.as_ref().is_some_and(|hardware| {
                    hardware.boards.iter().any(|board| {
                        &board.board_id == board_id
                            && board.locks.contains_key(&request.assignment_id)
                    })
                })
            {
                return Resolution::Unchanged;
            }
            return Resolution::Retire(
                "The assignment or selected pin is no longer available on this board.".into(),
            );
        }
        let Some(proposal) = propose_pin_lock(
            &accepted.document,
            board_id,
            &request.assignment_id,
            request.pin.as_deref(),
        ) else {
            return Resolution::Unchanged;
        };
        if proposal == *accepted.document {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![board_id.clone()],
            EditOperation::ReplaceDocument {
                document: Box::new(proposal),
            },
        )
    })
}

pub fn assignments(source: &PcbWiringSource, plan: &ElectricalPlan) -> Vec<PcbWiringPinAssignment> {
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
            .map(|assignment| &assignment.column_pin)
            .or_else(|| plan.peripheral_terminals.get(assignment_id)),
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
