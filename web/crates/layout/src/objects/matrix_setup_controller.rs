//! Editor-lifetime owner for Matrix Setup preparation, submission, and exact acknowledgement.
use super::matrix_setup::{
    MatrixSetupCreateRequest, MatrixSetupMount, MatrixSetupOwner, MatrixSetupProjection,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{
    matrix_setup_operation::{next_matrix_id, prepare_matrix},
    runtime::Runtime,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope, SelectionMode,
};
use boardstudio_core::model::{EditOperation, Matrix, PartDefinition};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct MatrixSetupSubmission {
    owner: MatrixSetupOwner,
    ticket: EditTicket,
    matrix_id: String,
}

/// Resolve creating a prepared matrix against the accepted document at execution time: the
/// board must still exist and neither the matrix nor its definitions may already be taken.
fn create_matrix_resolver(
    board_id: String,
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
) -> EditResolver {
    EditResolver::new("layout-matrix-setup", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        if !document.boards.iter().any(|board| board.id == board_id) {
            return Resolution::Retire("The board no longer exists.".into());
        }
        if document
            .matrices
            .iter()
            .any(|existing| existing.id == matrix.id)
        {
            return Resolution::Retire("The new matrix identity is already in use.".into());
        }
        if definitions.iter().any(|definition| {
            document
                .definitions
                .iter()
                .any(|existing| existing.id == definition.id)
        }) {
            return Resolution::Retire(
                "The prepared matrix definitions conflict with the accepted document.".into(),
            );
        }
        Resolution::submit(
            vec![matrix.id.clone()],
            EditOperation::SetMatrix {
                matrix: matrix.clone(),
                definitions: Some(definitions.clone()),
            },
        )
    })
}

