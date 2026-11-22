//! Field and action edit plumbing for the Matrix Inspector: resolvers, feedback, geometry.
use super::*;

use super::projection::preset_variant;
use super::variant::{
    current_variant_snapshot, matrix_setup_preset, navigate_variant_to_source_context,
    restore_source_after_variant_failure, wait_for_matrix_project, wait_for_variant_edit,
};

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

pub(super) use super::super::matrix_inspector_logic::{build_operation, field_value};

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
