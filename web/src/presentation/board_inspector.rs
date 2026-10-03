//! The Layout board-level Inspector and its accepted-session rename action.
use crate::runtime::Runtime;
use boardstudio_application::{Event, Lifecycle, Scope, SnapshotToken};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

use super::objects::{ScopedTreeContext, TreeContext};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextIdentity {
    scope: Option<Scope>,
    selected: Option<ScopedTreeContext>,
    workspace: &'static str,
    scope_generation: u64,
}

#[derive(Default)]
struct ContextGeneration {
    identity: Option<ContextIdentity>,
    value: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BoardInspectorOwner {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    context: ContextIdentity,
    generation: u64,
    board_id: String,
    accepted_name: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct BoardInspectorProjection {
    owner: BoardInspectorOwner,
    pub(super) outline_status: &'static str,
    pub(super) placed_parts: usize,
    pub(super) editable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BoardRenameAction {
    owner: BoardInspectorOwner,
    name: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct BoardInspectorMount {
    pub(super) projection: Option<BoardInspectorProjection>,
    pub(super) on_rename: EventHandler<BoardRenameAction>,
}

/// Owns only the transient Inspector context generation. Board data always comes from the
/// current accepted Runtime snapshot, and edits use the existing ReplaceDocument history path.
pub(super) fn use_board_inspector(
    runtime: Rc<Runtime>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> BoardInspectorMount {
    let context_generation = use_hook(|| Rc::new(RefCell::new(ContextGeneration::default())));
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let current_scope_generation = scope_generation();
    let context = context_identity(
        &runtime,
        selected.clone(),
        current_workspace,
        current_scope_generation,
    );
    let generation = {
        let mut tracker = context_generation.borrow_mut();
        if tracker.identity.as_ref() != Some(&context) {
            tracker.value = tracker
                .value
                .checked_add(1)
                .expect("board Inspector context generation exhausted");
            tracker.identity = Some(context.clone());
        }
        tracker.value
    };
    let projection = project_current(&runtime, selected.as_ref(), context.clone(), generation);

    let on_rename = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        move |action: BoardRenameAction| {
            let owner = &action.owner;
            let current_context = context_identity(
                &runtime,
                selected_context.read().clone(),
                workspace(),
                scope_generation(),
            );
            if workspace() != "Layout"
                || context_generation.borrow().value != owner.generation
                || current_context != owner.context
                || runtime.scope().as_ref() != Some(&owner.scope)
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if model.lifecycle != Lifecycle::Ready
                || snapshot.token != owner.token
                || snapshot.document.revision != owner.revision
                || snapshot.session_epoch != owner.scope.session_epoch
                || snapshot.document.id != owner.scope.document_id
                || model.active_board_id != owner.board_id
                || !board_context_is_current(
                    &runtime,
                    &model,
                    &owner.scope,
                    selected_context.read().as_ref(),
                )
            {
                return;
            }
            let name = action.name.trim();
            if name.is_empty() || name == owner.accepted_name {
                return;
            }
            let mut document = (*snapshot.document).clone();
            let Some(board) = document
                .boards
                .iter_mut()
                .find(|board| board.id == owner.board_id)
            else {
                return;
            };
            if board.name != owner.accepted_name {
                return;
            }
            board.name = name.to_owned();
            let operation_id = runtime.operation();
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: owner.revision,
                    transaction_id: format!("board-inspector-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![owner.board_id.clone()],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                },
            });
        }
    });

    BoardInspectorMount {
        projection,
        on_rename,
    }
}

fn context_identity(
    runtime: &Runtime,
    selected: Option<ScopedTreeContext>,
    workspace: &'static str,
    scope_generation: u64,
) -> ContextIdentity {
    ContextIdentity {
        scope: runtime.scope(),
        selected,
        workspace,
        scope_generation,
    }
}

fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    context: ContextIdentity,
    generation: u64,
) -> Option<BoardInspectorProjection> {
    if context.workspace != "Layout" {
        return None;
    }
    let model = runtime.model();
    let snapshot = model.accepted.as_ref()?;
    let scope = context.scope.clone()?;
    if !board_context_is_current(runtime, &model, &scope, selected)
        || scope.board_id != model.active_board_id
        || scope.document_id != snapshot.document.id
        || scope.session_epoch != snapshot.session_epoch
    {
        return None;
    }
    let board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)?;
    let placed_parts = board
        .part_ids
        .iter()
        .filter(|id| snapshot.document.parts.iter().any(|part| part.id == **id))
        .count();
    let outline_status = if snapshot
        .scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == board.id)
        .is_some_and(|readiness| readiness.outline)
    {
        "Resolved"
    } else {
        "Not defined"
    };
    Some(BoardInspectorProjection {
        owner: BoardInspectorOwner {
            scope,
            token: snapshot.token,
            revision: snapshot.document.revision,
            context,
            generation,
            board_id: board.id.clone(),
            accepted_name: board.name.clone(),
        },
        outline_status,
        placed_parts,
        editable: model.lifecycle == Lifecycle::Ready,
    })
}

