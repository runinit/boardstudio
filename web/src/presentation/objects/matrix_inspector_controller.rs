//! Fresh accepted-source admission and exact operation acknowledgement for matrix fields.
use super::matrix_inspector::{
    MatrixEditFeedback, MatrixEditField, MatrixEditRequest, MatrixEditState, MatrixEditValue,
    MatrixInspectorOwner, MatrixInspectorProjection, MatrixNameTarget,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    Durability, Event, Lifecycle, OperationId, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Matrix, MatrixAssembly, MatrixCell, ProjectDoc,
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct PendingMatrixEdit {
    request: MatrixEditRequest,
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

/// Projection and exact callback state to mount beside the selected Layout context.
#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct MatrixInspectorMount {
    pub projection: Option<MatrixInspectorProjection>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub busy: bool,
    pub feedback: Vec<MatrixEditFeedback>,
    pub on_edit: EventHandler<MatrixEditRequest>,
}

/// Must be called unconditionally at the shared presentation lifetime. It owns no document
/// state: every projection and callback resolves the current accepted snapshot again.
pub(in crate::presentation) fn use_matrix_inspector(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> MatrixInspectorMount {
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
                .expect("matrix inspector context generation exhausted");
            tracker.identity = observed_identity.clone();
            tracker.initialized = true;
        }
        tracker.value
    };

    let request_sequence = use_signal(|| 0u64);
    let last_request_id = use_signal(|| 0u64);
    let pending = use_signal(|| None::<PendingMatrixEdit>);
    let feedback = use_signal(Vec::<MatrixEditFeedback>::new);
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
        move |request: MatrixEditRequest| {
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
                // Keep the active request's exact feedback, but settle this distinct rejected
                // attempt so its field draft cannot remain visually pending forever.
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixEditState::Failed,
                    Some("Wait for the current matrix change to finish, then retry.".into()),
                );
                return;
            }
            let Some(selected) = selected_context.read().clone() else {
                return;
            };
            let live_scope = runtime.scope();
            let model = runtime.model();
            let Some((current, editable)) = project_current_for(
                &runtime,
                &model,
                live_scope.as_ref(),
                Some(&selected),
                MatrixProjectionContext {
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
                    MatrixEditState::Failed,
                    Some("The matrix is not ready to edit. Wait for the current operation to finish, then retry.".into()),
                );
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let Some(current_baseline) = field_value(
                &snapshot.document,
                &request.owner.matrix_id,
                &request.owner.name_target,
                request.field,
            ) else {
                return;
            };
            if current_baseline != request.baseline {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixEditState::Failed,
                    Some(
                        "The accepted value changed. Press Escape to reload it before editing."
                            .into(),
                    ),
                );
                return;
            }
            if request.value == request.baseline {
                publish_feedback(&mut feedback, &request, MatrixEditState::Saved, None);
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
                &snapshot.document,
                matrix,
                &request.owner.name_target,
                request.field,
                request.value.clone(),
            ) {
                Ok(operation) => operation,
                Err(message) => {
                    publish_feedback(
                        &mut feedback,
                        &request,
                        MatrixEditState::Failed,
                        Some(message),
                    );
                    return;
                }
            };
            let target_id = match &request.owner.name_target {
                MatrixNameTarget::Matrix => request.owner.matrix_id.clone(),
                MatrixNameTarget::Layout { id } if request.field == MatrixEditField::Name => {
                    id.clone()
                }
                MatrixNameTarget::Layout { .. } => request.owner.matrix_id.clone(),
            };
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let transaction_id = format!(
                "matrix-inspector-{}-{}-{}",
                editor_instance_id, request.request_id, operation_id.0
            );
            pending.set(Some(PendingMatrixEdit {
                request: request.clone(),
                operation_id,
                outcome,
                base_token: snapshot.token,
                base_revision: snapshot.document.revision,
            }));
            publish_feedback(&mut feedback, &request, MatrixEditState::Pending, None);
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id,
                    phase: EditPhase::Commit,
                    target_ids: vec![target_id],
                    operation,
                },
            });
        }
    });

    MatrixInspectorMount {
        projection,
        request_sequence,
        editable,
        busy: pending.read().is_some(),
        feedback: feedback.read().clone(),
        on_edit,
    }
}

fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
) -> (Option<MatrixInspectorProjection>, bool) {
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
        MatrixProjectionContext {
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

struct MatrixProjectionContext {
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
    context: MatrixProjectionContext,
) -> Option<(MatrixInspectorProjection, bool)> {
    let MatrixProjectionContext {
        editor_instance_id,
        context_generation,
        scope_generation,
        workspace,
    } = context;
    if workspace != "Layout" {
        return None;
    }
    let scope = live_scope?;
    let selected = selected?;
    if &selected.scope != scope || runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let TreeContext::Matrix { matrix_id } = &selected.context else {
        return None;
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
    let layout = snapshot
        .document
        .layouts
        .iter()
        .find(|layout| layout.board_id == scope.board_id && layout.matrix_id == matrix.id);
    let (name_target, name_label, name_value, name_baseline) = if let Some(layout) = layout {
        (
            MatrixNameTarget::Layout {
                id: layout.id.clone(),
            },
            "Layout name",
            layout.name.clone(),
            MatrixEditValue::Name(Some(layout.name.clone())),
        )
    } else {
        let name = matrix.name.clone();
        let fallback = super::context_label(model, &selected.context)
            .unwrap_or_else(|| format!("Matrix {}", matrix.id));
        (
            MatrixNameTarget::Matrix,
            "Matrix name",
            name.clone().unwrap_or(fallback),
            MatrixEditValue::Name(name),
        )
    };
    let saved = model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: snapshot.document.revision,
            });
    let editable = saved && model.display_preview.is_none() && model.gesture.is_none();
    Some((
        MatrixInspectorProjection {
            owner: MatrixInspectorOwner {
                editor_instance_id,
                context_generation,
                scope_generation,
                scope: scope.clone(),
                matrix_id: matrix.id.clone(),
                name_target,
            },
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            matrix_label: super::context_label(model, &selected.context)
                .unwrap_or_else(|| format!("Matrix {}", matrix.id)),
            name_label,
            name_value,
            name_baseline,
            rows: matrix.rows,
            columns: matrix.columns,
            pitch_x: matrix.pitch.x,
            pitch_y: matrix.pitch.y,
        },
        editable,
    ))
}

fn settle_pending(
    runtime: &Runtime,
    scope_generation: u64,
    pending: &mut Signal<Option<PendingMatrixEdit>>,
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
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
                finish_pending(
                    pending,
                    feedback,
                    &waiting,
                    MatrixEditState::Failed,
                    Some("The matrix operation completed, but the accepted document did not save. Retry after recovery.".into()),
                );
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
            let value = field_value(
                &snapshot.document,
                &waiting.request.owner.matrix_id,
                &waiting.request.owner.name_target,
                waiting.request.field,
            );
            if value.as_ref() == Some(&waiting.request.value) {
                finish_pending(pending, feedback, &waiting, MatrixEditState::Saved, None);
            } else {
                finish_pending(
                    pending,
                    feedback,
                    &waiting,
                    MatrixEditState::Failed,
                    Some("The saved matrix does not contain the requested value. Review the current field and retry.".into()),
                );
            }
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => finish_pending(
            pending,
            feedback,
            &waiting,
            MatrixEditState::Failed,
            Some(message),
        ),
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            finish_pending(
                pending,
                feedback,
                &waiting,
                MatrixEditState::Failed,
                Some("The matrix edit did not complete in the active session. Review the current value and retry.".into()),
            );
        }
    }
}

fn finish_pending(
    pending: &mut Signal<Option<PendingMatrixEdit>>,
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    waiting: &PendingMatrixEdit,
    state: MatrixEditState,
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
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    request: &MatrixEditRequest,
    state: MatrixEditState,
    message: Option<String>,
) {
    let next = MatrixEditFeedback {
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
        let removable = entries
            .iter()
            .position(|entry| entry.state != MatrixEditState::Pending);
        let Some(index) = removable else {
            break;
        };
        entries.remove(index);
    }
    feedback.set(entries);
}

