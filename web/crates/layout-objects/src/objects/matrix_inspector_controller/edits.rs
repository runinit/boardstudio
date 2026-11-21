//! Field and action edit plumbing for the Matrix Inspector: resolvers, feedback, geometry.
use super::*;

use super::projection::{matrix_preset_variant, preset_variant};
use super::variant::{
    current_variant_snapshot, matrix_setup_preset, navigate_variant_to_source_context,
    restore_source_after_variant_failure, wait_for_matrix_project, wait_for_variant_edit,
};

pub(super) fn settle_action_results(
    runtime: &Rc<Runtime>,
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    mut selected_context: Signal<Option<ScopedTreeContext>>,
    results: Vec<PendingEditResult<MatrixPendingKey>>,
) {
    for result in results {
        match result {
            PendingEditResult::Landed {
                key: MatrixPendingKey::Action(MatrixActionKey::Preset(Some(request))),
                ..
            } => clear_action_feedback(feedback, &request.owner, request.request_id),
            PendingEditResult::Failed {
                key: MatrixPendingKey::Action(MatrixActionKey::Preset(Some(request))),
                message,
            } => publish_action_feedback(
                feedback,
                &request.owner,
                request.request_id,
                MatrixEditField::ApplyPreset,
                message,
            ),
            PendingEditResult::Retired {
                key: MatrixPendingKey::Action(MatrixActionKey::Preset(Some(request))),
            } => clear_action_feedback(feedback, &request.owner, request.request_id),
            PendingEditResult::Landed {
                key: MatrixPendingKey::Action(MatrixActionKey::Delete(Some(request))),
                ..
            } => {
                if selected_context.read().as_ref().is_some_and(|selected| {
                    selected.scope == request.owner.scope
                        && matches!(
                            &selected.context,
                            TreeContext::Matrix { matrix_id }
                                if matrix_id == &request.owner.matrix_id
                        )
                }) {
                    selected_context.set(None);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: Vec::new(),
                        range_part_ids: Vec::new(),
                        mode: boardstudio_application::SelectionMode::Replace,
                    });
                }
            }
            PendingEditResult::Failed {
                key: MatrixPendingKey::Action(MatrixActionKey::Delete(Some(_))),
                message,
            } => runtime.report(format!("Could not delete matrix: {message}")),
            PendingEditResult::Retired {
                key: MatrixPendingKey::Action(MatrixActionKey::Delete(Some(_))),
            }
            | PendingEditResult::Landed {
                key: MatrixPendingKey::Action(MatrixActionKey::Unlink),
                ..
            }
            | PendingEditResult::Retired {
                key: MatrixPendingKey::Action(MatrixActionKey::Unlink),
            } => {}
            PendingEditResult::Failed {
                key: MatrixPendingKey::Action(MatrixActionKey::Unlink),
                message,
            } => runtime.report(format!("Could not unlink the halves: {message}")),
            // All action keys are created with their request metadata. These defensive arms
            // keep a malformed key from producing misleading feedback or a false recovery.
            PendingEditResult::Landed { .. }
            | PendingEditResult::Failed { .. }
            | PendingEditResult::Retired { .. } => {}
        }
    }
}

pub(super) fn clear_action_feedback(
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    owner: &MatrixInspectorOwner,
    request_id: u64,
) {
    let mut entries = feedback.read().clone();
    entries.retain(|entry| entry.owner != *owner || entry.request_id != request_id);
    feedback.set(entries);
}

pub(super) fn publish_action_feedback(
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    owner: &MatrixInspectorOwner,
    request_id: u64,
    field: MatrixEditField,
    message: String,
) {
    let next = MatrixEditFeedback {
        owner: owner.clone(),
        request_id,
        field,
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
        entries.remove(0);
    }
    feedback.set(entries);
}

pub(super) fn publish_feedback(
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    request: &MatrixEditRequest,
    message: String,
) {
    let next = MatrixEditFeedback {
        owner: request.owner.clone(),
        request_id: request.request_id,
        field: request.field,
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
        entries.remove(0);
    }
    feedback.set(entries);
}

pub(super) fn field_value(
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
                .find(|layout| {
                    layout.id == *id && layout.matrix_ids.iter().any(|id| *id == matrix_id)
                })
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
        MatrixEditField::SwitchDefinition => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::SwitchDefinition(matrix.definition_id.clone())),
        MatrixEditField::DiodeDirection => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::DiodeDirection(
                    matrix.diode_direction.unwrap_or(DiodeDirection::Row2col),
                )
            }),
        MatrixEditField::EdgeGapX => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::EdgeGapX(matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.x))
            }),
        MatrixEditField::EdgeGapY => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::EdgeGapY(matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.y))
            }),
        MatrixEditField::ApplyPreset => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Name(matrix_preset_variant(matrix))),
    }
}

