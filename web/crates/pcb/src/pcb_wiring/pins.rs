//! Accepted selected-board assignment pin and lock edits.
#[cfg(target_arch = "wasm32")]
use super::PcbWiringResolution;
use super::PcbWiringSource;
use super::mode::BoardWiringModeIdentity;
#[cfg(target_arch = "wasm32")]
use super::mode::use_board_wiring_edit;
use boardstudio_application::EditResolver;
use boardstudio_core_mechanical::electrical::{ElectricalMode, ElectricalPlan};
use boardstudio_model::model::ProjectDoc;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::editor::EditGuard;
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcbWiringPinAssignment {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
    pub value: Option<String>,
    pub locked: bool,
}

pub(super) struct PcbWiringPinGroups<'a> {
    pub rows: Vec<&'a PcbWiringPinAssignment>,
    pub columns: Vec<&'a PcbWiringPinAssignment>,
    pub other: Vec<&'a PcbWiringPinAssignment>,
}

pub(super) fn group_assignments(assignments: &[PcbWiringPinAssignment]) -> PcbWiringPinGroups<'_> {
    let mut groups = PcbWiringPinGroups {
        rows: Vec::new(),
        columns: Vec::new(),
        other: Vec::new(),
    };
    for assignment in assignments {
        match assignment.id.split_once('/') {
            Some(("row", _)) => groups.rows.push(assignment),
            Some(("column", _)) => groups.columns.push(assignment),
            _ => groups.other.push(assignment),
        }
    }
    groups
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
    /// The current board target's inline failure; landing and retirement are silent.
    pub(crate) failure: Option<String>,
    pub on_change: EventHandler<PcbWiringPinEditRequest>,
}

#[cfg(target_arch = "wasm32")]
pub fn use_pcb_wiring_pin_edits(
    guard: EditGuard,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
) -> PcbWiringPinActions {
    let (identity, editable, failure, on_change) = use_board_wiring_edit(
        guard,
        &source,
        resolution,
        "pcb-wiring-pins",
        |snapshot, plan, request: PcbWiringPinEditRequest| {
            pin_edit_is_available(
                &snapshot.document,
                plan,
                &request.identity.plan.scope.board_id,
                &request.assignment_id,
                request.pin.as_deref(),
            )
            .then(|| pins_resolver(request))
        },
        |request: &PcbWiringPinEditRequest| request.identity.clone(),
    );
    PcbWiringPinActions {
        identity,
        editable,
        failure,
        on_change,
    }
}

fn pins_resolver(request: PcbWiringPinEditRequest) -> EditResolver {
    super::mode::replace_document_resolver("pcb-wiring-pins", move |accepted, replacement| {
        let board_id = &request.identity.plan.scope.board_id;
        if !replacement.boards.iter().any(|board| &board.id == board_id) {
            return Err("The board was deleted.".into());
        }
        let plan = super::mode::resolve_preview_plan(&accepted.document, board_id);
        if !pin_edit_is_available(
            &accepted.document,
            &plan,
            board_id,
            &request.assignment_id,
            request.pin.as_deref(),
        ) {
            // Clearing a lock that no longer exists is already the desired state:
            // leave the document untouched and the resolver resolves Unchanged.
            let lock_gone = request.pin.is_none()
                && !accepted.document.hardware.as_ref().is_some_and(|hardware| {
                    hardware.boards.iter().any(|board| {
                        &board.board_id == board_id
                            && board.locks.contains_key(&request.assignment_id)
                    })
                });
            return if lock_gone {
                Ok(Vec::new())
            } else {
                Err("The assignment or selected pin is no longer available on this board.".into())
            };
        }
        if let Some(proposal) = propose_pin_lock(
            &accepted.document,
            board_id,
            &request.assignment_id,
            request.pin.as_deref(),
        ) {
            *replacement = proposal;
        }
        Ok(vec![board_id.clone()])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_pins_accept_the_current_and_free_pins_for_existing_assignments() {
        let plan: ElectricalPlan = serde_json::from_value(serde_json::json!({
            "mode": "matrix", "boardId": "left", "revision": 3, "assignments": [],
            "rowPins": ["P0", "P1"], "columnPins": ["P2"], "freePins": ["P3"],
            "diagnostics": [], "fingerprint": "", "nets": [], "diodeDirection": "",
            "jumpers": [], "moduleAliases": {}, "peripherals": [], "peripheralPins": {},
            "peripheralTerminals": {}
        }))
        .expect("matrix plan fixture");
        let document = ProjectDoc::empty("fixture", "Pin assignment");
        for pin in ["P0", "P3"] {
            assert!(pin_edit_is_available(
                &document,
                &plan,
                "left",
                "row/0",
                Some(pin)
            ));
        }
        assert!(!pin_edit_is_available(
            &document,
            &plan,
            "left",
            "row/0",
            Some("P1")
        ));
        assert!(!pin_edit_is_available(
            &document,
            &plan,
            "left",
            "row/2",
            Some("P3")
        ));
        assert!(!pin_edit_is_available(
            &document, &plan, "left", "row/0", None
        ));
    }
}