fn board_context_is_current(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    scope: &Scope,
    selected: Option<&ScopedTreeContext>,
) -> bool {
    if runtime.scope().as_ref() != Some(scope) {
        return false;
    }
    match selected {
        None => model.selected_part_ids.is_empty(),
        Some(selected) => {
            selected.scope == *scope
                && matches!(
                    &selected.context,
                    TreeContext::Board { board_id } if board_id == &scope.board_id
                )
                && model.selected_part_ids.is_empty()
                && super::selection::context_is_current(model, scope, &selected.context)
        }
    }
}

#[component]
pub(super) fn BoardInspector(
    projection: BoardInspectorProjection,
    on_rename: EventHandler<BoardRenameAction>,
) -> Element {
    let mut draft = use_signal(|| None::<NameDraft>);
    let owner = projection.owner.clone();
    let accepted_name = projection.owner.accepted_name.clone();
    let name = draft()
        .filter(|draft| draft.owner == owner && draft.baseline == accepted_name)
        .map(|draft| draft.value)
        .unwrap_or_else(|| accepted_name.clone());
    let input_owner = owner.clone();
    let blur_owner = owner.clone();
    let key_owner = owner.clone();
    let input_name = accepted_name.clone();
    let blur_name = accepted_name.clone();
    let key_name = accepted_name.clone();
    let blur_handler = on_rename;
    let key_handler = on_rename;
    let submit_blur = move |_| {
        commit_draft(&mut draft, &blur_owner, &blur_name, blur_handler);
    };
    let submit_key = move |event: KeyboardEvent| match event.key().to_string().as_str() {
        "Enter" => {
            event.prevent_default();
            commit_draft(&mut draft, &key_owner, &key_name, key_handler);
        }
        "Escape" => {
            event.prevent_default();
            event.stop_propagation();
            draft.set(None);
        }
        _ => {}
    };
    rsx! {
        section { class: "m1-board-inspector", aria_label: "Board Inspector",
            p { class: "m1-board-inspector-guidance", "Select a key, component, or matrix to edit it." }
            label { class: "m1-board-inspector-name",
                "Board name"
                input {
                    aria_label: "Board name",
                    value: "{name}",
                    disabled: !projection.editable,
                    oninput: move |event| draft.set(Some(NameDraft {
                        owner: input_owner.clone(),
                        baseline: input_name.clone(),
                        value: event.value(),
                        submitted: false,
                    })),
                    onblur: submit_blur,
                    onkeydown: submit_key,
                }
            }
            div { class: "m1-board-inspector-measure",
                span { "Outline" }
                strong { "{projection.outline_status}" }
            }
            div { class: "m1-board-inspector-measure",
                span { "Placed parts" }
                strong { "{projection.placed_parts}" }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NameDraft {
    owner: BoardInspectorOwner,
    baseline: String,
    value: String,
    submitted: bool,
}

fn commit_draft(
    draft: &mut Signal<Option<NameDraft>>,
    owner: &BoardInspectorOwner,
    accepted_name: &str,
    on_rename: EventHandler<BoardRenameAction>,
) {
    let Some(mut value) = draft.read().clone() else {
        return;
    };
    if value.owner != *owner || value.baseline != accepted_name || value.submitted {
        return;
    }
    let name = value.value.trim();
    if name.is_empty() || name == accepted_name {
        draft.set(None);
        return;
    }
    let name = name.to_owned();
    value.submitted = true;
    draft.set(Some(value));
    on_rename.call(BoardRenameAction {
        owner: owner.clone(),
        name,
    });
}
