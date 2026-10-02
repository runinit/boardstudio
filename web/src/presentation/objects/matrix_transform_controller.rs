//! Accepted-snapshot projection and exact operation lifecycle for transform fields.
use super::matrix_transform_inspector::{
    MatrixTransformFeedback, MatrixTransformField, MatrixTransformFields,
    MatrixTransformInspectorOwner, MatrixTransformProjection, MatrixTransformRequest,
    MatrixTransformState, MatrixTransformValue,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    Durability, Event, Lifecycle, OperationId, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Matrix, MatrixCell, MatrixSplayAffect,
    MatrixSplayChange, Mirror, ProjectDoc, Vec2,
};
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

/// Called once at the Editor lifetime. Every request and projection resolves the live accepted
/// matrix again; no document copy survives as a writable store.
pub(in crate::presentation) fn use_matrix_transform_inspector(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    splay_affect: Signal<MatrixSplayAffect>,
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
                || current_workspace != "Layout"
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
) -> (Option<MatrixTransformProjection>, bool) {
    if workspace != "Layout" {
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
) -> Option<(MatrixTransformProjection, bool)> {
    if context.workspace != "Layout" {
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

fn build_operation(
    matrix: &Matrix,
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
    value: MatrixTransformValue,
    affect: MatrixSplayAffect,
) -> Result<EditOperation, String> {
    use MatrixTransformField as Field;
    use MatrixTransformValue as Value;
    match (fields, field, value) {
        (MatrixTransformFields::Matrix { .. }, Field::OriginX, Value::Number(x)) => {
            let mut next = matrix.clone();
            next.origin.x = x;
            Ok(set_matrix(next))
        }
        (MatrixTransformFields::Matrix { .. }, Field::OriginY, Value::Number(y)) => {
            let mut next = matrix.clone();
            next.origin.y = y;
            Ok(set_matrix(next))
        }
        (MatrixTransformFields::Matrix { .. }, Field::MatrixRotation, Value::Number(rotation)) => {
            let mut next = matrix.clone();
            next.rotation = Some(rotation);
            Ok(set_matrix(next))
        }
        (
            MatrixTransformFields::Matrix {
                mirror_y_locked, ..
            },
            Field::MatrixMirror,
            Value::Mirror(mirror),
        ) => {
            if *mirror_y_locked && mirror == Some(Mirror::Y) {
                return Err("Y-axis mirroring is unavailable for a linked layout.".into());
            }
            let mut next = matrix.clone();
            next.mirror = mirror;
            Ok(set_matrix(next))
        }
        (MatrixTransformFields::Row { row, offset }, Field::RowOffsetX, Value::Number(x)) => {
            let next_offset = Vec2 { x, y: offset.y };
            Ok(set_matrix_offset(matrix, true, *row, next_offset))
        }
        (MatrixTransformFields::Row { row, offset }, Field::RowOffsetY, Value::Number(y)) => {
            let next_offset = Vec2 { x: offset.x, y };
            Ok(set_matrix_offset(matrix, true, *row, next_offset))
        }
        (MatrixTransformFields::Row { row, .. }, Field::RowOffsetReset, Value::Offset(offset)) => {
            Ok(set_matrix_offset(matrix, true, *row, offset))
        }
        (
            MatrixTransformFields::Column { column, offset, .. },
            Field::ColumnOffsetX,
            Value::Number(x),
        ) => Ok(set_matrix_offset(
            matrix,
            false,
            *column,
            Vec2 { x, y: offset.y },
        )),
        (
            MatrixTransformFields::Column { column, offset, .. },
            Field::ColumnOffsetY,
            Value::Number(y),
        ) => Ok(set_matrix_offset(
            matrix,
            false,
            *column,
            Vec2 { x: offset.x, y },
        )),
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnOffsetReset,
            Value::Offset(offset),
        ) => Ok(set_matrix_offset(matrix, false, *column, offset)),
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnStagger,
            Value::Number(stagger),
        ) => {
            let mut next = matrix.clone();
            next.column_staggers = (0..matrix.columns)
                .map(|index| {
                    if index == *column {
                        stagger
                    } else {
                        matrix
                            .column_staggers
                            .get(index as usize)
                            .copied()
                            .unwrap_or(0.0)
                    }
                })
                .collect();
            Ok(set_matrix(next))
        }
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnSplay,
            Value::Number(angle),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Angle { angle, affect },
        }),
        (
            MatrixTransformFields::Column {
                column,
                splay_origin,
                ..
            },
            Field::SplayOriginMode,
            Value::OriginMode(custom),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Origin {
                world: custom.then_some(*splay_origin),
            },
        }),
        (
            MatrixTransformFields::Column {
                column,
                splay_origin,
                ..
            },
            Field::SplayOriginX,
            Value::Number(x),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Origin {
                world: Some(Vec2 {
                    x,
                    y: splay_origin.y,
                }),
            },
        }),
        (
            MatrixTransformFields::Column {
                column,
                splay_origin,
                ..
            },
            Field::SplayOriginY,
            Value::Number(y),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Origin {
                world: Some(Vec2 {
                    x: splay_origin.x,
                    y,
                }),
            },
        }),
        (
            MatrixTransformFields::Key {
                row,
                column,
                offset,
                ..
            },
            Field::KeyOffsetX,
            Value::Number(x),
        ) => {
            let cell_offset = Vec2 { x, y: offset.y };
            Ok(set_cell_transform(matrix, *row, *column, cell_offset, None))
        }
        (
            MatrixTransformFields::Key {
                row,
                column,
                offset,
                ..
            },
            Field::KeyOffsetY,
            Value::Number(y),
        ) => {
            let cell_offset = Vec2 { x: offset.x, y };
            Ok(set_cell_transform(matrix, *row, *column, cell_offset, None))
        }
        (
            MatrixTransformFields::Key {
                row,
                column,
                offset,
                ..
            },
            Field::KeyRotation,
            Value::Number(rotation),
        ) => Ok(set_cell_transform(
            matrix,
            *row,
            *column,
            *offset,
            Some(rotation),
        )),
        (
            MatrixTransformFields::Key { row, column, .. },
            Field::KeyTransformReset,
            Value::CellTransform { offset, rotation },
        ) => Ok(set_cell_transform(
            matrix,
            *row,
            *column,
            offset,
            Some(rotation),
        )),
        _ => Err("This transform field no longer matches the selected matrix context.".into()),
    }
}

