//! Owner checks and document edits for routed-board references, shared by the Layout and
//! Case reference panels.
use super::pcb_board_reference;
use super::{
    LayoutOwnerIdentity, SelectionAdapter, active_board_scope_matches, current_layout_owner,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::EditOperation;
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;
use std::rc::Rc;

pub fn board_reference_owner_is_current(
    runtime: &Rc<Runtime>,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if !matches!(owner.workspace, Workspace::Layout | Workspace::Case)
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
    if !active_board_scope_matches(&model, scope) {
        return false;
    }
    crate::pcb_wiring::ready_snapshot(runtime)
        .is_some_and(|accepted| accepted.token == token && accepted.document.revision == revision)
}

pub fn board_reference_owner_lineage_is_current(
    runtime: &Rc<Runtime>,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if !matches!(owner.workspace, Workspace::Layout | Workspace::Case)
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

pub(crate) fn submit_board_reference_document(
    runtime: &Rc<Runtime>,
    live: bool,
    owner: &LayoutOwnerIdentity,
    proposed: boardstudio_model::model::ProjectDoc,
    transaction_label: &str,
) -> Result<(), String> {
    if !live {
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
        return Err("The upload produced no project change.".to_owned());
    }
    // Bytes were stored before this intent. Failed or retired edits leave them unreferenced;
    // asset-store cleanup is deliberately outside this migration.
    let resolver = EditResolver::new(
        "board-reference-upload",
        move |accepted: &AcceptedSnapshot| {
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
            Resolution::submit(
                vec![scope.board_id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        },
    );
    runtime.commit(transaction_label, resolver).map(|_| ())
}

/// The routed-board removal proposal: drop the reference from its board, retiring when
/// it is already gone.
fn board_reference_removal_resolver(scope_board_id: String, reference_id: String) -> EditResolver {
    EditResolver::new("board-reference", move |accepted: &AcceptedSnapshot| {
        let mut proposed = accepted.document.as_ref().clone();
        let count = proposed.board_references.len();
        proposed.board_references.retain(|reference| {
            reference.id != reference_id || reference.board_id != scope_board_id
        });
        if proposed.board_references.len() == count {
            return Resolution::Retire(
                "The reference or model asset is no longer available.".into(),
            );
        }
        if proposed == *accepted.document {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![scope_board_id.clone()],
            EditOperation::ReplaceDocument {
                document: Box::new(proposed),
            },
        )
    })
}

pub(crate) fn dispatch_board_reference_action(
    runtime: &Rc<Runtime>,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    reference_id: &str,
    action: pcb_board_reference::Action,
) -> Option<Result<AcceptedSnapshot, String>> {
    if !board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return None;
    }
    let scope = owner.scope.clone()?;
    let reference_id = reference_id.to_owned();
    let resolver = if action == pcb_board_reference::Action::Remove {
        board_reference_removal_resolver(scope.board_id, reference_id)
    } else {
        EditResolver::new("board-reference", move |accepted: &AcceptedSnapshot| {
            let mut proposed = accepted.document.as_ref().clone();
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
                pcb_board_reference::Action::Remove => unreachable!("separate removal resolver"),
                pcb_board_reference::Action::SetPositionX(_)
                | pcb_board_reference::Action::SetPositionY(_)
                | pcb_board_reference::Action::SetRotation(_)
                | pcb_board_reference::Action::SetElevation(_) => {
                    return Resolution::Retire("Enter a finite reference position.".into());
                }
            }
            if proposed == *accepted.document {
                return Resolution::Unchanged;
            }
            Resolution::submit(
                vec![scope.board_id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(proposed),
                },
            )
        })
    };
    Some(runtime.commit("board-reference", resolver))
}
