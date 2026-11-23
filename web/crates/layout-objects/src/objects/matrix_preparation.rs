//! Shared admission, definition preparation, and ticket settlement for matrix creation.
use crate::matrix_setup_operation::{MatrixSetupRequest, PreparedMatrix, prepare_matrix};
#[cfg(target_arch = "wasm32")]
use crate::parts;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Durability, Lifecycle, ReadModel, Scope};
use boardstudio_web_runtime::operation_outcomes::{EditOutcome, Settlement};
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
use std::cell::RefCell;

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

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
thread_local! {
    static TEMPLATE_REPLY: RefCell<Option<Result<Vec<boardstudio_model::model::PartDefinition>, String>>> =
        const { RefCell::new(None) };
}

/// Stage the next template reply for native lifecycle tests. Until a reply is staged,
/// preparation stays pending across detached polls so tests can interleave editor unmounts
/// and accepted-revision changes while normalization is in flight.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(super) fn stage_test_templates(
    reply: Result<Vec<boardstudio_model::model::PartDefinition>, String>,
) {
    TEMPLATE_REPLY.with_borrow_mut(|slot| *slot = Some(reply));
}

/// Drop any staged template reply without completing preparation.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(super) fn clear_test_templates() {
    TEMPLATE_REPLY.with_borrow_mut(|slot| *slot = None);
}

#[cfg(target_arch = "wasm32")]
async fn normalize_prepared_definition(
    definition: boardstudio_model::model::PartDefinition,
) -> Result<boardstudio_model::model::PartDefinition, String> {
    parts::normalize_matrix_definition(definition).await
}

/// Native lifecycle tests exercise controller admission and settlement, not generator
/// normalization, so prepared definitions pass through unstamped like the retired browser
/// harness's port.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
async fn normalize_prepared_definition(
    definition: boardstudio_model::model::PartDefinition,
) -> Result<boardstudio_model::model::PartDefinition, String> {
    Ok(definition)
}

pub(super) async fn prepare_matrix_definitions(
    matrix_id: String,
    board_id: String,
    request: MatrixSetupRequest,
    reversible: bool,
    owner_is_current: impl Fn() -> bool,
) -> Result<Option<PreparedMatrix>, String> {
    #[cfg(target_arch = "wasm32")]
    let templates = parts::load_matrix_templates(reversible).await?;
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(test)]
    let templates = {
        std::future::poll_fn(|_| {
            TEMPLATE_REPLY.with_borrow_mut(|reply| {
                reply
                    .take()
                    .map_or(std::task::Poll::Pending, std::task::Poll::Ready)
            })
        })
        .await?
    };
    if !owner_is_current() {
        return Ok(None);
    }
    let mut prepared = prepare_matrix(matrix_id, board_id, request, reversible, &templates)?;
    for definition in &mut prepared.definitions {
        if !owner_is_current() {
            return Ok(None);
        }
        *definition = normalize_prepared_definition(definition.clone()).await?;
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

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{
        open_board_document as open_document, test_services as services,
    };
    use super::*;
    use boardstudio_application::Event;

    #[test]
    fn configured_board_instance_can_open_matrix_setup() {
        let runtime = Runtime::with_core(services());
        open_document(&runtime, "A", 10, true);
        runtime.submit(Event::Navigate {
            operation_id: runtime.operation(),
            board_id: "board".into(),
            instance_id: Some("primary".into()),
        });
        let model = runtime.model();
        assert!(
            matrix_source(
                &runtime,
                &model,
                runtime.scope().as_ref(),
                Workspace::Layout
            )
            .is_some(),
            "a current configured physical instance must admit logical board creation"
        );
    }

    #[test]
    fn nonexistent_physical_instance_cannot_open_matrix_setup() {
        let runtime = Runtime::with_core(services());
        open_document(&runtime, "A", 10, true);
        // Session rejects navigation to an unknown instance, so a scope naming one can only
        // arise from a stale owner. Admission must still refuse it.
        let mut scope = runtime.scope().expect("open session has a scope");
        scope.instance_id = Some("missing".into());
        let model = runtime.model();
        assert!(
            matrix_source(&runtime, &model, Some(&scope), Workspace::Layout).is_none(),
            "a nonexistent physical instance must not admit logical board creation"
        );
    }
}
