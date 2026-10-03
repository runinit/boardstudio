//! Editor-lifetime owner for placing a single Parts-library matrix assembly.
use super::matrix_setup::{
    MatrixPlacementInput, MatrixPlacementMount, MatrixPlacementMove, MatrixPlacementOwner,
    MatrixPlacementProjection,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{
    matrix_setup_operation::{MatrixSetupRequest, next_matrix_id, prepare_matrix},
    operation_outcomes::OutcomeSlot,
    presentation::canvas_interaction::CanvasInteractionOwner,
    runtime::Runtime,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, TerminalOutcome,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct PendingPlacement {
    owner: MatrixPlacementOwner,
    outcome: OutcomeSlot,
    matrix_id: String,
    selected_context: Option<ScopedTreeContext>,
    selected_part_ids: Vec<String>,
}

pub(in crate::presentation) fn use_matrix_placement(
    runtime: Rc<Runtime>,
    input: MatrixPlacementInput,
) -> MatrixPlacementMount {
    let MatrixPlacementInput {
        version,
        selected_context,
        anchor_scope,
        workspace,
        scope_generation,
        assembly_orientation,
        canvas_interaction,
    } = input;
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        let canvas_interaction = canvas_interaction.clone();
        move || {
            alive.set(false);
            canvas_interaction.release(CanvasInteractionOwner::MatrixPlacement);
        }
    });
    let request_id = use_signal(|| 0u64);
    let preparing = use_signal(|| None::<MatrixPlacementOwner>);
    let placement = use_signal(|| None::<MatrixPlacementProjection>);
    let pending = use_signal(|| None::<PendingPlacement>);
    let error = use_signal(|| None::<String>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let canvas_interaction = canvas_interaction.clone();
            let mut preparing = preparing;
            let mut placement = placement;
            let mut pending = pending;
            let mut error = error;
            let mut selected_context = selected_context;
            let mut anchor_scope = anchor_scope;
            move |(_, current_workspace, current_generation)| {
                let waiting = pending.read().clone();
                if let Some(waiting) = waiting {
                    let Some(outcome) = waiting.outcome.borrow().clone() else {
                        return;
                    };
                    if !same_session_scope(&runtime, &waiting.owner) {
                        pending.set(None);
                        error.set(None);
                        return;
                    }
                    match outcome {
                        TerminalOutcome::Completed => {
                            let model = runtime.model();
                            let Some(snapshot) = model.accepted.as_ref() else {
                                return;
                            };
                            if snapshot.token == waiting.owner.snapshot_token
                                || snapshot.document.revision <= waiting.owner.revision
                                || model.lifecycle != Lifecycle::Ready
                                || model.durability
                                    != (Durability::Saved {
                                        revision: snapshot.document.revision,
                                    })
                            {
                                return;
                            }
                            let created = snapshot
                                .document
                                .matrices
                                .iter()
                                .any(|matrix| matrix.id == waiting.matrix_id);
                            pending.set(None);
                            let selection_is_current = model.selected_part_ids
                                == waiting.selected_part_ids
                                && selected_context.peek().as_ref()
                                    == waiting.selected_context.as_ref();
                            if !created
                                || current_workspace != "Layout"
                                || current_generation != waiting.owner.scope_generation
                                || !selection_is_current
                            {
                                error.set(None);
                                return;
                            }
                            let context = TreeContext::Matrix {
                                matrix_id: waiting.matrix_id,
                            };
                            let Some(part_ids) = super::resolve_selection(&model, &context) else {
                                error.set(Some("The matrix was saved, but its Layout selection could not be restored.".into()));
                                return;
                            };
                            selected_context.set(Some(ScopedTreeContext {
                                scope: waiting.owner.scope.clone(),
                                context,
                            }));
                            anchor_scope.set(None);
                            runtime.submit(Event::SelectParts {
                                operation_id: runtime.operation(),
                                part_ids,
                                range_part_ids: Vec::new(),
                                mode: boardstudio_application::SelectionMode::Replace,
                            });
                            error.set(None);
                        }
                        TerminalOutcome::Rejected(message)
                        | TerminalOutcome::PersistenceFailed(message)
                        | TerminalOutcome::BlockedByRecovery(message)
                        | TerminalOutcome::ExecutorFailed(message) => {
                            pending.set(None);
                            if current_workspace == "Layout"
                                && current_generation == waiting.owner.scope_generation
                                && placement_selection_is_current(
                                    &runtime,
                                    &selected_context,
                                    &waiting,
                                )
                            {
                                error.set(Some(message));
                            } else {
                                error.set(None);
                            }
                        }
                        TerminalOutcome::Superseded
                        | TerminalOutcome::Cancelled
                        | TerminalOutcome::Closed => {
                            pending.set(None);
                            if current_workspace == "Layout"
                                && current_generation == waiting.owner.scope_generation
                                && placement_selection_is_current(
                                    &runtime,
                                    &selected_context,
                                    &waiting,
                                )
                            {
                                error.set(Some(
                                    "Matrix placement was superseded before it was saved.".into(),
                                ));
                            } else {
                                error.set(None);
                            }
                        }
                    }
                }
                let active_owner = preparing
                    .read()
                    .clone()
                    .or_else(|| placement.read().as_ref().map(|active| active.owner.clone()));
                if pending.read().is_none()
                    && let Some(owner) = active_owner
                    && (current_workspace != "Layout"
                        || current_generation != owner.scope_generation
                        || !same_session_scope(&runtime, &owner)
                        || !same_accepted_source(&runtime, &owner))
                {
                    preparing.set(None);
                    placement.set(None);
                    error.set(None);
                    canvas_interaction.release(CanvasInteractionOwner::MatrixPlacement);
                }
            }
        },
    ));

    let on_place = use_callback({
        let runtime = runtime.clone();
        let alive = alive.clone();
        let canvas_interaction = canvas_interaction.clone();
        let mut request_id = request_id;
        let mut preparing = preparing;
        let mut error = error;
        let assembly_orientation = assembly_orientation.0;
        move |preset: crate::presentation::parts::MatrixPresetId| {
            if pending.read().is_some() || preparing.read().is_some() || placement.read().is_some()
            {
                return;
            }
            let Some((snapshot, scope, board_id)) = placement_source(&runtime) else {
                error.set(Some(
                    "Open a saved, editable board before placing a key assembly.".into(),
                ));
                return;
            };
            if !canvas_interaction.try_acquire(CanvasInteractionOwner::MatrixPlacement) {
                return;
            }
            let next = request_id()
                .checked_add(1)
                .expect("matrix placement identity exhausted");
            request_id.set(next);
            let owner = MatrixPlacementOwner {
                editor_instance_id,
                request_id: next,
                scope_generation: scope_generation(),
                scope,
                board_id,
                snapshot_token: snapshot.token,
                revision: snapshot.document.revision,
            };
            let accepted = snapshot.clone();
            let reversible = accepted
                .document
                .parameters
                .get("reversibleLayout")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let preset = crate::presentation::parts::matrix_setup_preset(preset);
            let orientation = match assembly_orientation() {
                crate::presentation::parts::SwitchOrientation::South => {
                    super::MatrixSwitchOrientation::South
                }
                crate::presentation::parts::SwitchOrientation::North => {
                    super::MatrixSwitchOrientation::North
                }
            };
            let matrix_id = next_matrix_id(
                &accepted.document.matrices,
                &accepted.document.definitions,
                &accepted.document.parts,
                preset.as_str(),
                reversible,
            );
            preparing.set(Some(owner.clone()));
            error.set(None);
            let runtime = runtime.clone();
            let alive = alive.clone();
            let mut preparing = preparing;
            let mut placement = placement;
            let mut error = error;
            let canvas_interaction = canvas_interaction.clone();
            spawn_local(async move {
                let result = async {
                    let templates =
                        crate::presentation::parts::load_matrix_templates(reversible).await?;
                    let mut prepared = prepare_matrix(
                        matrix_id.clone(),
                        owner.board_id.clone(),
                        MatrixSetupRequest {
                            rows: 1,
                            columns: 1,
                            preset,
                        },
                        reversible,
                        &templates,
                    )?;
                    for definition in &mut prepared.definitions {
                        *definition = crate::presentation::parts::normalize_matrix_definition(
                            definition.clone(),
                        )
                        .await?;
                    }
                    let orientation_name = match orientation {
                        super::MatrixSwitchOrientation::South => "south",
                        super::MatrixSwitchOrientation::North => "north",
                    };
                    let variant = format!(
                        "preset/{}/{}{}",
                        preset.as_str(),
                        if reversible { "reversible/" } else { "" },
                        orientation_name,
                    );
                    let mut matrix = crate::presentation::objects::matrix_with_preset(
                        &prepared.matrix,
                        &prepared.matrix,
                        &variant,
                        orientation,
                    );
                    matrix.origin = boardstudio_core::model::Vec2::default();
                    let mut scenes = runtime
                        .project_matrices(accepted.document.revision, vec![matrix.clone()])
                        .await?;
                    if scenes.len() != 1 {
                        return Err("Core returned an incomplete matrix placement preview.".into());
                    }
                    Ok::<_, String>((matrix, prepared.definitions, scenes.remove(0)))
                }
                .await;
                if !alive.get() {
                    return;
                }
                if preparing.read().as_ref() != Some(&owner) {
                    return;
                }
                if workspace() != "Layout"
                    || scope_generation() != owner.scope_generation
                    || !same_session_scope(&runtime, &owner)
                    || !same_accepted_source(&runtime, &owner)
                {
                    preparing.set(None);
                    canvas_interaction.release(CanvasInteractionOwner::MatrixPlacement);
                    return;
                }
                match result {
                    Ok((matrix, definitions, scene)) => {
                        placement.set(Some(MatrixPlacementProjection {
                            owner: owner.clone(),
                            matrix,
                            scene,
                            definitions,
                        }));
                        preparing.set(None);
                    }
                    Err(message) => {
                        preparing.set(None);
                        error.set(Some(message));
                        canvas_interaction.release(CanvasInteractionOwner::MatrixPlacement);
                    }
                }
            });
        }
    });

    let on_move = use_callback({
        let runtime = runtime.clone();
        let canvas_interaction = canvas_interaction.clone();
        let mut placement = placement;
        move |movement: MatrixPlacementMove| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement)
                || workspace() != "Layout"
                || scope_generation() != movement.owner.scope_generation
                || !movement.center.x.is_finite()
                || !movement.center.y.is_finite()
                || !same_session_scope(&runtime, &movement.owner)
                || !same_accepted_source(&runtime, &movement.owner)
            {
                return;
            }
            let active = placement.read().clone();
            if let Some(mut active) = active.filter(|active| active.owner == movement.owner) {
                active.matrix.origin = movement.center;
                placement.set(Some(active));
            }
        }
    });

    let on_cancel = use_callback({
        let mut preparing = preparing;
        let mut placement = placement;
        let mut error = error;
        let canvas_interaction = canvas_interaction.clone();
        move |owner: MatrixPlacementOwner| {
            if pending.read().is_some()
                || !canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement)
            {
                return;
            }
            if preparing.read().as_ref() == Some(&owner)
                || placement
                    .read()
                    .as_ref()
                    .is_some_and(|active| active.owner == owner)
            {
                preparing.set(None);
                placement.set(None);
                error.set(None);
                canvas_interaction.release(CanvasInteractionOwner::MatrixPlacement);
            }
        }
    });

    let on_commit = use_callback({
        let runtime = runtime.clone();
        let canvas_interaction = canvas_interaction.clone();
        let mut placement = placement;
        let mut pending = pending;
        let mut error = error;
        move |movement: MatrixPlacementMove| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement)
                || pending.read().is_some()
                || workspace() != "Layout"
                || scope_generation() != movement.owner.scope_generation
                || !movement.center.x.is_finite()
                || !movement.center.y.is_finite()
                || !same_session_scope(&runtime, &movement.owner)
                || !same_accepted_source(&runtime, &movement.owner)
            {
                return;
            }
            let Some(mut active) = placement
                .read()
                .clone()
                .filter(|active| active.owner == movement.owner)
            else {
                return;
            };
            active.matrix.origin = movement.center;
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: movement.owner.revision,
                    })
            {
                return;
            }
            let required = active
                .definitions
                .into_iter()
                .filter(|definition| {
                    !snapshot
                        .document
                        .definitions
                        .iter()
                        .any(|existing| existing.id == definition.id)
                })
                .collect();
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let model = runtime.model();
            pending.set(Some(PendingPlacement {
                owner: movement.owner.clone(),
                outcome,
                matrix_id: active.matrix.id.clone(),
                selected_context: selected_context.peek().clone(),
                selected_part_ids: model.selected_part_ids.clone(),
            }));
            placement.set(None);
            error.set(None);
            canvas_interaction.release(CanvasInteractionOwner::MatrixPlacement);
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: movement.owner.revision,
                    transaction_id: format!(
                        "matrix-placement-{}-{}-{}",
                        editor_instance_id, movement.owner.request_id, operation_id.0
                    ),
                    phase: EditPhase::Commit,
                    target_ids: vec![active.matrix.id.clone()],
                    operation: EditOperation::SetMatrix {
                        matrix: active.matrix,
                        definitions: Some(required),
                    },
                },
            });
        }
    });

    MatrixPlacementMount {
        cancel_owner: preparing
            .read()
            .clone()
            .or_else(|| placement.read().as_ref().map(|active| active.owner.clone())),
        placement: placement.read().clone(),
        busy: preparing.read().is_some() || pending.read().is_some(),
        error: error.read().clone(),
        on_place,
        on_move,
        on_commit,
        on_cancel,
    }
}

