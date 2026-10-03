//! Editor-owned new-board action through the existing Session edit and navigation path.
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    Durability, Event, Lifecycle, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    Board, EditCommand, EditOperation, EditPhase, Operation, OutlineFeature,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct BoardCreateOwner {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    generation: u64,
    workspace: &'static str,
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct BoardSetupMount {
    pub owner: Option<BoardCreateOwner>,
    pub on_add: EventHandler<BoardCreateOwner>,
}

#[derive(Clone)]
struct PendingBoard {
    owner: BoardCreateOwner,
    board_id: String,
    outcome: OutcomeSlot,
}

pub(in crate::presentation) fn use_board_setup(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    on_navigate: EventHandler<(Scope, String, Option<String>)>,
) -> BoardSetupMount {
    let mut pending = use_signal(|| None::<PendingBoard>);
    use_effect(use_reactive((&version(), &workspace(), &generation()), {
        let runtime = runtime.clone();
        move |(_, current_workspace, current_generation)| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let terminal = waiting.outcome.borrow().clone();
            let Some(terminal) = terminal else { return };
            if terminal != TerminalOutcome::Completed {
                pending.set(None);
                return;
            }
            let model = runtime.model();
            if runtime.scope().as_ref() != Some(&waiting.owner.scope)
                || current_workspace != waiting.owner.workspace
                || current_generation != waiting.owner.generation
                || matches!(
                    model.lifecycle,
                    Lifecycle::RecoveryRequired | Lifecycle::Closed
                )
                || matches!(model.durability, Durability::Failed { .. })
            {
                pending.set(None);
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if snapshot.session_epoch != waiting.owner.scope.session_epoch
                || snapshot.document.id != waiting.owner.scope.document_id
                || snapshot.document.revision > waiting.owner.revision.saturating_add(1)
            {
                pending.set(None);
                return;
            }
            if model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: snapshot.document.revision,
                    })
                || snapshot.token == waiting.owner.token
                || snapshot.document.revision != waiting.owner.revision.saturating_add(1)
            {
                return;
            }
            let exists = snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == waiting.board_id);
            pending.set(None);
            if exists {
                on_navigate.call((waiting.owner.scope, waiting.board_id, None));
            }
        }
    }));
    let owner =
        board_source(&runtime, workspace(), generation()).filter(|_| pending.read().is_none());
    let on_add = use_callback({
        let runtime = runtime.clone();
        move |owner: BoardCreateOwner| {
            if pending.read().is_some()
                || board_source(&runtime, workspace(), generation()).as_ref() != Some(&owner)
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let operation = runtime.operation();
            let mut suffix = operation.0;
            let (board_id, outline_id) = loop {
                let board_id = format!("board-{suffix}");
                let outline_id = format!("board-outline-{suffix}");
                if !snapshot
                    .document
                    .boards
                    .iter()
                    .any(|board| board.id == board_id)
                    && !snapshot
                        .document
                        .outline
                        .iter()
                        .any(|feature| feature.id() == outline_id)
                {
                    break (board_id, outline_id);
                }
                let Some(next) = suffix.checked_add(1) else {
                    return;
                };
                suffix = next;
            };
            let mut document = (*snapshot.document).clone();
            document.boards.push(Board {
                id: board_id.clone(),
                name: format!("Board {}", document.boards.len() + 1),
                outline_ids: vec![outline_id.clone()],
                part_ids: Vec::new(),
                net_ids: Vec::new(),
                thickness: 1.6,
                traces: Vec::new(),
                vias: Vec::new(),
            });
            document.outline.push(OutlineFeature::PartEnvelope {
                id: outline_id.clone(),
                settings: crate::outline_settings::reference_outline_settings(),
                connections: Vec::new(),
                part_ids: Vec::new(),
                margin: 4.0,
                operation: Operation::Add,
            });
            pending.set(Some(PendingBoard {
                owner: owner.clone(),
                board_id: board_id.clone(),
                outcome: runtime.observe_operation(operation),
            }));
            runtime.submit(Event::Edit {
                operation_id: operation,
                command: EditCommand {
                    base_revision: owner.revision,
                    transaction_id: format!("new-board-{}", operation.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![board_id, outline_id],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                },
            });
        }
    });
    BoardSetupMount { owner, on_add }
}

fn board_source(
    runtime: &Runtime,
    workspace: &'static str,
    generation: u64,
) -> Option<BoardCreateOwner> {
    if !matches!(workspace, "Layout" | "PCB" | "Keymap" | "Keycaps") {
        return None;
    }
    let model = runtime.model();
    let snapshot = model.accepted.as_ref()?;
    let scope = runtime.scope()?;
    if model.lifecycle != Lifecycle::Ready
        || model.durability
            != (Durability::Saved {
                revision: snapshot.document.revision,
            })
        || model.gesture.is_some()
        || model.display_preview.is_some()
        || snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return None;
    }
    Some(BoardCreateOwner {
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
        generation,
        workspace,
    })
}
