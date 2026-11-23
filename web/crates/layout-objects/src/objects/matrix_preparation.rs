//! Shared admission, definition preparation, and ticket settlement for matrix creation.
use crate::matrix_setup_operation::{MatrixSetupRequest, PreparedMatrix, prepare_matrix};
use crate::{parts, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, Durability, Lifecycle, ReadModel, Scope};
use boardstudio_web_runtime::operation_outcomes::{EditOutcome, Settlement};
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;

#[derive(Clone)]
pub(super) struct PendingSettlement<Owner> {
    pub(super) ticket: EditOutcome,
    pub(super) owner: Owner,
}

impl<Owner: Clone + 'static> PendingSettlement<Owner> {
    pub(super) fn finish(
        waiting: &mut Signal<Option<Self>>,
        owner_is_live: impl FnOnce(&Owner) -> bool,
    ) -> Option<(Owner, Settlement)> {
        let pending = waiting.read().clone()?;
        let settlement = pending.ticket.settlement(owner_is_live(&pending.owner));
        waiting.set(None);
        Some((pending.owner, settlement))
    }
}

pub(super) async fn prepare_matrix_definitions(
    matrix_id: String,
    board_id: String,
    request: MatrixSetupRequest,
    reversible: bool,
    owner_is_current: impl Fn() -> bool,
) -> Result<Option<PreparedMatrix>, String> {
    let templates = parts::load_matrix_templates(reversible).await?;
    if !owner_is_current() {
        return Ok(None);
    }
    let mut prepared = prepare_matrix(matrix_id, board_id, request, reversible, &templates)?;
    for definition in &mut prepared.definitions {
        if !owner_is_current() {
            return Ok(None);
        }
        *definition = parts::normalize_matrix_definition(definition.clone()).await?;
        if !owner_is_current() {
            return Ok(None);
        }
    }
    Ok(Some(prepared))
}

pub(super) fn matrix_source<'a>(
    runtime: &Runtime,
    model: &'a ReadModel,
    scope: Option<&Scope>,
    workspace: Workspace,
) -> Option<(&'a AcceptedSnapshot, Scope, String)> {
    if workspace != Workspace::Layout {
        return None;
    }
    let scope = scope?.clone();
    if runtime.scope().as_ref() != Some(&scope) {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let durability_ready = matches!(
        model.durability,
        Durability::Saved { revision } | Durability::Saving { revision }
            if revision == snapshot.document.revision
    );
    if snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || scope.instance_id.as_ref().is_some_and(|instance_id| {
            !snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                hardware.instances.iter().any(|instance| {
                    &instance.id == instance_id && instance.board_id == scope.board_id
                })
            })
        })
        || model.lifecycle != Lifecycle::Ready
        || !durability_ready
        || model.display_preview.is_some()
        || model.gesture.is_some()
        || !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return None;
    }
    Some((snapshot, scope.clone(), scope.board_id))
}
