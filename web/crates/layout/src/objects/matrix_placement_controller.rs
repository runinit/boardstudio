//! Editor-lifetime owner for placing a single Parts-library matrix assembly.
use super::matrix_setup::{
    MatrixPlacementInput, MatrixPlacementMount, MatrixPlacementMove, MatrixPlacementOwner,
    MatrixPlacementProjection,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{
    matrix_setup_operation::{MatrixSetupRequest, next_matrix_id, prepare_matrix},
    presentation::canvas_interaction::CanvasInteractionOwner,
    runtime::Runtime,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{EditOperation, Matrix, PartDefinition};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct PlacementFollowUp {
    owner: MatrixPlacementOwner,
    matrix_id: String,
    selected_context: Option<ScopedTreeContext>,
    selected_part_ids: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlacementAction {
    Place,
}

/// Resolve placing a prepared matrix against the accepted document at execution time. Only
/// the definitions the accepted document does not yet have are sent, so a placement queued
/// behind an edit that already added one still lands.
fn place_matrix_resolver(matrix: Matrix, definitions: Vec<PartDefinition>) -> EditResolver {
    EditResolver::new(
        "layout-matrix-placement",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if matrix
                .board_id
                .as_deref()
                .is_some_and(|board_id| !document.boards.iter().any(|board| board.id == board_id))
            {
                return Resolution::Retire("The board no longer exists.".into());
            }
            if document
                .matrices
                .iter()
                .any(|existing| existing.id == matrix.id)
            {
                return Resolution::Retire("The matrix identity is already in use.".into());
            }
            let required = definitions
                .iter()
                .filter(|definition| {
                    !document
                        .definitions
                        .iter()
                        .any(|existing| existing.id == definition.id)
                })
                .cloned()
                .collect();
            Resolution::submit(
                vec![matrix.id.clone()],
                EditOperation::SetMatrix {
                    matrix: matrix.clone(),
                    definitions: Some(required),
                },
            )
        },
    )
}

pub fn use_matrix_placement(
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
    let pending = use_hook(|| PendingEditSignals::<PlacementAction>::new());
    let pending_place = use_signal(|| false);
    pending.bind_one_shot(PlacementAction::Place, pending_place);
    let pending_follow_up = use_signal(|| None::<PlacementFollowUp>);
    let error = use_signal(|| None::<String>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let canvas_interaction = canvas_interaction.clone();
            let mut preparing = preparing;
            let mut placement = placement;
            let pending = pending.clone();
            let mut pending_follow_up = pending_follow_up;
            let mut error = error;
            let mut selected_context = selected_context;
            let mut anchor_scope = anchor_scope;
            move |(_, current_workspace, current_generation)| {
                let waiting = pending_follow_up.read().clone();
                if let Some(waiting) = waiting {
                    let owner_is_live = same_session_scope(&runtime, &waiting.owner);
                    let visible = current_workspace == "Layout"
                        && current_generation == waiting.owner.scope_generation
                        && runtime.model().selected_part_ids == waiting.selected_part_ids
                        && selected_context.peek().as_ref() == waiting.selected_context.as_ref();
                    let results = pending.settle(owner_is_live, |_| String::new());
                    if results.is_empty() {
                        return;
                    }
                    for result in results {
                        pending_follow_up.set(None);
                        match result {
                            PendingEditResult::Retired { .. } => error.set(None),
                            PendingEditResult::Landed { .. } => {
                                let model = runtime.model();
                                let created = model.accepted.as_ref().is_some_and(|snapshot| {
                                    snapshot
                                        .document
                                        .matrices
                                        .iter()
                                        .any(|matrix| matrix.id == waiting.matrix_id)
                                });
                                if !created || !visible {
                                    error.set(None);
                                } else {
                                    let context = TreeContext::Matrix {
                                        matrix_id: waiting.matrix_id.clone(),
                                    };
                                    match super::resolve_selection(&model, &context) {
                                    None => error.set(Some("The matrix was saved, but its Layout selection could not be restored.".into())),
                                    Some(part_ids) => {
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
                                }
                                }
                            }
                            PendingEditResult::Failed { message, .. } => {
                                error.set(visible.then_some(message));
                            }
                        }
                    }
                }
                let active_owner = preparing
                    .read()
                    .clone()
                    .or_else(|| placement.read().as_ref().map(|active| active.owner.clone()));
                if pending_follow_up.read().is_none()
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
        move |source: super::MatrixPlacementSource| {
            if pending_place() || preparing.read().is_some() || placement.read().is_some() {
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
            let matrix_key = match &source {
                super::MatrixPlacementSource::Preset(preset) => {
                    crate::presentation::parts::matrix_setup_preset(*preset)
                        .as_str()
                        .to_owned()
                }
                super::MatrixPlacementSource::Assembly { assembly, .. } => {
                    format!("saved-assembly-{}", assembly.id)
                }
            };
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
                &matrix_key,
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
                    let (mut matrix, definitions) = match source {
                        super::MatrixPlacementSource::Preset(preset_id) => {
                            let preset = crate::presentation::parts::matrix_setup_preset(preset_id);
                            let templates =
                                crate::presentation::parts::load_matrix_templates(reversible)
                                    .await?;
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
                                *definition =
                                    crate::presentation::parts::normalize_matrix_definition(
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
                            let matrix = crate::presentation::objects::matrix_with_preset(
                                &prepared.matrix,
                                &prepared.matrix,
                                &variant,
                                orientation,
                            );
                            (matrix, prepared.definitions)
                        }
                        super::MatrixPlacementSource::Assembly {
                            assembly,
                            definitions: sources,
                        } => {
                            let seed = assembly_matrix_seed(
                                matrix_id.clone(),
                                owner.board_id.clone(),
                                assembly.name.clone(),
                            );
                            let (matrix, mut definitions) =
                                crate::presentation::parts::matrix_with_assembly(
                                    &seed,
                                    &assembly,
                                    &sources,
                                    &accepted.document,
                                    &format!("placement-{}", owner.request_id),
                                )?;
                            for definition in &mut definitions {
                                *definition =
                                    crate::presentation::parts::normalize_matrix_definition(
                                        definition.clone(),
                                    )
                                    .await?;
                            }
                            (matrix, definitions)
                        }
                    };
                    matrix.origin = boardstudio_core::model::Vec2::default();
                    let mut scenes = runtime
                        .project_matrices(accepted.document.revision, vec![matrix.clone()])
                        .await?;
                    if scenes.len() != 1 {
                        return Err("Core returned an incomplete matrix placement preview.".into());
                    }
                    Ok::<_, String>((matrix, definitions, scenes.remove(0)))
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
            if pending_place()
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
        let pending = pending.clone();
        let mut pending_follow_up = pending_follow_up;
        let mut error = error;
        move |movement: MatrixPlacementMove| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement)
                || pending_place()
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
            if model.accepted.is_none()
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: movement.owner.revision,
                    })
            {
                return;
            }
            let matrix_id = active.matrix.id.clone();
            let follow_up = PlacementFollowUp {
                owner: movement.owner.clone(),
                matrix_id,
                selected_context: selected_context.peek().clone(),
                selected_part_ids: model.selected_part_ids.clone(),
            };
            pending_follow_up.set(Some(follow_up));
            pending.begin_one_shot(
                &runtime,
                PlacementAction::Place,
                "layout-matrix-placement",
                Some("matrix placement".into()),
                place_matrix_resolver(active.matrix, active.definitions),
            );
            placement.set(None);
            error.set(None);
            canvas_interaction.release(CanvasInteractionOwner::MatrixPlacement);
        }
    });

    MatrixPlacementMount {
        cancel_owner: preparing
            .read()
            .clone()
            .or_else(|| placement.read().as_ref().map(|active| active.owner.clone())),
        placement: placement.read().clone(),
        busy: preparing.read().is_some() || pending_place(),
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

fn assembly_matrix_seed(
    id: String,
    board_id: String,
    name: String,
) -> boardstudio_core::model::Matrix {
    use boardstudio_core::model::{Matrix, MatrixCell, Vec2};
    Matrix {
        id,
        name: Some(name),
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.05, y: 19.05 },
        origin: Vec2::default(),
        definition_id: String::new(),
        part_ids: Vec::new(),
        board_id: Some(board_id),
        mirror: None,
        rotation: None,
        edge_gap: Some(Vec2 { x: 1.0, y: 1.0 }),
        diode_direction: None,
        row_offsets: Vec::new(),
        column_offsets: Vec::new(),
        column_staggers: Vec::new(),
        column_splays: Vec::new(),
        column_origins: Vec::new(),
        cells: vec![MatrixCell {
            row: 0,
            column: 0,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: Some(0.0),
            assemblies: Vec::new(),
            assemblies_local: Some(true),
        }],
    }
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
