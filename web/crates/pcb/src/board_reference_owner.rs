//! Owner checks and document edits for routed-board references, shared by the Layout and
//! Case reference panels.
use super::pcb_board_reference;
use super::{
    LayoutOwnerIdentity, SelectionAdapter, active_board_scope_matches, current_layout_owner,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use boardstudio_web_runtime::edit_ticket::EditTicket;
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
        || !matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
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
) -> Result<Option<EditTicket>, String> {
    if !board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return Err("The active project or board changed. Retry with the current board.".into());
    }
    let scope = owner
        .scope
        .clone()
        .ok_or("The active board is unavailable.")?;
    let accepted = runtime.model().accepted.ok_or("No project is open.")?;
    let assets = proposed
        .assets
        .iter()
        .filter(|asset| {
            !accepted
                .document
                .assets
                .iter()
                .any(|current| current.id == asset.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let changes = proposed
        .board_references
        .iter()
        .filter(|reference| reference.board_id == scope.board_id)
        .filter_map(|reference| {
            let previous = accepted
                .document
                .board_references
                .iter()
                .find(|current| current.id == reference.id);
            (previous != Some(reference)).then(|| (previous.cloned(), reference.clone()))
        })
        .collect::<Vec<_>>();
    if changes.is_empty() && assets.is_empty() {
        return Ok(None);
    }
    // Bytes were stored before this intent. Failed or retired edits leave them unreferenced;
    // asset-store cleanup is deliberately outside this migration.
    let resolver = EditResolver::new(
        "board-reference-upload",
        move |accepted: &AcceptedSnapshot| {
            if accepted.session_epoch != scope.session_epoch
                || accepted.document.id != scope.document_id
            {
                return Resolution::Retire(
                    boardstudio_application::DOCUMENT_SESSION_CHANGED.into(),
                );
            }
            if !accepted
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
            {
                return Resolution::Retire("The PCB design was deleted.".into());
            }
            let mut document = (*accepted.document).clone();
            for asset in &assets {
                if document.assets.iter().any(|current| current.id == asset.id) {
                    return Resolution::Retire(
                        "The uploaded asset identity is already in use.".into(),
                    );
                }
                document.assets.push(asset.clone());
            }
            for (previous, proposed) in &changes {
                match previous {
                    None => {
                        if document
                            .board_references
                            .iter()
                            .any(|reference| reference.id == proposed.id)
                        {
                            return Resolution::Retire(
                                "The reference identity is already in use.".into(),
                            );
                        }
                        document.board_references.push(proposed.clone());
                    }
                    Some(previous) => {
                        let Some(current) =
                            document.board_references.iter_mut().find(|reference| {
                                reference.id == proposed.id
                                    && reference.board_id == scope.board_id
                                    && reference.asset_id == previous.asset_id
                            })
                        else {
                            return Resolution::Retire(
                                "The routed-board reference was deleted or replaced.".into(),
                            );
                        };
                        if previous.asset_id != proposed.asset_id {
                            current.asset_id = proposed.asset_id.clone();
                        }
                        if previous.enabled != proposed.enabled {
                            current.enabled = proposed.enabled;
                        }
                        for (path, asset_id) in &proposed.model_assets {
                            if previous.model_assets.get(path) != Some(asset_id) {
                                current.model_assets.insert(path.clone(), asset_id.clone());
                            }
                        }
                    }
                }
            }
            Resolution::Submit(EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: String::new(),
                phase: EditPhase::Commit,
                target_ids: vec![scope.board_id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            })
        },
    );
    Ok(Some(EditTicket::begin(
        runtime,
        transaction_label,
        Some("board reference".into()),
        resolver,
    )))
}

pub fn dispatch_board_reference_action(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    reference_id: &str,
    action: pcb_board_reference::Action,
) -> Option<EditTicket> {
    if !board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return None;
    }
    let scope = owner.scope.clone()?;
    let reference_id = reference_id.to_owned();
    let resolver = EditResolver::new("board-reference", move |accepted: &AcceptedSnapshot| {
        if accepted.session_epoch != scope.session_epoch
            || accepted.document.id != scope.document_id
        {
            return Resolution::Retire(boardstudio_application::DOCUMENT_SESSION_CHANGED.into());
        }
        let mut proposed = accepted.document.as_ref().clone();
        if matches!(&action, pcb_board_reference::Action::Remove) {
            let count = proposed.board_references.len();
            proposed.board_references.retain(|reference| {
                reference.id != reference_id || reference.board_id != scope.board_id
            });
            if proposed.board_references.len() == count {
                return Resolution::Retire(
                    "The reference or model asset is no longer available.".into(),
                );
            }
        } else {
            let Some(reference) = proposed.board_references.iter_mut().find(|reference| {
                reference.id == reference_id && reference.board_id == scope.board_id
            }) else {
                return Resolution::Retire(
                    "The reference or model asset is no longer available.".into(),
                );
            };
            match action.clone() {
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
                        return Resolution::Retire(
                            "The reference or model asset is no longer available.".into(),
                        );
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
                | pcb_board_reference::Action::SetElevation(_) => {
                    return Resolution::Retire("Enter a finite reference position.".into());
                }
            }
        }
        if proposed == *accepted.document {
            return Resolution::Unchanged;
        }
        Resolution::Submit(EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec![scope.board_id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(proposed),
            },
        })
    });
    Some(EditTicket::begin(
        runtime,
        "board-reference",
        Some("board reference".into()),
        resolver,
    ))
}