pub(super) fn build_operation(
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
                    .find(|layout| {
                        layout.id == *id && layout.matrix_ids.iter().any(|id| id == &matrix.id)
                    })
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
pub(super) fn matrix_edit(
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
        (MatrixEditField::SwitchDefinition, MatrixEditValue::SwitchDefinition(id))
            if !id.trim().is_empty() =>
        {
            let mut next = matrix.clone();
            next.definition_id = id;
            Ok(next)
        }
        (MatrixEditField::DiodeDirection, MatrixEditValue::DiodeDirection(direction)) => {
            let mut next = matrix.clone();
            next.diode_direction = Some(direction);
            Ok(next)
        }
        (MatrixEditField::EdgeGapX, MatrixEditValue::EdgeGapX(x)) if x.is_finite() && x >= 0.0 => {
            let mut next = matrix.clone();
            let mut gap = next.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            gap.x = x;
            next.edge_gap = Some(gap);
            Ok(next)
        }
        (MatrixEditField::EdgeGapY, MatrixEditValue::EdgeGapY(y)) if y.is_finite() && y >= 0.0 => {
            let mut next = matrix.clone();
            let mut gap = next.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            gap.y = y;
            next.edge_gap = Some(gap);
            Ok(next)
        }
        _ => Err("The matrix field value is invalid.".into()),
    }
}

/// Build the accepted matrix resize request.
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
                shift: None,
                bend: None,
                deleted: false,
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

pub(super) async fn prepare_preset_matrix(
    document: &ProjectDoc,
    matrix: &Matrix,
    board_id: &str,
    preset: MatrixPreset,
    orientation: SwitchOrientation,
    reversible: bool,
) -> Result<PreparedPreset, String> {
    let templates = crate::presentation::parts::load_matrix_templates(reversible).await?;
    let setup_preset = matrix_setup_preset(preset);
    let mut prepared = crate::matrix_setup_operation::prepare_matrix(
        matrix.id.clone(),
        board_id.to_owned(),
        crate::matrix_setup_operation::MatrixSetupRequest {
            rows: matrix.rows,
            columns: matrix.columns,
            preset: setup_preset,
        },
        reversible,
        &templates,
    )?;
    for definition in &mut prepared.definitions {
        *definition =
            crate::presentation::parts::normalize_matrix_definition(definition.clone()).await?;
        if document
            .definitions
            .iter()
            .any(|existing| existing.id == definition.id && existing != definition)
        {
            return Err(format!(
                "The prepared matrix definition {} conflicts with an existing definition.",
                definition.id
            ));
        }
    }
    Ok(PreparedPreset {
        matrix: prepared.matrix,
        definitions: prepared.definitions,
        variant: preset_variant(preset, orientation, reversible),
        orientation,
    })
}