fn field_value(
    document: &ProjectDoc,
    matrix_id: &str,
    name_target: &MatrixNameTarget,
    field: MatrixEditField,
) -> Option<MatrixEditValue> {
    match field {
        MatrixEditField::Name => match name_target {
            MatrixNameTarget::Matrix => document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .map(|matrix| MatrixEditValue::Name(matrix.name.clone())),
            MatrixNameTarget::Layout { id } => document
                .layouts
                .iter()
                .find(|layout| layout.id == *id && layout.matrix_id == matrix_id)
                .map(|layout| MatrixEditValue::Name(Some(layout.name.clone()))),
        },
        MatrixEditField::Rows => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Rows(matrix.rows)),
        MatrixEditField::Columns => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Columns(matrix.columns)),
        MatrixEditField::PitchX => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::PitchX(matrix.pitch.x)),
        MatrixEditField::PitchY => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::PitchY(matrix.pitch.y)),
    }
}

fn build_operation(
    document: &ProjectDoc,
    matrix: &Matrix,
    name_target: &MatrixNameTarget,
    field: MatrixEditField,
    value: MatrixEditValue,
) -> Result<EditOperation, String> {
    if field == MatrixEditField::Name {
        let MatrixEditValue::Name(Some(name)) = value else {
            return Err("Enter a non-empty matrix or layout name.".into());
        };
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a non-empty matrix or layout name.".into());
        }
        return match name_target {
            MatrixNameTarget::Matrix => {
                let mut replacement = matrix.clone();
                replacement.name = Some(name.to_owned());
                Ok(EditOperation::SetMatrix {
                    matrix: replacement,
                    definitions: None,
                })
            }
            MatrixNameTarget::Layout { id } => {
                let mut layout = document
                    .layouts
                    .iter()
                    .find(|layout| layout.id == *id && layout.matrix_id == matrix.id)
                    .cloned()
                    .ok_or_else(|| "The selected layout no longer owns this matrix.".to_owned())?;
                layout.name = name.to_owned();
                Ok(EditOperation::SetLayout { layout })
            }
        };
    }
    if matches!(name_target, MatrixNameTarget::Layout { .. }) && field == MatrixEditField::Name {
        return Err("The layout name target is unavailable.".into());
    }
    let replacement = matrix_edit(matrix, field, value)?;
    Ok(EditOperation::SetMatrix {
        matrix: replacement,
        definitions: None,
    })
}

/// Apply one real matrix field edit to a fresh accepted matrix. Row/column edits use the exact
/// presentation request recipe from the existing React `matrixResize.ts`; Core still validates
/// and applies the replacement through `SetMatrix`.
fn matrix_edit(
    matrix: &Matrix,
    field: MatrixEditField,
    value: MatrixEditValue,
) -> Result<Matrix, String> {
    match (field, value) {
        (MatrixEditField::Rows, MatrixEditValue::Rows(rows)) => {
            resize_matrix(matrix, rows, matrix.columns)
        }
        (MatrixEditField::Columns, MatrixEditValue::Columns(columns)) => {
            resize_matrix(matrix, matrix.rows, columns)
        }
        (MatrixEditField::PitchX, MatrixEditValue::PitchX(x)) if x.is_finite() && x > 0.0 => {
            let mut next = matrix.clone();
            next.pitch.x = x;
            Ok(next)
        }
        (MatrixEditField::PitchY, MatrixEditValue::PitchY(y)) if y.is_finite() && y > 0.0 => {
            let mut next = matrix.clone();
            next.pitch.y = y;
            Ok(next)
        }
        _ => Err("The matrix field value is invalid.".into()),
    }
}