fn set_matrix(matrix: Matrix) -> EditOperation {
    EditOperation::SetMatrix {
        matrix,
        definitions: None,
    }
}

fn set_matrix_offset(matrix: &Matrix, row_axis: bool, index: u32, value: Vec2) -> EditOperation {
    let mut next = matrix.clone();
    let offsets = if row_axis {
        &mut next.row_offsets
    } else {
        &mut next.column_offsets
    };
    while offsets.len() <= index as usize {
        offsets.push(Vec2 { x: 0.0, y: 0.0 });
    }
    offsets[index as usize] = value;
    set_matrix(next)
}

fn set_cell_transform(
    matrix: &Matrix,
    row: u32,
    column: u32,
    offset: Vec2,
    rotation: Option<f64>,
) -> EditOperation {
    let mut next = matrix.clone();
    let mut cell = next
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .cloned()
        .unwrap_or(MatrixCell {
            row,
            column,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: None,
        });
    cell.offset = Some(offset);
    if let Some(rotation) = rotation {
        cell.rotation = Some(rotation);
    }
    if let Some(existing) = next
        .cells
        .iter_mut()
        .find(|existing| existing.row == row && existing.column == column)
    {
        *existing = cell;
    } else {
        next.cells.push(cell);
    }
    set_matrix(next)
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
    if scope.as_ref() != Some(&waiting.request.owner.scope)
        || scope_generation != waiting.request.owner.scope_generation
    {
        pending.set(None);
        feedback.set(Vec::new());
        return;
    }
    let Some(outcome) = waiting.outcome.borrow().clone() else {
        return;
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
                    workspace: "Layout",
                },
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::MatrixAssembly;

    fn matrix() -> Matrix {
        Matrix {
            id: "matrix-1".into(),
            name: Some("Fixture".into()),
            rows: 2,
            columns: 3,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2 { x: 12.0, y: -4.0 },
            definition_id: "switch".into(),
            part_ids: vec!["p0".into()],
            board_id: Some("board-1".into()),
            mirror: None,
            rotation: Some(0.0),
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![Vec2 { x: 2.0, y: 3.0 }, Vec2 { x: 4.0, y: 5.0 }],
            column_offsets: vec![Vec2 { x: 1.0, y: 2.0 }],
            column_staggers: vec![0.25, 0.5, 0.75],
            column_splays: vec![0.0, 4.0, 8.0],
            column_origins: Vec::new(),
            cells: vec![MatrixCell {
                row: 1,
                column: 2,
                enabled: true,
                definition_id: Some("switch".into()),
                variant: Some("north".into()),
                offset: Some(Vec2 { x: 6.0, y: 7.0 }),
                rotation: Some(9.0),
                assemblies: vec![MatrixAssembly {
                    id: "led".into(),
                    definition_id: "led-def".into(),
                    offset: Vec2 { x: 1.0, y: 0.0 },
                    rotation: Some(90.0),
                    side: None,
                }],
                assemblies_local: Some(true),
            }],
        }
    }

    #[test]
    fn row_offset_commit_changes_only_the_selected_row_and_preserves_other_matrix_data() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Row {
            row: 1,
            offset: Vec2 { x: 4.0, y: 5.0 },
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::RowOffsetX,
            MatrixTransformValue::Number(-2.5),
            MatrixSplayAffect::Following,
        )
        .expect("row offset is a supported accepted edit");
        let EditOperation::SetMatrix {
            matrix: next,
            definitions,
        } = operation
        else {
            panic!("row edits use SetMatrix");
        };
        assert_eq!(
            next.row_offsets,
            vec![Vec2 { x: 2.0, y: 3.0 }, Vec2 { x: -2.5, y: 5.0 }]
        );
        assert_eq!(next.cells, matrix.cells);
        assert_eq!(next.column_staggers, matrix.column_staggers);
        assert_eq!(next.origin, matrix.origin);
        assert!(definitions.is_none());
    }

    #[test]
    fn key_reset_changes_local_pose_without_replacing_assembly_or_cell_identity() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Key {
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyTransformReset,
            MatrixTransformValue::CellTransform {
                offset: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            MatrixSplayAffect::Following,
        )
        .expect("cell-local reset is a supported accepted edit");
        let EditOperation::SetMatrix { matrix: next, .. } = operation else {
            panic!("cell edits use SetMatrix");
        };
        assert_eq!(next.cells.len(), 1);
        let cell = &next.cells[0];
        assert_eq!((cell.row, cell.column), (1, 2));
        assert_eq!(cell.offset, Some(Vec2 { x: 0.0, y: 0.0 }));
        assert_eq!(cell.rotation, Some(0.0));
        assert_eq!(cell.definition_id.as_deref(), Some("switch"));
        assert_eq!(cell.variant.as_deref(), Some("north"));
        assert_eq!(cell.assemblies, matrix.cells[0].assemblies);
        assert_eq!(cell.assemblies_local, Some(true));
    }

    #[test]
    fn editing_an_empty_key_cell_creates_only_its_matrix_cell_record() {
        let mut matrix = matrix();
        matrix.cells.clear();
        matrix.part_ids = vec!["existing-primary".into()];
        let fields = MatrixTransformFields::Key {
            row: 0,
            column: 1,
            offset: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyOffsetX,
            MatrixTransformValue::Number(-1.25),
            MatrixSplayAffect::Following,
        )
        .expect("a semantic empty cell can receive a local transform");
        let EditOperation::SetMatrix { matrix: next, .. } = operation else {
            panic!("cell-local fields use SetMatrix");
        };
        assert_eq!(next.part_ids, vec!["existing-primary"]);
        assert_eq!(next.cells.len(), 1);
        assert_eq!((next.cells[0].row, next.cells[0].column), (0, 1));
        assert_eq!(next.cells[0].offset, Some(Vec2 { x: -1.25, y: 0.0 }));
        assert!(next.cells[0].enabled);
        assert!(next.cells[0].definition_id.is_none());
        assert!(next.cells[0].assemblies.is_empty());
    }

    #[test]
    fn editing_one_cell_preserves_the_order_and_fields_of_other_cells() {
        let mut matrix = matrix();
        matrix.cells.insert(
            0,
            MatrixCell {
                row: 0,
                column: 1,
                enabled: false,
                definition_id: Some("alternate".into()),
                variant: Some("south".into()),
                offset: Some(Vec2 { x: -3.0, y: 2.0 }),
                rotation: Some(-4.0),
                assemblies: Vec::new(),
                assemblies_local: None,
            },
        );
        let untouched = matrix.cells[0].clone();
        let fields = MatrixTransformFields::Key {
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
        };

        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyOffsetX,
            MatrixTransformValue::Number(-1.0),
            MatrixSplayAffect::Following,
        )
        .expect("a cell-local edit is supported");
        let EditOperation::SetMatrix { matrix: next, .. } = operation else {
            panic!("cell-local fields use SetMatrix");
        };

        assert_eq!(next.cells.len(), 2);
        assert_eq!(next.cells[0], untouched);
        assert_eq!(next.cells[1].offset, Some(Vec2 { x: -1.0, y: 7.0 }));
        assert_eq!(next.cells[1].assemblies, matrix.cells[1].assemblies);
    }

    #[test]
    fn splay_angle_uses_the_selected_affect_and_origin_mode_uses_existing_operations() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Column {
            column: 2,
            offset: Vec2 { x: 0.0, y: 0.0 },
            stagger: 0.0,
            splay_angle: 8.0,
            splay_origin: Vec2 { x: 11.0, y: 12.0 },
            custom_origin: false,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::ColumnSplay,
            MatrixTransformValue::Number(12.5),
            MatrixSplayAffect::Column,
        )
        .expect("splay uses the existing typed operation");
        assert!(matches!(operation, EditOperation::SetMatrixSplay {
            matrix_id, column: 2,
            change: MatrixSplayChange::Angle { angle: 12.5, affect: MatrixSplayAffect::Column }
        } if matrix_id == "matrix-1"));
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::SplayOriginMode,
            MatrixTransformValue::OriginMode(true),
            MatrixSplayAffect::Following,
        )
        .expect("custom origin uses the existing typed operation");
        assert!(matches!(
            operation,
            EditOperation::SetMatrixSplay {
                column: 2,
                change: MatrixSplayChange::Origin {
                    world: Some(Vec2 { x: 11.0, y: 12.0 })
                },
                ..
            }
        ));
    }

    #[test]
    fn mirror_y_is_rejected_for_a_linked_layout_even_if_a_stale_ui_requests_it() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: 0.0,
            mirror: None,
            mirror_y_locked: true,
        };
        let error = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::MatrixMirror,
            MatrixTransformValue::Mirror(Some(Mirror::Y)),
            MatrixSplayAffect::Following,
        )
        .expect_err("linked layout mirrors must retain the React restriction");
        assert!(error.contains("linked layout"));
    }
}
