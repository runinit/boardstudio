//! The Layout board-level Inspector and its accepted-session rename action.
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::EditOperation;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::TextCommitField;
use boardstudio_web_ui_shared::ui::field::FieldError;
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use super::objects::{ScopedTreeContext, TreeContext};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextIdentity {
    scope: Option<Scope>,
    selected: Option<ScopedTreeContext>,
    workspace: Workspace,
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
pub struct BoardInspectorProjection {
    owner: BoardInspectorOwner,
    pub board_name: String,
    pub outline_status: &'static str,
    pub placed_parts: usize,
    pub editable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardRenameAction {
    owner: BoardInspectorOwner,
    name: String,
}

#[derive(Clone, PartialEq)]
pub struct BoardInspectorMount {
    pub projection: Option<BoardInspectorProjection>,
    pub on_rename: EventHandler<BoardRenameAction>,
    mounted: Signal<bool>,
    failure: Signal<Option<String>>,
}

/// Owns only the transient Inspector context generation. Board data always comes from the
/// current accepted Runtime snapshot, and edits use the existing ReplaceDocument history path.
pub fn use_board_inspector(
    runtime: Rc<Runtime>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
) -> BoardInspectorMount {
    let context_generation = use_hook(|| Rc::new(RefCell::new(ContextGeneration::default())));
    let mounted = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
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
    let _ = use_context::<Signal<u64>>()();
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
            if !mounted()
                || workspace() != Workspace::Layout
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
            if name.is_empty() {
                return;
            }
            let board_id = owner.board_id.clone();
            let name = name.to_owned();
            let resolver_name = name.clone();
            let resolver = EditResolver::new("board-name", move |accepted: &AcceptedSnapshot| {
                let mut document = accepted.document.as_ref().clone();
                let Some(board) = document
                    .boards
                    .iter_mut()
                    .find(|board| board.id == board_id)
                else {
                    return Resolution::Retire("The board no longer exists.".into());
                };
                if board.name == resolver_name {
                    return Resolution::Unchanged;
                }
                board.name = resolver_name.clone();
                Resolution::submit(
                    vec![board_id.clone()],
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                )
            });
            let result = runtime.commit("board-name", resolver);
            failure.set(result.err());
        }
    });

    BoardInspectorMount {
        projection,
        on_rename,
        mounted,
        failure,
    }
}

fn context_identity(
    runtime: &Runtime,
    selected: Option<ScopedTreeContext>,
    workspace: Workspace,
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
    if context.workspace != Workspace::Layout {
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
        board_name: board.name.clone(),
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
pub fn BoardInspector(mount: BoardInspectorMount) -> Element {
    let failure = mount.failure;
    let mut mounted = mount.mounted;
    let Some(projection) = mount.projection else {
        return rsx! {};
    };
    let on_rename = mount.on_rename;
    let did_mount = use_hook(|| Rc::new(Cell::new(false)));
    let mut mounted_for_effect = mounted;
    let did_mount_for_effect = did_mount.clone();
    use_effect(move || {
        if !did_mount_for_effect.replace(true) {
            mounted_for_effect.set(true);
        }
    });
    use_drop(move || mounted.set(false));
    let owner = projection.owner.clone();
    let accepted_name = projection.owner.accepted_name.clone();
    rsx! {
        section { class: "grid content-start gap-2.5 min-w-0 px-4 pt-3 pb-5 text-foreground", aria_label: "Board Inspector",
            p { class: "m-0 mb-[5px] text-muted-foreground text-[11px] leading-[1.6]",
                "Select a key, component, or matrix to edit it."
            }
            TextCommitField {
                label: "Board name".to_owned(),
                value: accepted_name.clone(),
                id: format!(
                    "layout-input-{}-1",
                    dioxus::core::current_scope_id().0,
                ),
                aria_label: "Board name",
                disabled_reason: (!projection.editable).then(|| "Editing is unavailable".to_owned()),
                on_commit: move |name: String| {
                    on_rename.call(BoardRenameAction {
                        owner: owner.clone(),
                        name,
                    });
                },
            }
            if let Some(message) = failure() {
                FieldError { "{message}" }
            }
            div { class: "flex justify-between py-2 border-b border-border text-muted-foreground text-[10px]",
                span { "Outline" }
                strong { class: "text-foreground font-mono text-[10px] font-normal", "{projection.outline_status}" }
            }
            div { class: "flex justify-between py-2 border-b border-border text-muted-foreground text-[10px]",
                span { "Placed parts" }
                strong { class: "text-foreground font-mono text-[10px] font-normal", "{projection.placed_parts}" }
            }
        }
    }
}