/// Port of `app/src/ui/matrixResize.ts`, limited to the accepted Matrix request object.
fn resize_matrix(matrix: &Matrix, rows: u32, columns: u32) -> Result<Matrix, String> {
    if rows == 0 || columns == 0 {
        return Err("Matrix dimensions must be positive whole numbers.".into());
    }
    if rows
        .checked_mul(columns)
        .is_none_or(|cell_count| cell_count > 4096)
    {
        return Err("Matrix dimensions cannot exceed 4096 cells.".into());
    }
    let template = matrix.cells.iter().find(|cell| {
        cell.definition_id.as_deref() == Some(matrix.definition_id.as_str())
            && cell
                .variant
                .as_deref()
                .is_some_and(|variant| variant.starts_with("preset/"))
    });
    let mut replacement = matrix.clone();
    replacement.rows = rows;
    replacement.columns = columns;
    let Some(template) = template else {
        return Ok(replacement);
    };

    replacement
        .cells
        .retain(|cell| cell.row < rows && cell.column < columns);
    let assemblies: Vec<MatrixAssembly> = template
        .assemblies
        .iter()
        .filter(|assembly| assembly.id == "diode" || assembly.id == "led")
        .cloned()
        .collect();
    let definition_id = template.definition_id.clone();
    let variant = template.variant.clone();
    let assemblies_local = template.assemblies_local;
    let rotation = Some(
        if variant
            .as_deref()
            .is_some_and(|variant| variant.ends_with("/north"))
        {
            180.0
        } else {
            0.0
        },
    );
    for row in 0..rows {
        for column in 0..columns {
            if row < matrix.rows && column < matrix.columns {
                continue;
            }
            replacement.cells.push(MatrixCell {
                row,
                column,
                enabled: true,
                definition_id: definition_id.clone(),
                variant: variant.clone(),
                offset: None,
                rotation,
                assemblies: assemblies.clone(),
                assemblies_local,
            });
        }
    }
    Ok(replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reviung_document() -> ProjectDoc {
        serde_json::from_str(include_str!(
            "../../../../core/tests/fixtures/reviung41-outline-original.json"
        ))
        .expect("checked-in Reviung 41 project fixture should deserialize")
    }

    fn reviung_matrix() -> Matrix {
        let document = reviung_document();
        document
            .matrices
            .into_iter()
            .find(|matrix| matrix.id == "main-right-keys")
            .expect("fixture has the right-hand 3 by 6 matrix")
    }

    fn request_matrix(matrix: &Matrix, field: MatrixEditField, value: MatrixEditValue) -> Matrix {
        let document = reviung_document();
        match build_operation(&document, matrix, &MatrixNameTarget::Matrix, field, value)
            .expect("the selected matrix field should produce a SetMatrix request")
        {
            EditOperation::SetMatrix { matrix, .. } => matrix,
            _ => panic!("matrix fields should use SetMatrix"),
        }
    }

    #[test]
    fn resizing_real_preset_matrix_preserves_cells_and_inherits_only_electrical_members() {
        let mut matrix = reviung_matrix();
        matrix.rows = 2;
        matrix.columns = 2;
        let old_rows = matrix.rows;
        let old_columns = matrix.columns;
        matrix
            .cells
            .retain(|cell| cell.row < old_rows && cell.column < old_columns);
        let original = matrix.cells[0].clone();
        matrix.cells[0].variant = Some("preset/mx-hotswap/north".into());
        matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "led".into(),
            definition_id: "fixture-led".into(),
            offset: boardstudio_core::model::Vec2 { x: 2.0, y: -3.0 },
            rotation: Some(27.0),
            side: Some(boardstudio_core::model::Side::Back),
        });
        matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "local-extra".into(),
            definition_id: "fixture-extra".into(),
            offset: boardstudio_core::model::Vec2 { x: 9.0, y: 8.0 },
            rotation: None,
            side: None,
        });
        let template = matrix.cells[0].clone();

        let resized = request_matrix(
            &matrix,
            MatrixEditField::Columns,
            MatrixEditValue::Columns(3),
        );
        assert_eq!(resized.rows, 2);
        assert_eq!(resized.columns, 3);
        assert_eq!(resized.cells[0].row, original.row);
        assert_eq!(resized.cells[0].offset, original.offset);
        assert_eq!(resized.cells[0].enabled, original.enabled);
        let added = resized
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.column == 2)
            .unwrap();
        assert!(added.enabled);
        assert_eq!(
            added.definition_id.as_deref(),
            Some(matrix.definition_id.as_str())
        );
        assert_eq!(added.variant.as_deref(), Some("preset/mx-hotswap/north"));
        assert_eq!(added.rotation, Some(180.0));
        assert_eq!(added.offset, None);
        assert_eq!(added.assemblies_local, template.assemblies_local);
        assert_eq!(
            added.assemblies,
            template
                .assemblies
                .iter()
                .filter(|assembly| assembly.id == "diode" || assembly.id == "led")
                .cloned()
                .collect::<Vec<_>>()
        );
        assert_eq!(added.assemblies.len(), 2);
        assert!(
            !added
                .assemblies
                .iter()
                .any(|assembly| assembly.id == "local-extra")
        );
        assert_eq!(matrix.cells[0].assemblies.len(), template.assemblies.len());
    }

    #[test]
    fn resize_preserves_sparse_holes_and_trims_only_cells_outside_new_bounds() {
        let mut matrix = reviung_matrix();
        matrix.rows = 2;
        matrix.columns = 2;
        let old_rows = matrix.rows;
        let old_columns = matrix.columns;
        matrix
            .cells
            .retain(|cell| cell.row < old_rows && cell.column < old_columns);
        matrix
            .cells
            .retain(|cell| !(cell.row == 1 && cell.column == 1));
        matrix.cells.push(MatrixCell {
            row: 2,
            column: 5,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: None,
        });
        let before = matrix
            .cells
            .iter()
            .filter(|cell| cell.row < 2 && cell.column < 2)
            .cloned()
            .collect::<Vec<_>>();
        let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(3));
        let retained = resized
            .cells
            .iter()
            .filter(|cell| cell.row < 2 && cell.column < 2)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(retained, before);
        assert!(
            !resized
                .cells
                .iter()
                .any(|cell| cell.row == 1 && cell.column == 1)
        );
        assert!(
            !resized
                .cells
                .iter()
                .any(|cell| cell.row == 2 && cell.column == 5)
        );
        assert!(
            resized
                .cells
                .iter()
                .any(|cell| cell.row == 2 && cell.column == 1)
        );
    }

    #[test]
    fn custom_resize_leaves_cells_unchanged_and_rejects_oversized_dimensions_before_allocation() {
        let mut matrix = reviung_matrix();
        matrix.cells.iter_mut().for_each(|cell| cell.variant = None);
        let cells = matrix.cells.clone();
        let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(4));
        assert_eq!(resized.cells, cells);
        assert!(
            matrix_edit(
                &matrix,
                MatrixEditField::Rows,
                MatrixEditValue::Rows(u32::MAX),
            )
            .is_err()
        );
        assert!(matrix_edit(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(0),).is_err());
    }

    #[test]
    fn matrix_pitch_request_preserves_every_unrelated_accepted_matrix_field() {
        let matrix = reviung_matrix();
        let next = request_matrix(
            &matrix,
            MatrixEditField::PitchX,
            MatrixEditValue::PitchX(20.0),
        );
        assert_eq!(next.pitch.x, 20.0);
        assert_eq!(next.pitch.y, matrix.pitch.y);
        assert_eq!(next.rows, matrix.rows);
        assert_eq!(next.columns, matrix.columns);
        assert_eq!(next.cells, matrix.cells);
        assert_eq!(next.part_ids, matrix.part_ids);
        assert_eq!(next.board_id, matrix.board_id);
    }

    #[test]
    fn owning_layout_name_request_uses_set_layout_without_renaming_the_matrix() {
        let document = reviung_document();
        let matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "main-right-keys")
            .unwrap();
        let layout = document
            .layouts
            .iter()
            .find(|layout| layout.matrix_id == matrix.id && layout.board_id == "main")
            .unwrap();
        let operation = build_operation(
            &document,
            matrix,
            &MatrixNameTarget::Layout {
                id: layout.id.clone(),
            },
            MatrixEditField::Name,
            MatrixEditValue::Name(Some("Renamed layout".into())),
        )
        .unwrap();
        let EditOperation::SetLayout { layout: renamed } = operation else {
            panic!("layout name changes should use SetLayout");
        };
        assert_eq!(renamed.id, layout.id);
        assert_eq!(renamed.matrix_id, matrix.id);
        assert_eq!(renamed.name, "Renamed layout");
        assert_eq!(matrix.name.as_deref(), Some("right keys"));
    }
}