/// Called once for the Editor lifetime. The form may be hidden while its admitted SetMatrix
/// completes; that operation always settles against its captured scope and never retargets.
pub fn use_matrix_setup(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    anchor_scope: Signal<Option<Scope>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> MatrixSetupMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let open_id = use_signal(|| 0u64);
    let open = use_signal(|| None::<MatrixSetupOwner>);
    let preparing = use_signal(|| None::<MatrixSetupOwner>);
    let error = use_signal(|| None::<String>);
    let status = use_signal(|| None::<String>);
    let pending = use_signal(|| None::<MatrixSetupSubmission>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            let mut pending = pending;
            let mut selected_context = selected_context;
            let mut anchor_scope = anchor_scope;
            move |(_, workspace, scope_generation)| {
                settle_pending(
                    &runtime,
                    workspace,
                    scope_generation,
                    &mut SetupSettlement {
                        pending: &mut pending,
                        open: &mut open,
                        error: &mut error,
                        status: &mut status,
                        selected_context: &mut selected_context,
                        anchor_scope: &mut anchor_scope,
                    },
                );
                if pending.read().is_none()
                    && open.read().as_ref().is_some_and(|owner| {
                        workspace != "Layout"
                            || owner.scope_generation != scope_generation
                            || runtime.scope().as_ref() != Some(&owner.scope)
                    })
                {
                    open.set(None);
                    preparing.set(None);
                    error.set(None);
                    status.set(None);
                }
            }
        },
    ));

    let on_open = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut open_id = open_id;
        let mut error = error;
        let mut status = status;
        move |_| {
            if pending.read().is_some() || preparing.read().is_some() {
                return;
            }
            let current_workspace = workspace();
            let current_generation = scope_generation();
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((snapshot, scope, board_id)) =
                setup_source(&runtime, &model, scope.as_ref(), current_workspace)
            else {
                error.set(Some(
                    "Open a saved, editable board in Layout before creating a matrix.".into(),
                ));
                return;
            };
            let id = open_id()
                .checked_add(1)
                .expect("matrix setup identity exhausted");
            open_id.set(id);
            error.set(None);
            status.set(None);
            open.set(Some(MatrixSetupOwner {
                editor_instance_id,
                open_id: id,
                scope_generation: current_generation,
                scope,
                board_id,
                snapshot_token: snapshot.token,
                revision: snapshot.document.revision,
            }));
        }
    });

    let on_cancel = use_callback({
        let mut open = open;
        let mut error = error;
        let mut status = status;
        let mut preparing = preparing;
        move |owner: MatrixSetupOwner| {
            if open.read().as_ref() == Some(&owner) && pending.read().is_none() {
                open.set(None);
                preparing.set(None);
                error.set(None);
                status.set(None);
            }
        }
    });

    let on_create = use_callback({
        let runtime = runtime.clone();
        let alive = alive.clone();
        let mut preparing = preparing;
        let mut error = error;
        let mut status = status;
        move |request: MatrixSetupCreateRequest| {
            if open.read().as_ref() != Some(&request.owner)
                || pending.read().is_some()
                || preparing.read().is_some()
                || request.owner.editor_instance_id != editor_instance_id
                || workspace() != "Layout"
                || scope_generation() != request.owner.scope_generation
            {
                return;
            }
            if request.rows == 0
                || request.columns == 0
                || request
                    .rows
                    .checked_mul(request.columns)
                    .is_none_or(|cells| cells > 4096)
            {
                error.set(Some(
                    "Rows and columns must be positive, with at most 4096 cells.".into(),
                ));
                return;
            }
            let model = runtime.model();
            let live_scope = runtime.scope();
            let Some((snapshot, scope, board_id)) =
                setup_source(&runtime, &model, live_scope.as_ref(), workspace())
            else {
                error.set(Some(
                    "The selected board is no longer ready for matrix creation.".into(),
                ));
                return;
            };
            if scope != request.owner.scope
                || board_id != request.owner.board_id
                || snapshot.token != request.owner.snapshot_token
                || snapshot.document.revision != request.owner.revision
            {
                error.set(Some("The accepted board changed. Close Matrix Setup and reopen it before creating a matrix.".into()));
                return;
            }
            let owner = request.owner.clone();
            let accepted = snapshot.clone();
            let reversible = accepted
                .document
                .parameters
                .get("reversibleLayout")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let matrix_id = next_matrix_id(
                &accepted.document.matrices,
                &accepted.document.definitions,
                &accepted.document.parts,
                request.preset.as_str(),
                reversible,
            );
            preparing.set(Some(owner.clone()));
            error.set(None);
            status.set(Some("Preparing matrix definitions…".into()));
            let runtime = runtime.clone();
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            let mut pending = pending;
            let mut open = open;
            let alive = alive.clone();
            spawn_local(async move {
                if !alive.get() {
                    return;
                }
                let result = async {
                    let templates =
                        crate::presentation::parts::load_matrix_templates(reversible).await?;
                    if !alive.get() {
                        return Err("Matrix Setup owner closed.".into());
                    }
                    let mut prepared = prepare_matrix(
                        matrix_id.clone(),
                        board_id.clone(),
                        crate::matrix_setup_operation::MatrixSetupRequest {
                            rows: request.rows,
                            columns: request.columns,
                            preset: request.preset,
                        },
                        reversible,
                        &templates,
                    )?;
                    for definition in &mut prepared.definitions {
                        *definition = crate::presentation::parts::normalize_matrix_definition(
                            definition.clone(),
                        )
                        .await?;
                        if !alive.get() {
                            return Err("Matrix Setup owner closed.".into());
                        }
                    }
                    Ok::<_, String>((prepared.matrix, prepared.definitions))
                }
                .await;
                if !alive.get() {
                    return;
                }
                if !preparing
                    .read()
                    .as_ref()
                    .is_some_and(|active| active == &owner)
                {
                    return;
                }
                let model = runtime.model();
                let current_scope = runtime.scope();
                let current_workspace = workspace();
                let current_generation = scope_generation();
                let Some((current, scope, current_board)) =
                    setup_source(&runtime, &model, current_scope.as_ref(), current_workspace)
                else {
                    preparing.set(None);
                    open.set(None);
                    error.set(None);
                    status.set(None);
                    return;
                };
                if current_workspace != "Layout"
                    || current_generation != owner.scope_generation
                    || scope != owner.scope
                    || current_board != owner.board_id
                    || current.token != accepted.token
                    || current.document.revision != accepted.document.revision
                    || open.read().as_ref() != Some(&owner)
                {
                    preparing.set(None);
                    open.set(None);
                    error.set(None);
                    status.set(None);
                    return;
                }
                let (matrix, definitions) = match result {
                    Ok(prepared) => prepared,
                    Err(message) => {
                        preparing.set(None);
                        error.set(Some(message));
                        status.set(None);
                        return;
                    }
                };
                if matrix.board_id.as_deref() != Some(current_board.as_str())
                    || definitions.iter().any(|definition| {
                        definition.generator.is_none()
                            || definition.id.is_empty()
                            || current
                                .document
                                .definitions
                                .iter()
                                .any(|existing| existing.id == definition.id)
                    })
                {
                    preparing.set(None);
                    error.set(Some("The prepared matrix definition set is incomplete or conflicts with the accepted document.".into()));
                    status.set(None);
                    return;
                }
                let matrix_id = matrix.id.clone();
                let ticket = EditTicket::begin(
                    &runtime,
                    "layout-matrix-setup",
                    Some("matrix".into()),
                    create_matrix_resolver(current_board.clone(), matrix, definitions),
                );
                pending.set(Some(MatrixSetupSubmission {
                    owner: owner.clone(),
                    ticket,
                    matrix_id,
                }));
                preparing.set(None);
                status.set(Some("Creating matrix…".into()));
            });
        }
    });

    let projection = open.read().clone().map(|owner| {
        let idle = pending.read().is_none() && preparing.read().is_none();
        MatrixSetupProjection {
            editable: idle && setup_owner_live(&runtime, &owner, workspace(), scope_generation()),
            can_cancel: pending.read().is_none(),
            owner,
            status: status.read().clone(),
            error: error.read().clone(),
        }
    });
    let current_model = runtime.model();
    let can_open = pending.read().is_none()
        && preparing.read().is_none()
        && open.read().is_none()
        && setup_source(
            &runtime,
            &current_model,
            runtime.scope().as_ref(),
            workspace(),
        )
        .is_some();
    MatrixSetupMount {
        projection,
        can_open,
        on_open,
        on_cancel,
        on_create,
    }
}

