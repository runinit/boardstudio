//! Accepted board-level mode edits for PCB wiring.
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core_mechanical::electrical::ElectricalMode;
use boardstudio_model::model::EditOperation;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeIdentity {
    /// Selection-independent accepted board-plan identity.
    pub plan: WiringPlanIdentity,
    /// Selection and UI-scope identity of the rendered control.
    pub ui_scope: Scope,
    pub selected_part_id: Option<String>,
    pub scope_generation: u64,
}

impl BoardWiringModeIdentity {
    pub fn feedback_target(&self) -> BoardWiringModeFeedbackTarget {
        BoardWiringModeFeedbackTarget {
            ui_scope: self.ui_scope.clone(),
            selected_part_id: self.selected_part_id.clone(),
            scope_generation: self.scope_generation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeFeedbackTarget {
    pub ui_scope: Scope,
    pub selected_part_id: Option<String>,
    pub scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeEditRequest {
    pub identity: BoardWiringModeIdentity,
    pub mode: ElectricalMode,
}

/// The label used in the wiring panel header.
pub(crate) fn mode_text(mode: ElectricalMode) -> String {
    match mode {
        ElectricalMode::Matrix => "matrix".to_owned(),
        ElectricalMode::Direct => "direct".to_owned(),
    }
}

/// The board's accepted wiring mode; a board without configuration is a matrix.
pub(crate) fn board_mode(
    document: &boardstudio_model::model::ProjectDoc,
    board_id: &str,
) -> ElectricalMode {
    document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == board_id)
        })
        .map_or(ElectricalMode::Matrix, |configuration| configuration.mode)
}

/// The board panel's latest action failure, attributed to the target it belongs to so a
/// navigated-away board never shows another board's message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoardWiringFailure {
    pub(crate) target: BoardWiringModeFeedbackTarget,
    pub(crate) message: String,
}

#[derive(Clone, PartialEq)]
pub struct BoardWiringModeActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    pub(crate) mode: ElectricalMode,
    pub(crate) failure: Option<String>,
    pub on_change: EventHandler<BoardWiringModeEditRequest>,
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
    let _ = version();
    let failure = use_signal(|| None::<BoardWiringFailure>);
    let mode = source.as_ref().map_or(ElectricalMode::Matrix, |source| {
        board_mode(&source.document, &source.ui_scope.board_id)
    });
    let on_change = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut failure = failure;
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
            let target = request.identity.feedback_target();
            let result = runtime.commit("pcb-wiring-mode", mode_resolver(request));
            failure.set(
                result
                    .err()
                    .map(|message| BoardWiringFailure { target, message }),
            );
        }
    });
    let identity = source.as_ref().map(mode_identity);
    let editable = identity.as_ref().is_some_and(|identity| current_edit_snapshot(&runtime, identity, workspace(), scope_generation(), instance_is_current()).is_some()
        && matches!(&*resolution.read(), PcbWiringResolution::Current {identity: current, ..} if current == &identity.plan));
    let target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let failure = failure()
        .filter(|failure| Some(&failure.target) == target.as_ref())
        .map(|failure| failure.message);
    BoardWiringModeActions {
        identity,
        editable,
        mode,
        failure,
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

pub(super) fn pcb_source(
    runtime: &Runtime,
    generation: u64,
) -> Option<(AcceptedSnapshot, PcbWiringSource)> {
    let model = runtime.model();
    let snapshot = model.accepted?;
    let scope = runtime.scope()?;
    if model.lifecycle != Lifecycle::Ready
        || model.gesture.is_some()
        || model.display_preview.is_some()
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
    {
        return None;
    }
    let board_scope = Scope {
        instance_id: None,
        ..scope.clone()
    };
    let source = PcbWiringSource::new(
        &snapshot,
        &board_scope,
        &scope,
        model.selected_part_ids.first().map(String::as_str),
        generation,
    )?;
    Some((snapshot, source))
}

pub fn current_snapshot(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: &str,
    scope_generation: u64,
    instance_is_current: bool,
) -> Option<AcceptedSnapshot> {
    let snapshot = current_edit_snapshot(
        runtime,
        identity,
        workspace,
        scope_generation,
        instance_is_current,
    )?;
    (runtime.model().durability
        == Durability::Saved {
            revision: snapshot.document.revision,
        })
    .then_some(snapshot)
}

pub(super) fn current_edit_snapshot(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: &str,
    scope_generation: u64,
    instance_is_current: bool,
) -> Option<AcceptedSnapshot> {
    if workspace != "PCB" || !instance_is_current {
        return None;
    }
    let (snapshot, source) = pcb_source(runtime, scope_generation)?;
    (*identity == mode_identity(&source)).then_some(snapshot)
}

#[cfg(test)]
mod board_mode_tests {
    use super::*;

    #[test]
    fn a_board_without_configuration_is_a_matrix_and_mode_text_round_trips() {
        let document = boardstudio_model::model::ProjectDoc::empty("project", "Mode text");
        assert_eq!(board_mode(&document, "left"), ElectricalMode::Matrix);
        assert_eq!(mode_text(ElectricalMode::Matrix), "matrix");
        assert_eq!(mode_text(ElectricalMode::Direct), "direct");
    }
}
