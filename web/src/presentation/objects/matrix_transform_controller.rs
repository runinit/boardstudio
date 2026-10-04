//! Accepted-snapshot projection and exact operation lifecycle for transform fields.
use super::matrix_transform_inspector::{
    MatrixTransformFeedback, MatrixTransformInspectorOwner, MatrixTransformProjection,
    MatrixTransformRequest, MatrixTransformState,
};
use super::{ScopedTreeContext, TreeContext};
use crate::matrix_transform_lifecycle::{AcceptedIdentity, PendingSettlement, pending_settlement};
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue, build_operation,
};
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    Durability, Event, Lifecycle, OperationId, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{EditCommand, EditPhase, MatrixSplayAffect, ProjectDoc, Vec2};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct PendingTransformEdit {
    request: MatrixTransformRequest,
    operation_id: OperationId,
    outcome: OutcomeSlot,
    base_token: SnapshotToken,
    base_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextIdentity {
    scope: Scope,
    context: TreeContext,
    workspace: &'static str,
    scope_generation: u64,
}

#[derive(Default)]
struct ContextGeneration {
    initialized: bool,
    identity: Option<ContextIdentity>,
    value: u64,
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct MatrixTransformInspectorMount {
    pub projection: Option<MatrixTransformProjection>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub busy: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

impl MatrixTransformInspectorMount {
    pub(in crate::presentation) fn pick_splay_origin(&self, point: Vec2) -> bool {
        let Some(projection) = self.projection.as_ref() else {
            return false;
        };
        let MatrixTransformFields::Column { splay_origin, .. } = &projection.fields else {
            return false;
        };
        if !self.editable || self.busy || !point.x.is_finite() || !point.y.is_finite() {
            return false;
        }
        let Some(request_id) = (self.request_sequence)().checked_add(1) else {
            return false;
        };
        let mut request_sequence = self.request_sequence;
        request_sequence.set(request_id);
        self.on_edit.call(MatrixTransformRequest {
            owner: projection.owner.clone(),
            request_id,
            snapshot_token: projection.snapshot_token,
            revision: projection.revision,
            field: MatrixTransformField::SplayOriginPoint,
            baseline: MatrixTransformValue::Point(*splay_origin),
            value: MatrixTransformValue::Point(point),
            splay_affect: (self.splay_affect)(),
        });
        true
    }
}

/// Called once at the Editor lifetime. Every request and projection resolves the live accepted
/// matrix again; no document copy survives as a writable store.
pub(in crate::presentation) fn use_workspace_matrix_transform(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    splay_affect: Signal<MatrixSplayAffect>,
    owner_workspace: &'static str,
) -> MatrixTransformInspectorMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let context_generation = use_hook(|| Rc::new(RefCell::new(ContextGeneration::default())));
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let current_scope_generation = scope_generation();
    let observed_identity = selected.as_ref().map(|selected| ContextIdentity {
        scope: selected.scope.clone(),
        context: selected.context.clone(),
        workspace: current_workspace,
        scope_generation: current_scope_generation,
    });
    let generation = {
        let mut tracker = context_generation.borrow_mut();
        if !tracker.initialized || tracker.identity != observed_identity {
            tracker.value = tracker
                .value
                .checked_add(1)
                .expect("matrix transform context generation exhausted");
            tracker.identity = observed_identity.clone();
            tracker.initialized = true;
        }
        tracker.value
    };

    let request_sequence = use_signal(|| 0u64);
    let last_request_id = use_signal(|| 0u64);
    let pending = use_signal(|| None::<PendingTransformEdit>);
    let feedback = use_signal(Vec::<MatrixTransformFeedback>::new);
    let (projection, editable) = project_current(
        &runtime,
        selected.as_ref(),
        editor_instance_id,
        generation,
        current_scope_generation,
        current_workspace,
        owner_workspace,
    );
    let identity = observed_identity.clone();

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation(), &identity),
        {
            let runtime = runtime.clone();
            let mut pending = pending;
            let mut feedback = feedback;
            move |(_, _, scope_generation, _)| {
                settle_pending(&runtime, scope_generation, &mut pending, &mut feedback);
            }
        },
    ));

    let on_edit = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let mut last_request_id = last_request_id;
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: MatrixTransformRequest| {
            let current_generation = context_generation.borrow().value;
            let current_workspace = workspace();
            let current_scope_generation = scope_generation();
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != current_generation
                || request.owner.scope_generation != current_scope_generation
                || !workspace_route_matches(current_workspace, owner_workspace)
                || request.owner.workspace != owner_workspace
                || request.request_id <= last_request_id()
            {
                return;
            }
            last_request_id.set(request.request_id);
            if pending.read().is_some() {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixTransformState::Failed,
                    Some("Wait for the current transform change to finish, then retry.".into()),
                );
                return;
            }
            let Some(selected) = selected_context.read().clone() else {
                return;
            };
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((current, editable)) = project_current_for(
                &runtime,
                &model,
                scope.as_ref(),
                Some(&selected),
                ProjectionContext {
                    editor_instance_id,
                    context_generation: current_generation,
                    scope_generation: current_scope_generation,
                    workspace: current_workspace,
                },
                owner_workspace,
            ) else {
                return;
            };
            if current.owner != request.owner {
                return;
            }
            if !editable {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixTransformState::Failed,
                    Some("The matrix transform is not ready to edit. Wait for the current operation to finish, then retry.".into()),
                );
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !(AcceptedIdentity {
                token: snapshot.token,
                revision: snapshot.document.revision,
            })
            .admits(AcceptedIdentity {
                token: request.snapshot_token,
                revision: request.revision,
            }) {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixTransformState::Failed,
                    Some(
                        "The accepted document changed. Review the current field and retry.".into(),
                    ),
                );
                return;
            }
            let Some(current_baseline) = field_value(&current.fields, request.field) else {
                return;
            };
            if current_baseline != request.baseline {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixTransformState::Failed,
                    Some("The accepted value changed. Review the current field and retry.".into()),
                );
                return;
            }
            if request.value == request.baseline {
                publish_feedback(&mut feedback, &request, MatrixTransformState::Saved, None);
                return;
            }
            let Some(matrix) = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == request.owner.matrix_id)
            else {
                return;
            };
            let operation = match build_operation(
                matrix,
                &current.fields,
                request.field,
                request.value.clone(),
                request.splay_affect.clone(),
            ) {
                Ok(operation) => operation,
                Err(message) => {
                    publish_feedback(
                        &mut feedback,
                        &request,
                        MatrixTransformState::Failed,
                        Some(message),
                    );
                    return;
                }
            };
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let transaction_id = format!(
                "matrix-transform-{}-{}-{}",
                editor_instance_id, request.request_id, operation_id.0
            );
            pending.set(Some(PendingTransformEdit {
                request: request.clone(),
                operation_id,
                outcome,
                base_token: snapshot.token,
                base_revision: snapshot.document.revision,
            }));
            publish_feedback(&mut feedback, &request, MatrixTransformState::Pending, None);
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id,
                    phase: EditPhase::Commit,
                    target_ids: vec![matrix.id.clone()],
                    operation,
                },
            });
        }
    });

    MatrixTransformInspectorMount {
        projection,
        request_sequence,
        editable,
        busy: pending.read().is_some(),
        feedback: feedback.read().clone(),
        on_edit,
        splay_affect,
    }
}

fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
    owner_workspace: &'static str,
) -> (Option<MatrixTransformProjection>, bool) {
    if !workspace_route_matches(workspace, owner_workspace) {
        return (None, false);
    }
    let model = runtime.model();
    let scope = runtime.scope();
    project_current_for(
        runtime,
        &model,
        scope.as_ref(),
        selected,
        ProjectionContext {
            editor_instance_id,
            context_generation,
            scope_generation,
            workspace,
        },
        owner_workspace,
    )
    .map_or((None, false), |(projection, editable)| {
        (Some(projection), editable)
    })
}

struct ProjectionContext {
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
}

fn project_current_for(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    live_scope: Option<&Scope>,
    selected: Option<&ScopedTreeContext>,
    context: ProjectionContext,
    owner_workspace: &'static str,
) -> Option<(MatrixTransformProjection, bool)> {
    if !workspace_route_matches(context.workspace, owner_workspace) {
        return None;
    }
    let scope = live_scope?;
    let selected = selected?;
    if &selected.scope != scope || runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let matrix_id = match &selected.context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => matrix_id,
        _ => return None,
    };
    let snapshot = model.accepted.as_ref()?;
    if snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || super::resolve_selection(model, &selected.context).is_none()
    {
        return None;
    }
    let matrix = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == *matrix_id)?;
    let fields = match &selected.context {
        TreeContext::Matrix { .. } => MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: matrix.rotation.unwrap_or(0.0),
            mirror: matrix.mirror,
            mirror_y_locked: matrix_has_linked_layout(
                &snapshot.document,
                &scope.board_id,
                &matrix.id,
            ),
        },
        TreeContext::Row { row, .. } => MatrixTransformFields::Row {
            row: *row,
            offset: matrix
                .row_offsets
                .get(*row as usize)
                .copied()
                .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
        },
        TreeContext::Column { column, .. } => {
            let scene = snapshot
                .scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix.id)?;
            let basis = scene.columns.iter().find(|basis| basis.column == *column)?;
            MatrixTransformFields::Column {
                column: *column,
                offset: matrix
                    .column_offsets
                    .get(*column as usize)
                    .copied()
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                stagger: matrix
                    .column_staggers
                    .get(*column as usize)
                    .copied()
                    .unwrap_or(0.0),
                splay_angle: basis.splay_angle,
                splay_origin: basis.splay_origin,
                custom_origin: basis.custom_origin,
            }
        }
        TreeContext::Key { row, column, .. } => {
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column);
            MatrixTransformFields::Key {
                row: *row,
                column: *column,
                offset: cell
                    .and_then(|cell| cell.offset)
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                rotation: cell.and_then(|cell| cell.rotation).unwrap_or(0.0),
            }
        }
        _ => return None,
    };
    let editable = model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: snapshot.document.revision,
            })
        && model.display_preview.is_none()
        && model.gesture.is_none();
    Some((
        MatrixTransformProjection {
            owner: MatrixTransformInspectorOwner {
                editor_instance_id: context.editor_instance_id,
                workspace: context.workspace,
                context_generation: context.context_generation,
                scope_generation: context.scope_generation,
                scope: scope.clone(),
                matrix_id: matrix.id.clone(),
                context: selected.context.clone(),
            },
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            label: super::context_label(model, &selected.context)
                .unwrap_or_else(|| format!("Matrix {}", matrix.id)),
            fields,
        },
        editable,
    ))
}