/// Clone the whole accepted project, open it through the Session lifecycle, and apply the chosen
/// assembly preset to the copy. If preset preparation or application fails, reopen the saved
/// source project so the user returns to the design they started from.
#[allow(clippy::too_many_arguments)]
pub(super) async fn duplicate_design_variant(
    runtime: Rc<Runtime>,
    request: &MatrixDuplicateRequest,
    mut document: ProjectDoc,
    alive: Rc<Cell<bool>>,
    report_scope: Rc<RefCell<Option<VariantReportOwner>>>,
) -> Result<VariantSetupResult, String> {
    if !alive.get() {
        return Err("The matrix design variant owner closed.".into());
    }
    let source_session_epoch = request.owner.scope.session_epoch;
    let variant_session_epoch = boardstudio_application::SessionEpoch(
        source_session_epoch
            .0
            .checked_add(1)
            .ok_or_else(|| "The project session identity is exhausted.".to_owned())?,
    );
    document.id = crate::runtime::new_project_id()?;
    let source_name = document.name.trim();
    let source_name = if source_name.is_empty() {
        "Untitled keyboard"
    } else {
        source_name
    };
    document.name = format!("{source_name} (variant)");
    let variant_id = document.id.clone();
    let open_id = runtime.operation();
    let open_outcome = runtime.observe_operation(open_id);
    let open_event = if runtime.model().lifecycle == Lifecycle::RecoveryRequired {
        Event::RecoverWithDocument {
            operation_id: open_id,
            document,
        }
    } else {
        Event::Open {
            operation_id: open_id,
            document,
        }
    };
    runtime.submit(open_event);
    let accepted = match wait_for_matrix_project(
        &runtime,
        open_outcome,
        &variant_id,
        variant_session_epoch,
        &alive,
    )
    .await
    {
        Ok(accepted) => accepted,
        Err(error) => {
            if let (Some(active), Some(scope)) = (runtime.model().accepted, runtime.scope())
                && active.session_epoch == variant_session_epoch
                && active.document.id == variant_id
                && scope.session_epoch == variant_session_epoch
                && scope.document_id == variant_id
            {
                *report_scope.borrow_mut() = Some(VariantReportOwner {
                    scope,
                    token: active.token,
                    revision: active.document.revision,
                });
            }
            let recovery = if alive.get() {
                restore_source_after_variant_failure(
                    &runtime,
                    request,
                    &variant_id,
                    variant_session_epoch,
                    None,
                    &report_scope,
                )
                .await
            } else {
                Ok(false)
            };
            return Err(match recovery {
                Ok(true) => format!(
                    "Could not open the matrix design variant; reopened the saved original. {error}"
                ),
                Ok(false) => format!("Could not open the matrix design variant. {error}"),
                Err(recovery_error) => format!(
                    "Could not open the matrix design variant ({error}); could not confirm reopening the saved original ({recovery_error})."
                ),
            });
        }
    };
    if !alive.get() {
        return Err("The matrix design variant owner closed.".into());
    }
    let accepted = match navigate_variant_to_source_context(
        &runtime,
        accepted,
        &variant_id,
        &request.owner.scope,
        &alive,
    )
    .await
    {
        Ok(accepted) => accepted,
        Err(error) => {
            let recovery = if alive.get() {
                restore_source_after_variant_failure(
                    &runtime,
                    request,
                    &variant_id,
                    variant_session_epoch,
                    None,
                    &report_scope,
                )
                .await
            } else {
                Ok(false)
            };
            return Err(match recovery {
                Ok(true) => format!(
                    "Could not select the original board context; reopened the saved original. {error}"
                ),
                Ok(false) => format!("Could not select the original board context. {error}"),
                Err(recovery_error) => format!(
                    "Could not select the original board context ({error}); could not confirm reopening the saved original ({recovery_error})."
                ),
            });
        }
    };
    let clone_scope = runtime
        .scope()
        .ok_or_else(|| "The duplicated project has no active scope.".to_owned())?;
    *report_scope.borrow_mut() = Some(VariantReportOwner {
        scope: clone_scope,
        token: accepted.token,
        revision: accepted.document.revision,
    });
    let reversible = is_reversible(&accepted.document);
    let result = async {
        let accepted =
            match current_variant_snapshot(&runtime, &accepted, &variant_id, &request.owner.scope)?
            {
                Some(accepted) => accepted,
                None => return Ok(VariantEditWait::Retired),
            };
        let Some(matrix) = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == request.owner.matrix_id)
            .cloned()
        else {
            return Ok(VariantEditWait::Retired);
        };
        let prepared = prepare_preset_matrix(
            &accepted.document,
            &matrix,
            &request.owner.scope.board_id,
            request.preset,
            request.orientation,
            reversible,
        )
        .await?;
        if !alive.get() {
            return Ok(VariantEditWait::Retired);
        }
        // Preparing the catalogue can take long enough for the user to open another project.
        // Confirm the clone is still the active project before submitting any intent.
        match current_variant_snapshot(&runtime, &accepted, &variant_id, &request.owner.scope)? {
            Some(_) => {}
            None => return Ok(VariantEditWait::Retired),
        }
        wait_for_variant_edit(
            &runtime,
            &accepted,
            &variant_id,
            &request.owner.scope,
            &alive,
            matrix_preset_resolver(matrix.id.clone(), prepared),
        )
        .await
    }
    .await;
    if matches!(&result, Ok(VariantEditWait::Retired)) {
        return Ok(VariantSetupResult::Retired);
    }
    if let Err(error) = result {
        let recovery = if alive.get() {
            restore_source_after_variant_failure(
                &runtime,
                request,
                &variant_id,
                variant_session_epoch,
                Some(&accepted),
                &report_scope,
            )
            .await
        } else {
            Ok(false)
        };
        return Err(match recovery {
            Ok(true) => format!(
                "Could not prepare the matrix design variant; reopened the saved original. {error}"
            ),
            Ok(false) => format!("Could not prepare the matrix design variant. {error}"),
            Err(recovery_error) => format!(
                "Could not prepare the matrix design variant ({error}); could not confirm reopening the saved original ({recovery_error})."
            ),
        });
    }
    // Confirm the clone remains the active project after its matrix edit was durably saved.
    if runtime
        .model()
        .accepted
        .is_none_or(|active| active.document.id != variant_id)
    {
        return Ok(VariantSetupResult::Retired);
    }
    Ok(VariantSetupResult::Completed)
}
