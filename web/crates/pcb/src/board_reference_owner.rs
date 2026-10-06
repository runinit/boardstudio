//! Owner checks and document edits for routed-board references, shared by the Layout and
//! Case reference panels.
use super::pcb_board_reference;
use super::{
    LayoutOwnerIdentity, SelectionAdapter, active_board_scope_matches, current_layout_owner,
};
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Event, Lifecycle};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use dioxus::prelude::*;
use std::rc::Rc;

pub fn board_reference_owner_is_current(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if !matches!(owner.workspace, "Layout" | "Case")
        || current_layout_owner(runtime, workspace, adapter) != *owner
    {
        return false;
    }
    let model = runtime.model();
    let Some(scope) = owner.scope.as_ref() else {
        return false;
    };
    let (Some(token), Some(revision)) = (owner.token, owner.revision) else {
        return false;
    };
    if !active_board_scope_matches(&model, scope)
        || model.lifecycle != Lifecycle::Ready
        || model.durability != (Durability::Saved { revision })
        || model.display_preview.is_some()
        || model.gesture.is_some()
    {
        return false;
    }
    model
        .accepted
        .as_ref()
        .is_some_and(|accepted| accepted.token == token && accepted.document.revision == revision)
}

pub fn board_reference_target_is_current(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    reference_id: &str,
    source_asset_id: &str,
) -> bool {
    if !board_reference_owner_lineage_is_current(runtime, workspace, adapter, owner) {
        return false;
    }
    let model = runtime.model();
    let Some(scope) = owner.scope.as_ref() else {
        return false;
    };
    active_board_scope_matches(&model, scope)
        && model.accepted.as_ref().is_some_and(|accepted| {
            accepted.document.board_references.iter().any(|reference| {
                reference.id == reference_id
                    && reference.board_id == scope.board_id
                    && reference.asset_id == source_asset_id
            })
        })
}

pub fn board_reference_owner_lineage_is_current(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if !matches!(owner.workspace, "Layout" | "Case")
        || workspace() != owner.workspace
        || (adapter.generation)() != owner.generation
        || runtime.scope() != owner.scope
    {
        return false;
    }
    let model = runtime.model();
    owner
        .scope
        .as_ref()
        .is_some_and(|scope| active_board_scope_matches(&model, scope))
}

pub fn submit_board_reference_document(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    proposed: boardstudio_core::model::ProjectDoc,
    transaction_label: &str,
) -> Result<Option<crate::operation_outcomes::OutcomeSlot>, String> {
    if !board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return Err("The active project or board changed. Retry with the current board.".into());
    }
    let model = runtime.model();
    let Some(scope) = owner.scope.as_ref() else {
        return Err("The active board is unavailable.".into());
    };
    let (Some(token), Some(revision)) = (owner.token, owner.revision) else {
        return Err("The accepted board identity is unavailable.".into());
    };
    let Some(accepted) = model
        .accepted
        .as_ref()
        .filter(|accepted| accepted.token == token && accepted.document.revision == revision)
    else {
        return Err("The accepted board changed. Reopen the reference panel and retry.".into());
    };
    if proposed.id != accepted.document.id
        || !proposed
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return Err("The routed-board edit no longer matches the accepted project.".into());
    }
    if proposed == *accepted.document {
        return Ok(None);
    }
    let operation_id = runtime.operation();
    let outcome = runtime.observe_operation(operation_id);
    runtime.submit(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: revision,
            transaction_id: format!("{transaction_label}-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![scope.board_id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(proposed),
            },
        },
    });
    Ok(Some(outcome))
}

pub fn dispatch_board_reference_action(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    reference_id: &str,
    action: pcb_board_reference::Action,
) {
    if !board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return;
    }
    let model = runtime.model();
    let Some(scope) = owner.scope.as_ref() else {
        return;
    };
    let (Some(token), Some(revision)) = (owner.token, owner.revision) else {
        return;
    };
    let Some(accepted) = model
        .accepted
        .as_ref()
        .filter(|accepted| accepted.token == token && accepted.document.revision == revision)
    else {
        return;
    };
    let mut proposed = accepted.document.as_ref().clone();
    if matches!(&action, pcb_board_reference::Action::Remove) {
        let count = proposed.board_references.len();
        proposed.board_references.retain(|reference| {
            reference.id != reference_id || reference.board_id != scope.board_id
        });
        if proposed.board_references.len() == count {
            return;
        }
    } else {
        let Some(reference) = proposed
            .board_references
            .iter_mut()
            .find(|reference| reference.id == reference_id && reference.board_id == scope.board_id)
        else {
            return;
        };
        match action {
            pcb_board_reference::Action::SetEnabled(enabled) => reference.enabled = enabled,
            pcb_board_reference::Action::SetPositionX(value) if value.is_finite() => {
                reference.pose.at.x = value;
            }
            pcb_board_reference::Action::SetPositionY(value) if value.is_finite() => {
                reference.pose.at.y = value;
            }
            pcb_board_reference::Action::SetRotation(value) if value.is_finite() => {
                reference.pose.rotation = value;
            }
            pcb_board_reference::Action::SetElevation(value) if value.is_finite() => {
                reference.elevation = value;
            }
            pcb_board_reference::Action::SetModelAsset { path, asset_id } => {
                if asset_id.as_ref().is_some_and(|asset_id| {
                    !proposed.assets.iter().any(|asset| {
                        asset.id == *asset_id
                            && [".step", ".stp", ".stl", ".wrl"].iter().any(|extension| {
                                asset.name.to_ascii_lowercase().ends_with(extension)
                            })
                    })
                }) {
                    return;
                }
                if let Some(asset_id) = asset_id {
                    reference.model_assets.insert(path, asset_id);
                } else {
                    reference.model_assets.remove(&path);
                }
            }
            pcb_board_reference::Action::Remove
            | pcb_board_reference::Action::SetPositionX(_)
            | pcb_board_reference::Action::SetPositionY(_)
            | pcb_board_reference::Action::SetRotation(_)
            | pcb_board_reference::Action::SetElevation(_) => return,
        }
    }
    if proposed == *accepted.document {
        return;
    }
    let _ = submit_board_reference_document(
        runtime,
        workspace,
        adapter,
        owner,
        proposed,
        &format!("board-reference-{reference_id}"),
    );
}