fn placement_source(runtime: &Runtime) -> Option<(AcceptedSnapshot, Scope, String)> {
    let model = runtime.model();
    let scope = runtime.scope()?;
    let snapshot = model.accepted.as_ref()?;
    if model.lifecycle != Lifecycle::Ready
        || model.durability
            != (Durability::Saved {
                revision: snapshot.document.revision,
            })
        || model.display_preview.is_some()
        || model.gesture.is_some()
        || snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return None;
    }
    Some((snapshot.clone(), scope.clone(), scope.board_id))
}

fn same_session_scope(runtime: &Runtime, owner: &MatrixPlacementOwner) -> bool {
    let model = runtime.model();
    runtime.scope().as_ref() == Some(&owner.scope)
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.document.id == owner.scope.document_id
                && snapshot.session_epoch == owner.scope.session_epoch
        })
}

fn same_accepted_source(runtime: &Runtime, owner: &MatrixPlacementOwner) -> bool {
    let model = runtime.model();
    model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: owner.revision,
            })
        && model.active_board_id == owner.board_id
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.token == owner.snapshot_token && snapshot.document.revision == owner.revision
        })
}

fn placement_selection_is_current(
    runtime: &Runtime,
    selected_context: &Signal<Option<ScopedTreeContext>>,
    pending: &PendingPlacement,
) -> bool {
    runtime.model().selected_part_ids == pending.selected_part_ids
        && selected_context.peek().as_ref() == pending.selected_context.as_ref()
}