fn workspace_route_matches(current: &str, owner: &str) -> bool {
    matches!(owner, "Layout" | "PCB") && current == owner
}

#[cfg(all(test, target_arch = "wasm32"))]
mod workspace_route_tests {
    use super::workspace_route_matches;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn matrix_transform_owner_cannot_cross_workspace_routes() {
        assert!(workspace_route_matches("Layout", "Layout"));
        assert!(workspace_route_matches("PCB", "PCB"));
        assert!(!workspace_route_matches("PCB", "Layout"));
        assert!(!workspace_route_matches("Layout", "PCB"));
        assert!(!workspace_route_matches("Case", "Case"));
    }
}

fn matrix_has_linked_layout(document: &ProjectDoc, board_id: &str, matrix_id: &str) -> bool {
    document
        .layouts
        .iter()
        .find(|layout| layout.board_id == board_id && layout.matrix_id == matrix_id)
        .is_some_and(|layout| {
            layout.mirror_link.is_some()
                || document.layouts.iter().any(|other| {
                    other
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                })
        })
}

fn field_value(
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
) -> Option<MatrixTransformValue> {
    use MatrixTransformField as Field;
    use MatrixTransformValue as Value;
    match (fields, field) {
        (MatrixTransformFields::Matrix { origin, .. }, Field::OriginX) => {
            Some(Value::Number(origin.x))
        }
        (MatrixTransformFields::Matrix { origin, .. }, Field::OriginY) => {
            Some(Value::Number(origin.y))
        }
        (MatrixTransformFields::Matrix { rotation, .. }, Field::MatrixRotation) => {
            Some(Value::Number(*rotation))
        }
        (MatrixTransformFields::Matrix { mirror, .. }, Field::MatrixMirror) => {
            Some(Value::Mirror(*mirror))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetReset) => {
            Some(Value::Offset(*offset))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetReset) => {
            Some(Value::Offset(*offset))
        }
        (MatrixTransformFields::Column { stagger, .. }, Field::ColumnStagger) => {
            Some(Value::Number(*stagger))
        }
        (MatrixTransformFields::Column { splay_angle, .. }, Field::ColumnSplay) => {
            Some(Value::Number(*splay_angle))
        }
        (MatrixTransformFields::Column { custom_origin, .. }, Field::SplayOriginMode) => {
            Some(Value::OriginMode(*custom_origin))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginX) => {
            Some(Value::Number(splay_origin.x))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginY) => {
            Some(Value::Number(splay_origin.y))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginPoint) => {
            Some(Value::Point(*splay_origin))
        }
        (MatrixTransformFields::Key { offset, .. }, Field::KeyOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Key { offset, .. }, Field::KeyOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Key { rotation, .. }, Field::KeyRotation) => {
            Some(Value::Number(*rotation))
        }
        (
            MatrixTransformFields::Key {
                offset, rotation, ..
            },
            Field::KeyTransformReset,
        ) => Some(Value::CellTransform {
            offset: *offset,
            rotation: *rotation,
        }),
        _ => None,
    }
}

