//! Editor-owned new-board action through the existing Session edit and navigation path.
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{Board, EditOperation, Operation, OutlineFeature};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoardAction {
    Add,
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
    let pending = use_hook(|| PendingEditSignals::<BoardAction>::new());
    let pending_add = use_signal(|| false);
    pending.bind_one_shot(BoardAction::Add, pending_add);
    let mut pending_owner = use_signal(|| None::<(BoardCreateOwner, String)>);
    use_effect(use_reactive((&version(), &workspace(), &generation()), {
        let runtime = runtime.clone();
        let pending = pending.clone();
        move |(_, current_workspace, current_generation)| {
            let Some((owner, board_id)) = pending_owner.read().clone() else {
                return;
            };
            let owner_is_live = runtime.scope().as_ref() == Some(&owner.scope)
                && current_workspace == owner.workspace
                && current_generation == owner.generation;
            for result in pending.settle(owner_is_live, |_| String::new()) {
                match result {
                    PendingEditResult::Landed { .. } => {
                        let exists = runtime.model().accepted.as_ref().is_some_and(|snapshot| {
                            snapshot
                                .document
                                .boards
                                .iter()
                                .any(|board| board.id == board_id)
                        });
                        pending_owner.set(None);
                        if exists {
                            on_navigate.call((owner.scope.clone(), board_id.clone(), None));
                        }
                    }
                    PendingEditResult::Failed { .. } | PendingEditResult::Retired { .. } => {
                        pending_owner.set(None);
                    }
                }
            }
        }
    }));
    let owner = board_source(&runtime, workspace(), generation()).filter(|_| !pending_add());
    let on_add = use_callback({
        let runtime = runtime.clone();
        move |owner: BoardCreateOwner| {
            if pending_add()
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
            pending_owner.set(Some((owner.clone(), board_id.clone())));
            pending.begin_one_shot(
                &runtime,
                BoardAction::Add,
                "layout-add-board",
                Some("board".into()),
                add_board_resolver(board_id.clone(), outline_id),
            );
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
