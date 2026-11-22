//! Matrix assembly presets: the preset and orientation choices, applying one to a matrix, and
//! duplicating the whole design as a variant that carries a different preset.
use super::matrix_preset_logic::preset_variant;
pub use super::matrix_preset_logic::{
    MatrixPreset, SwitchOrientation, is_reversible, matrix_preset, matrix_with_preset,
    missing_definitions, switch_choices,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Event, Lifecycle, Resolution, Scope, TerminalOutcome,
};
use boardstudio_model::model::{EditOperation, Matrix, PartDefinition, ProjectDoc};
use std::rc::Rc;

struct Prepared {
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
    variant: String,
    orientation: SwitchOrientation,
}

async fn prepare(
    document: &ProjectDoc,
    matrix: &Matrix,
    board_id: &str,
    preset: MatrixPreset,
    orientation: SwitchOrientation,
) -> Result<Prepared, String> {
    let reversible = is_reversible(document);
    let templates = crate::presentation::parts::load_matrix_templates(reversible).await?;
    let mut prepared = crate::matrix_setup_operation::prepare_matrix(
        matrix.id.clone(),
        board_id.to_owned(),
        crate::matrix_setup_operation::MatrixSetupRequest {
            rows: matrix.rows,
            columns: matrix.columns,
            preset: preset.setup(),
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
    Ok(Prepared {
        matrix: prepared.matrix,
        definitions: prepared.definitions,
        variant: preset_variant(preset, orientation, reversible),
        orientation,
    })
}

/// Apply `preset` to the matrix, keeping its size, enabled keys, local poses and extra components.
pub async fn apply_preset(
    runtime: &Rc<Runtime>,
    matrix_id: &str,
    board_id: &str,
    preset: MatrixPreset,
    orientation: SwitchOrientation,
) -> Result<(), String> {
    let snapshot = runtime.model().accepted.ok_or("No accepted keyboard.")?;
    let matrix = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("The selected matrix no longer exists.")?
        .clone();
    let prepared = prepare(&snapshot.document, &matrix, board_id, preset, orientation).await?;
    let id = matrix_id.to_owned();
    runtime
        .commit(
            "layout-matrix-preset",
            EditResolver::new("layout-matrix-preset", move |accepted: &AcceptedSnapshot| {
                let Some(matrix) = accepted.document.matrices.iter().find(|matrix| matrix.id == id) else {
                    return Resolution::Retire("The selected matrix no longer exists.".into());
                };
                if (matrix.rows, matrix.columns) != (prepared.matrix.rows, prepared.matrix.columns) {
                    return Resolution::Retire("The matrix size changed before the preset was applied. Apply the preset again.".into());
                }
                let replacement = matrix_with_preset(matrix, &prepared.matrix, &prepared.variant, prepared.orientation);
                if &replacement == matrix {
                    return Resolution::Unchanged;
                }
                Resolution::submit(vec![id.clone()], EditOperation::SetMatrix { matrix: replacement, definitions: Some(prepared.definitions.clone()) })
            }),
        )
        .map(|_| ())
}

/// Poll an observed operation until it settles `Completed` with a saved accepted document `id`.
async fn wait_saved(
    runtime: &Rc<Runtime>,
    outcome: crate::operation_outcomes::OutcomeSlot,
    id: &str,
) -> Result<AcceptedSnapshot, String> {
    for _ in 0..1_200 {
        if let Some(outcome) = outcome.borrow_mut().take() {
            if outcome != TerminalOutcome::Completed {
                return Err(format!(
                    "The project operation did not complete: {outcome:?}"
                ));
            }
            let model = runtime.model();
            let accepted = model
                .accepted
                .ok_or("The project operation was not accepted.")?;
            if accepted.document.id != id || model.lifecycle != Lifecycle::Ready {
                return Err("The project operation was superseded by another open action.".into());
            }
            return Ok(accepted);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The project operation did not settle.".into())
}

/// Clone the whole accepted project into a new document, open it, and apply `preset` to the copy.
/// A failure after the copy opened reopens the saved original so the user returns to their design.
pub async fn duplicate_variant(
    runtime: Rc<Runtime>,
    source: &Scope,
    matrix_id: &str,
    preset: MatrixPreset,
    orientation: SwitchOrientation,
) -> Result<(), String> {
    let mut document = (*runtime
        .model()
        .accepted
        .ok_or("No accepted keyboard.")?
        .document)
        .clone();
    let source_id = document.id.clone();
    document.id = crate::runtime::new_project_id()?;
    let name = if document.name.trim().is_empty() {
        "Untitled keyboard"
    } else {
        document.name.trim()
    }
    .to_owned();
    document.name = format!("{name} (variant)");
    let variant_id = document.id.clone();
    let open_id = runtime.operation();
    let opened = runtime.observe_operation(open_id);
    runtime.submit(
        if runtime.model().lifecycle == Lifecycle::RecoveryRequired {
            Event::RecoverWithDocument {
                operation_id: open_id,
                document,
            }
        } else {
            Event::Open {
                operation_id: open_id,
                document,
            }
        },
    );
    let work = async {
        wait_saved(&runtime, opened, &variant_id).await?;
        let navigate_id = runtime.operation();
        let navigated = runtime.observe_operation(navigate_id);
        runtime.submit(Event::Navigate {
            operation_id: navigate_id,
            board_id: source.board_id.clone(),
            instance_id: source.instance_id.clone(),
        });
        wait_saved(&runtime, navigated, &variant_id).await?;
        apply_preset(&runtime, matrix_id, &source.board_id, preset, orientation).await
    };
    let Err(error) = work.await else {
        return Ok(());
    };
    let model = runtime.model();
    let restored = match (model.accepted, runtime.scope()) {
        (Some(current), Some(scope)) => runtime
            .reopen_saved_if_current(source_id, &current, model.lifecycle, &scope)
            .await
            .is_ok(),
        _ => false,
    };
    Err(if restored {
        format!("Could not create the matrix design variant; reopened the saved original. {error}")
    } else {
        format!("Could not create the matrix design variant. {error}")
    })
}