fn settle_pending(
    runtime: &Runtime,
    scope_generation: u64,
    pending: &mut Signal<Option<PendingTransformEdit>>,
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
) {
    let Some(waiting) = pending.read().clone() else {
        return;
    };
    let model = runtime.model();
    let scope = runtime.scope();
    let target_is_current = scope.as_ref() == Some(&waiting.request.owner.scope)
        && scope_generation == waiting.request.owner.scope_generation;
    let outcome = match pending_settlement(waiting.outcome.borrow().clone(), target_is_current) {
        PendingSettlement::Wait => {
            if !target_is_current {
                feedback.set(Vec::new());
            }
            return;
        }
        PendingSettlement::Suppress => {
            pending.set(None);
            feedback.set(Vec::new());
            return;
        }
        PendingSettlement::Settle(outcome) => outcome,
    };
    match outcome {
        TerminalOutcome::Completed => {
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if matches!(model.durability, Durability::Failed { .. })
                || matches!(
                    model.lifecycle,
                    Lifecycle::RecoveryRequired | Lifecycle::Closed
                )
            {
                finish_pending(pending, feedback, &waiting, MatrixTransformState::Failed,
                    Some("The transform completed, but the accepted document did not save. Retry after recovery.".into()));
                return;
            }
            if snapshot.token == waiting.base_token
                || snapshot.document.revision <= waiting.base_revision
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: snapshot.document.revision,
                    })
            {
                return;
            }
            let value = project_current_for(
                runtime,
                &model,
                scope.as_ref(),
                Some(&ScopedTreeContext {
                    scope: waiting.request.owner.scope.clone(),
                    context: waiting.request.owner.context.clone(),
                }),
                ProjectionContext {
                    editor_instance_id: waiting.request.owner.editor_instance_id,
                    context_generation: waiting.request.owner.context_generation,
                    scope_generation: waiting.request.owner.scope_generation,
                    workspace: waiting.request.owner.workspace,
                },
                waiting.request.owner.workspace,
            )
            .and_then(|(projection, _)| field_value(&projection.fields, waiting.request.field));
            if value.as_ref() == Some(&waiting.request.value) {
                finish_pending(
                    pending,
                    feedback,
                    &waiting,
                    MatrixTransformState::Saved,
                    None,
                );
            } else {
                finish_pending(pending, feedback, &waiting, MatrixTransformState::Failed,
                    Some("The saved transform does not contain the requested value. Review the current field and retry.".into()));
            }
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => {
            finish_pending(
                pending,
                feedback,
                &waiting,
                MatrixTransformState::Failed,
                Some(message),
            );
        }
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            finish_pending(pending, feedback, &waiting, MatrixTransformState::Failed,
                Some("The transform did not complete in the active session. Review the current value and retry.".into()));
        }
    }
}

fn finish_pending(
    pending: &mut Signal<Option<PendingTransformEdit>>,
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
    waiting: &PendingTransformEdit,
    state: MatrixTransformState,
    message: Option<String>,
) {
    if pending
        .read()
        .as_ref()
        .is_some_and(|current| current.operation_id == waiting.operation_id)
    {
        pending.set(None);
        publish_feedback(feedback, &waiting.request, state, message);
    }
}

fn publish_feedback(
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
    request: &MatrixTransformRequest,
    state: MatrixTransformState,
    message: Option<String>,
) {
    let next = MatrixTransformFeedback {
        owner: request.owner.clone(),
        request_id: request.request_id,
        field: request.field,
        state,
        message,
    };
    let mut entries = feedback.read().clone();
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.owner == next.owner && entry.request_id == next.request_id)
    {
        *existing = next;
    } else {
        entries.push(next);
    }
    while entries.len() > 8 {
        let Some(index) = entries
            .iter()
            .position(|entry| entry.state != MatrixTransformState::Pending)
        else {
            break;
        };
        entries.remove(index);
    }
    feedback.set(entries);
}
