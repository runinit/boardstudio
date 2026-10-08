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
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeEditRequest {
    pub identity: BoardWiringModeIdentity,
    pub mode: ElectricalMode,
}

/// The accepted or pending mode written as the mode control's `value` text.
pub(crate) fn mode_text(mode: ElectricalMode) -> String {
    match mode {
        ElectricalMode::Matrix => "matrix".to_owned(),
        ElectricalMode::Direct => "direct".to_owned(),
    }
}

/// The board's accepted wiring mode; a board without configuration is a matrix.
pub(crate) fn board_mode(
    document: &boardstudio_core::model::ProjectDoc,
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
    /// The value the mode control shows: the submitted choice while its edit is pending,
    /// otherwise the accepted mode. The helper restores it on settlement and a newer
    /// choice is never overwritten by an older outcome.
    pub(crate) draft: Signal<String>,
    /// The current target's latest observation is still pending.
    pub(crate) pending: bool,
    /// The current target's inline failure; landing and retirement are silent.
    pub(crate) failure: Signal<Option<String>>,
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
    // A rendered owner has one field binding. Departing drops only its observation;
    // its edit remains in the Session queue and can still land normally.
    let edits = use_hook(|| {
        Rc::new(RefCell::new(PendingEditSignals::<
            BoardWiringModeFeedbackTarget,
        >::new()))
    });
    let mut draft = use_signal(String::new);
    let mut failure = use_signal(|| None::<String>);
    let mut bound = use_signal(|| None::<BoardWiringModeFeedbackTarget>);
    let settlement_tick = use_signal(|| 0u64);
    let _ = settlement_tick();
    let identity = source.as_ref().map(mode_identity);
    let target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let accepted_snapshot = runtime.model().accepted;
    let accepted = identity.as_ref().map_or_else(
        || mode_text(ElectricalMode::Matrix),
        |identity| {
            accepted_snapshot.as_ref().map_or_else(
                || mode_text(ElectricalMode::Matrix),
                |snapshot| mode_text(board_mode(&snapshot.document, &identity.ui_scope.board_id)),
            )
        },
    );
    if bound.peek().as_ref() != target.as_ref() {
        // Feedback targets exclude accepted revision/token, so ordinary landing does
        // not replace this helper; only selection or scope ownership does.
        *edits.borrow_mut() = PendingEditSignals::new();
        if let Some(current) = target.clone() {
            edits.borrow().bind_field(current, draft, failure);
        }
        bound.set(target.clone());
        failure.set(None);
        draft.set(accepted.clone());
    }
    let pending = target
        .as_ref()
        .is_some_and(|target| edits.borrow().is_pending(target));
    if !pending && draft.peek().as_str() != accepted {
        draft.set(accepted);
    }
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let edits = edits.clone();
        let mut settlement_tick = settlement_tick;
        move |_| {
            // The helper owns panel-lifetime gating; each ticket retires itself when its
            // captured Scope moves on. The mode control therefore only answers for the
            // workspace it belongs to.
            let panel_is_live = workspace() == "PCB";
            let results = edits.borrow().settle(panel_is_live, |target| {
                let model = runtime.model();
                model.accepted.as_ref().map_or_else(
                    || mode_text(ElectricalMode::Matrix),
                    |snapshot| mode_text(board_mode(&snapshot.document, &target.ui_scope.board_id)),
                )
            });
            if !results.is_empty() {
                settlement_tick.set(settlement_tick().wrapping_add(1));
            }
        }
    }));
    let on_change = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let edits = edits.clone();
        let mut draft = draft;
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
            // Submit the exact text the user chose; the control keeps it while pending.
            let text = mode_text(request.mode);
            draft.set(text.clone());
            failure.set(None);
            edits.borrow().begin_field(
                &runtime,
                request.identity.feedback_target(),
                "pcb-wiring-mode",
                Some("wiring mode".into()),
                mode_resolver(request),
                &text,
            );
        }
    });
    let identity = source.as_ref().map(mode_identity);
    let editable = identity.as_ref().is_some_and(|identity| current_edit_snapshot(&runtime, identity, workspace(), scope_generation(), instance_is_current()).is_some()
        && matches!(&*resolution.read(), PcbWiringResolution::Current {identity: current, ..} if current == &identity.plan));
    BoardWiringModeActions {
        identity,
        editable,
        draft,
        pending,
        failure,
        on_change,
    }
}

#[cfg(test)]
mod board_mode_tests {
    use super::*;

    #[test]
    fn a_board_without_configuration_is_a_matrix_and_mode_text_round_trips() {
        let document = boardstudio_core::model::ProjectDoc::empty("project", "Mode text");
        assert_eq!(board_mode(&document, "left"), ElectricalMode::Matrix);
        assert_eq!(mode_text(ElectricalMode::Matrix), "matrix");
        assert_eq!(mode_text(ElectricalMode::Direct), "direct");
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