fn setup_owner_live(
    runtime: &Runtime,
    owner: &MatrixSetupOwner,
    workspace: &'static str,
    scope_generation: u64,
) -> bool {
    if workspace != "Layout" || owner.scope_generation != scope_generation {
        return false;
    }
    let model = runtime.model();
    setup_source(runtime, &model, runtime.scope().as_ref(), workspace).is_some_and(
        |(snapshot, scope, board_id)| {
            scope == owner.scope
                && board_id == owner.board_id
                && snapshot.token == owner.snapshot_token
                && snapshot.document.revision == owner.revision
        },
    )
}

fn setup_source<'a>(
    runtime: &Runtime,
    model: &'a boardstudio_application::ReadModel,
    scope: Option<&Scope>,
    workspace: &'static str,
) -> Option<(&'a AcceptedSnapshot, Scope, String)> {
    if workspace != "Layout" {
        return None;
    }
    let scope = scope?.clone();
    if runtime.scope().as_ref() != Some(&scope) {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
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
        || model.durability
            != (Durability::Saved {
                revision: snapshot.document.revision,
            })
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

struct SetupSettlement<'a> {
    pending: &'a mut Signal<Option<MatrixSetupSubmission>>,
    open: &'a mut Signal<Option<MatrixSetupOwner>>,
    error: &'a mut Signal<Option<String>>,
    status: &'a mut Signal<Option<String>>,
    selected_context: &'a mut Signal<Option<ScopedTreeContext>>,
    anchor_scope: &'a mut Signal<Option<Scope>>,
}

fn settle_pending(
    runtime: &Rc<Runtime>,
    workspace: &'static str,
    scope_generation: u64,
    signals: &mut SetupSettlement<'_>,
) {
    let Some(waiting) = signals.pending.read().clone() else {
        return;
    };
    let model = runtime.model();
    // A hidden or departed owner cannot reconcile selection or publish feedback.
    let owner_is_live = workspace == "Layout"
        && scope_generation == waiting.owner.scope_generation
        && runtime.scope().as_ref() == Some(&waiting.owner.scope)
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == waiting.owner.scope.session_epoch
                && snapshot.document.id == waiting.owner.scope.document_id
        });
    let settlement = waiting.ticket.settlement(owner_is_live);
    match settlement {
        Settlement::Pending => {}
        Settlement::Retired => {
            finish_pending(signals.pending, signals.status, &waiting);
            signals.open.set(None);
            signals.error.set(None);
        }
        Settlement::Landed { .. } => {
            finish_pending(signals.pending, signals.status, &waiting);
            signals.open.set(None);
            signals.error.set(None);
            if model.active_board_id == waiting.owner.board_id {
                let context = TreeContext::Matrix {
                    matrix_id: waiting.matrix_id,
                };
                let Some(ids) = super::resolve_selection(&model, &context) else {
                    signals.error.set(Some(
                        "The new matrix could not be selected from the saved board.".into(),
                    ));
                    return;
                };
                signals.selected_context.set(Some(ScopedTreeContext {
                    scope: waiting.owner.scope.clone(),
                    context,
                }));
                signals.anchor_scope.set(None);
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: ids,
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
            }
        }
        Settlement::Failed { message } => {
            finish_pending(signals.pending, signals.status, &waiting);
            signals.error.set(Some(message));
        }
    }
}

fn finish_pending(
    pending: &mut Signal<Option<MatrixSetupSubmission>>,
    status: &mut Signal<Option<String>>,
    waiting: &MatrixSetupSubmission,
) {
    if pending
        .read()
        .as_ref()
        .is_some_and(|current| current.ticket.operation() == waiting.ticket.operation())
    {
        pending.set(None);
        status.set(None);
    }
}
