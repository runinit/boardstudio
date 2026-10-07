//! Editor-owned new-board action through the existing Session edit and navigation path.
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{Board, EditOperation, Operation, OutlineFeature};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardCreateOwner {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    generation: u64,
    workspace: &'static str,
}

#[derive(Clone, PartialEq)]
pub struct BoardSetupMount {
    pub owner: Option<BoardCreateOwner>,
    pub on_add: EventHandler<BoardCreateOwner>,
}

#[derive(Clone)]
struct BoardSubmission {
    owner: BoardCreateOwner,
    board_id: String,
    ticket: EditTicket,
}

/// Resolve adding a board against the accepted document at execution time. The board and
/// outline identities were chosen when the user clicked; the replacement document is cloned
/// from the snapshot the resolver is handed.
fn add_board_resolver(board_id: String, outline_id: String) -> EditResolver {
    EditResolver::new("layout-add-board", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        if document.boards.iter().any(|board| board.id == board_id)
            || document
                .outline
                .iter()
                .any(|feature| feature.id() == outline_id)
        {
            return Resolution::Retire("The new board identity is already in use.".into());
        }
        let mut replacement = document.as_ref().clone();
        replacement.boards.push(Board {
            id: board_id.clone(),
            name: format!("Board {}", replacement.boards.len() + 1),
            outline_ids: vec![outline_id.clone()],
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        replacement.outline.push(OutlineFeature::PartEnvelope {
            id: outline_id.clone(),
            settings: crate::outline_settings::reference_outline_settings(),
            connections: Vec::new(),
            part_ids: Vec::new(),
            margin: 4.0,
            operation: Operation::Add,
        });
        Resolution::submit(
            vec![board_id.clone(), outline_id.clone()],
            EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
        )
    })
}

pub fn use_board_setup(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    on_navigate: EventHandler<(Scope, String, Option<String>)>,
) -> BoardSetupMount {
    let mut pending = use_signal(|| None::<BoardSubmission>);
    use_effect(use_reactive((&version(), &workspace(), &generation()), {
        let runtime = runtime.clone();
        move |(_, current_workspace, current_generation)| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let owner_is_live = runtime.scope().as_ref() == Some(&waiting.owner.scope)
                && current_workspace == waiting.owner.workspace
                && current_generation == waiting.owner.generation;
            match waiting.ticket.settlement(owner_is_live) {
                Settlement::Pending => {}
                Settlement::Landed { .. } => {
                    let exists = runtime.model().accepted.as_ref().is_some_and(|snapshot| {
                        snapshot
                            .document
                            .boards
                            .iter()
                            .any(|board| board.id == waiting.board_id)
                    });
                    pending.set(None);
                    if exists {
                        on_navigate.call((waiting.owner.scope, waiting.board_id, None));
                    }
                }
                Settlement::Failed { .. } | Settlement::Retired => pending.set(None),
            }
        }
    }));
    let owner = board_source(&runtime, workspace(), generation()).filter(|_| {
        !pending
            .read()
            .as_ref()
            .is_some_and(|waiting| waiting.ticket.is_pending())
    });
    let on_add = use_callback({
        let runtime = runtime.clone();
        move |owner: BoardCreateOwner| {
            if pending
                .read()
                .as_ref()
                .is_some_and(|waiting| waiting.ticket.is_pending())
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
            let ticket = EditTicket::begin(
                &runtime,
                "layout-add-board",
                Some("board".into()),
                add_board_resolver(board_id.clone(), outline_id),
            );
            pending.set(Some(BoardSubmission {
                owner: owner.clone(),
                board_id,
                ticket,
            }));
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
