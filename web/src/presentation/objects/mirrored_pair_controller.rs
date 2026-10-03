//! Editor-lifetime owner for mirrored-pair preparation, placement, and exact saved acknowledgement.
use super::mirrored_pair::{
    MirroredPairCreated, MirroredPairFormProjection, MirroredPairFormValues, MirroredPairMount,
    MirroredPairOwner, MirroredPairPlacement, MirroredPairRequest,
};
use crate::{
    matrix_setup_operation::{MatrixSetupRequest, next_matrix_id, prepare_matrix},
    mirrored_pair_geometry::{MirroredPairGeometryInput, MirroredPairIds, project_mirrored_pair},
    mirrored_pair_lifecycle::{PairFormStage, PairFormState, PairResultGuard},
    operation_outcomes::OutcomeSlot,
    presentation::canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner},
    runtime::Runtime,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, Scope, TerminalOutcome,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, Layout};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct PendingPair {
    owner: MirroredPairOwner,
    operation_id: OperationId,
    outcome: OutcomeSlot,
    left_matrix_id: String,
    right_matrix_id: String,
    left_layout: Layout,
    right_layout: Layout,
    transaction_id: String,
}

#[derive(Clone)]
struct ActivePair {
    owner: MirroredPairOwner,
    request: MirroredPairRequest,
    matrix: boardstudio_core::model::Matrix,
    definitions: Vec<boardstudio_core::model::PartDefinition>,
    ids: MirroredPairIds,
    placement: MirroredPairPlacement,
}

/// Called once during the Editor lifetime. The parent mounts its form and forwards the typed
/// world-coordinate placement callbacks from the existing SVG event owner.
pub(in crate::presentation) fn use_mirrored_pair(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    on_created: EventHandler<MirroredPairCreated>,
    canvas_interaction: CanvasInteractionArbiter,
) -> MirroredPairMount {
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
    let open = use_signal(|| None::<MirroredPairOwner>);
    let form_state = use_signal(|| PairFormState::new(MirroredPairFormValues::default()));
    let preparing = use_signal(|| None::<MirroredPairOwner>);
    let error = use_signal(|| None::<String>);
    let status = use_signal(|| None::<String>);
    let placement = use_signal(|| None::<ActivePair>);
    let pending = use_signal(|| None::<PendingPair>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut form_state = form_state;
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            let mut placement = placement;
            let mut pending = pending;
            move |(_, workspace, scope_generation)| {
                settle_pending(
                    &runtime,
                    workspace,
                    scope_generation,
                    &mut PairSettlement {
                        pending: &mut pending,
                        open: &mut open,
                        form_state: &mut form_state,
                        error: &mut error,
                        status: &mut status,
                        placement: &mut placement,
                        on_created,
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
                    placement.set(None);
                    error.set(None);
                    status.set(None);
                    form_state.set(PairFormState::new(MirroredPairFormValues::default()));
                }
                if pending.read().is_none()
                    && open.read().as_ref().is_some_and(|owner| {
                        let model = runtime.model();
                        model.accepted.as_ref().is_none_or(|snapshot| {
                            snapshot.token != owner.snapshot_token
                                || snapshot.document.revision != owner.revision
                                || snapshot.document.id != owner.scope.document_id
                                || snapshot.session_epoch != owner.scope.session_epoch
                        })
                    })
                {
                    preparing.set(None);
                    placement.set(None);
                    form_state.write().stage = PairFormStage::Setup;
                    error.set(Some("The accepted board changed. Cancel this setup and reopen it before placing a pair.".into()));
                    status.set(None);
                }
            }
        },
    ));

    let on_open = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut open_id = open_id;
        let mut form_state = form_state;
        let mut error = error;
        let mut status = status;
        let canvas_interaction = canvas_interaction.clone();
        move |_| {
            if pending.read().is_some()
                || preparing.read().is_some()
                || placement.read().is_some()
                || open.read().is_some()
            {
                return;
            }
            let current_workspace = workspace();
            let current_generation = scope_generation();
            let model = runtime.model();
            let current_scope = runtime.scope();
            let Some((snapshot, scope, board_id)) =
                pair_source(&runtime, &model, current_scope.as_ref(), current_workspace)
            else {
                error.set(Some(
                    "Open a saved, editable board in Layout before creating a mirrored pair."
                        .into(),
                ));
                return;
            };
            if !canvas_interaction.try_acquire(CanvasInteractionOwner::MirroredPair) {
                return;
            }
            let next = open_id()
                .checked_add(1)
                .expect("mirrored pair identity exhausted");
            open_id.set(next);
            form_state.set(PairFormState::new(MirroredPairFormValues::default()));
            error.set(None);
            status.set(None);
            open.set(Some(MirroredPairOwner {
                editor_instance_id,
                open_id: next,
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
        let mut form_state = form_state;
        let mut preparing = preparing;
        let mut placement = placement;
        let mut error = error;
        let mut status = status;
        let canvas_interaction = canvas_interaction.clone();
        move |owner: MirroredPairOwner| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
                return;
            }
            let active = open.read().as_ref() == Some(&owner)
                || placement
                    .read()
                    .as_ref()
                    .is_some_and(|pair| pair.owner == owner);
            if crate::mirrored_pair_lifecycle::pair_cancel_is_allowed(
                active,
                pending.read().is_some(),
            ) {
                open.set(None);
                preparing.set(None);
                placement.set(None);
                form_state.set(PairFormState::new(MirroredPairFormValues::default()));
                error.set(None);
                status.set(None);
                canvas_interaction.release(CanvasInteractionOwner::MirroredPair);
            }
        }
    });

    let on_preview = use_callback({
        let runtime = runtime.clone();
        let alive = alive.clone();
        let mut form_state = form_state;
        let mut preparing = preparing;
        let mut error = error;
        let mut status = status;
        let canvas_interaction = canvas_interaction.clone();
        move |request: MirroredPairRequest| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
                || open.read().as_ref() != Some(&request.owner)
                || pending.read().is_some()
                || preparing.read().is_some()
                || placement.read().is_some()
                || !form_state.read().setup_is_editable()
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
                || request.gap_mm < 0.0
                || !request.gap_mm.is_finite()
                || request.left_name.trim().is_empty()
                || request.right_name.trim().is_empty()
                || request.left_name.trim() == request.right_name.trim()
            {
                error.set(Some(
                    "Choose distinct names, a non-negative gap, and dimensions up to 4096 cells."
                        .into(),
                ));
                return;
            }
            let model = runtime.model();
            let live_scope = runtime.scope();
            let Some((snapshot, scope, board_id)) =
                pair_source(&runtime, &model, live_scope.as_ref(), workspace())
            else {
                error.set(Some(
                    "The selected board is no longer ready for mirrored-pair placement.".into(),
                ));
                return;
            };
            if scope != request.owner.scope
                || board_id != request.owner.board_id
                || snapshot.token != request.owner.snapshot_token
                || snapshot.document.revision != request.owner.revision
            {
                error.set(Some("The accepted board changed. Close this setup and reopen it before placing a pair.".into()));
                return;
            }
            let reversible = snapshot
                .document
                .parameters
                .get("reversibleLayout")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let left_matrix_id = next_matrix_id(
                &snapshot.document.matrices,
                &snapshot.document.definitions,
                &snapshot.document.parts,
                request.preset.as_str(),
                reversible,
            );
            form_state.set(PairFormState {
                stage: PairFormStage::Preparing,
                values: MirroredPairFormValues {
                    left_name: request.left_name.clone(),
                    right_name: request.right_name.clone(),
                    rows: request.rows.to_string(),
                    columns: request.columns.to_string(),
                    preset: request.preset,
                    gap_mm: request.gap_mm.to_string(),
                },
            });
            preparing.set(Some(request.owner.clone()));
            error.set(None);
            status.set(Some("Preparing mirrored pair definitions…".into()));

            let request_copy = request.clone();
            let accepted = snapshot.clone();
            let owner = request.owner.clone();
            let runtime = runtime.clone();
            let mut preparing = preparing;
            let mut placement = placement;
            let mut form_state = form_state;
            let mut error = error;
            let mut status = status;
            let alive = alive.clone();
            spawn_local(async move {
                if !alive.get() {
                    return;
                }
                let templates =
                    match crate::presentation::parts::load_matrix_templates(reversible).await {
                        Ok(templates) => templates,
                        Err(message) => {
                            if alive.get()
                                && pair_owner_is_current(
                                    &runtime,
                                    &owner,
                                    workspace(),
                                    scope_generation(),
                                    &open,
                                )
                            {
                                preparing.set(None);
                                form_state.write().stage = PairFormStage::Setup;
                                error.set(Some(message));
                                status.set(None);
                            } else if alive.get() {
                                clear_preparing(
                                    &mut preparing,
                                    &mut form_state,
                                    &owner,
                                    &open,
                                    &mut error,
                                    &mut status,
                                );
                            }
                            return;
                        }
                    };
                if !alive.get() {
                    return;
                }
                if !pair_owner_is_current(&runtime, &owner, workspace(), scope_generation(), &open)
                {
                    clear_preparing(
                        &mut preparing,
                        &mut form_state,
                        &owner,
                        &open,
                        &mut error,
                        &mut status,
                    );
                    return;
                }
                let mut prepared = match prepare_matrix(
                    left_matrix_id.clone(),
                    board_id.clone(),
                    MatrixSetupRequest {
                        rows: request_copy.rows,
                        columns: request_copy.columns,
                        preset: request_copy.preset,
                    },
                    reversible,
                    &templates,
                ) {
                    Ok(prepared) => prepared,
                    Err(message) => {
                        preparing.set(None);
                        form_state.write().stage = PairFormStage::Setup;
                        error.set(Some(message));
                        status.set(None);
                        return;
                    }
                };
                for definition in &mut prepared.definitions {
                    if !alive.get() {
                        return;
                    }
                    if !pair_owner_is_current(
                        &runtime,
                        &owner,
                        workspace(),
                        scope_generation(),
                        &open,
                    ) {
                        clear_preparing(
                            &mut preparing,
                            &mut form_state,
                            &owner,
                            &open,
                            &mut error,
                            &mut status,
                        );
                        return;
                    }
                    let normalized = match crate::presentation::parts::normalize_matrix_definition(
                        definition.clone(),
                    )
                    .await
                    {
                        Ok(normalized) => normalized,
                        Err(message) => {
                            if alive.get()
                                && pair_owner_is_current(
                                    &runtime,
                                    &owner,
                                    workspace(),
                                    scope_generation(),
                                    &open,
                                )
                            {
                                preparing.set(None);
                                form_state.write().stage = PairFormStage::Setup;
                                error.set(Some(message));
                                status.set(None);
                            } else if alive.get() {
                                clear_preparing(
                                    &mut preparing,
                                    &mut form_state,
                                    &owner,
                                    &open,
                                    &mut error,
                                    &mut status,
                                );
                            }
                            return;
                        }
                    };
                    if !alive.get() {
                        return;
                    }
                    if !pair_owner_is_current(
                        &runtime,
                        &owner,
                        workspace(),
                        scope_generation(),
                        &open,
                    ) {
                        clear_preparing(
                            &mut preparing,
                            &mut form_state,
                            &owner,
                            &open,
                            &mut error,
                            &mut status,
                        );
                        return;
                    }
                    *definition = normalized;
                }
                if !alive.get() {
                    return;
                }
                if !pair_owner_is_current(&runtime, &owner, workspace(), scope_generation(), &open)
                {
                    clear_preparing(
                        &mut preparing,
                        &mut form_state,
                        &owner,
                        &open,
                        &mut error,
                        &mut status,
                    );
                    return;
                }
                let mut matrices = accepted.document.matrices.clone();
                matrices.push(prepared.matrix.clone());
                let mut definitions = accepted.document.definitions.clone();
                definitions.extend(prepared.definitions.iter().cloned());
                let right_matrix_id = next_matrix_id(
                    &matrices,
                    &definitions,
                    &accepted.document.parts,
                    request_copy.preset.as_str(),
                    reversible,
                );
                let ids = MirroredPairIds {
                    right_matrix_id,
                    left_layout_id: next_layout_id(&accepted.document.layouts, "mirrored-left"),
                    right_layout_id: next_layout_id(&accepted.document.layouts, "mirrored-right"),
                };
                let geometry = MirroredPairGeometryInput {
                    left_name: request_copy.left_name.clone(),
                    right_name: request_copy.right_name.clone(),
                    gap_mm: request_copy.gap_mm,
                };
                let pair = match project_mirrored_pair(
                    prepared.matrix.clone(),
                    &ids,
                    &geometry,
                    boardstudio_core::model::Vec2::default(),
                ) {
                    Ok(pair) => pair,
                    Err(message) => {
                        preparing.set(None);
                        form_state.write().stage = PairFormStage::Setup;
                        error.set(Some(message));
                        status.set(None);
                        return;
                    }
                };
                let preview_scenes = match runtime
                    .project_matrices(
                        accepted.document.revision,
                        vec![pair.matrix.clone(), pair.right_preview.clone()],
                    )
                    .await
                {
                    Ok(scenes) => scenes,
                    Err(message) => {
                        if alive.get()
                            && pair_owner_is_current(
                                &runtime,
                                &owner,
                                workspace(),
                                scope_generation(),
                                &open,
                            )
                        {
                            preparing.set(None);
                            form_state.write().stage = PairFormStage::Setup;
                            error.set(Some(message));
                            status.set(None);
                        } else if alive.get() {
                            clear_preparing(
                                &mut preparing,
                                &mut form_state,
                                &owner,
                                &open,
                                &mut error,
                                &mut status,
                            );
                        }
                        return;
                    }
                };
                if !alive.get() {
                    return;
                }
                if !pair_owner_is_current(&runtime, &owner, workspace(), scope_generation(), &open)
                {
                    clear_preparing(
                        &mut preparing,
                        &mut form_state,
                        &owner,
                        &open,
                        &mut error,
                        &mut status,
                    );
                    return;
                }
                let [left_scene, right_scene] = preview_scenes.as_slice() else {
                    preparing.set(None);
                    form_state.write().stage = PairFormStage::Setup;
                    error.set(Some(
                        "Core returned an incomplete mirrored-pair preview.".into(),
                    ));
                    status.set(None);
                    return;
                };
                placement.set(Some(ActivePair {
                    owner: owner.clone(),
                    request: request_copy,
                    matrix: prepared.matrix,
                    definitions: prepared.definitions,
                    ids,
                    placement: MirroredPairPlacement {
                        owner,
                        pair,
                        left_scene: left_scene.clone(),
                        right_scene: right_scene.clone(),
                    },
                }));
                form_state.write().stage = PairFormStage::Placement;
                preparing.set(None);
                error.set(None);
                status.set(Some(
                    "Place linked halves · Click to place · Esc cancels".into(),
                ));
            });
        }
    });

    let on_move = use_callback({
        let runtime = runtime.clone();
        let mut placement = placement;
        let mut error = error;
        let canvas_interaction = canvas_interaction.clone();
        move |request: super::mirrored_pair::MirroredPairMove| {
            if !canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
                return;
            }
            let Some(active) = placement.read().clone() else {
                return;
            };
            if active.owner != request.owner
                || !pair_owner_is_current(
                    &runtime,
                    &request.owner,
                    workspace(),
                    scope_generation(),
                    &open,
                )
            {
                return;
            }
            match project_mirrored_pair(
                active.matrix.clone(),
                &active.ids,
                &MirroredPairGeometryInput {
                    left_name: active.request.left_name.clone(),
                    right_name: active.request.right_name.clone(),
                    gap_mm: active.request.gap_mm,
                },
                request.center,
            ) {
                Ok(pair) => {
                    let mut next = active;
                    next.placement.pair = pair;
                    placement.set(Some(next));
                    error.set(None);
                }
                Err(message) => error.set(Some(message)),
            }
        }
    });

    let on_commit = use_callback({
        let runtime = runtime.clone();
        let mut placement = placement;
        let mut pending = pending;
        let mut form_state = form_state;
        let mut status = status;
        let mut error = error;
        let canvas_interaction = canvas_interaction.clone();
        move |request: super::mirrored_pair::MirroredPairMove| {
            let owner = request.owner;
            if !canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
                || pending.read().is_some()
                || workspace() != "Layout"
                || scope_generation() != owner.scope_generation
                || !pair_owner_is_current(&runtime, &owner, workspace(), scope_generation(), &open)
            {
                return;
            }
            let Some(active) = placement
                .read()
                .clone()
                .filter(|active| active.owner == owner)
            else {
                return;
            };
            let current = runtime.model();
            let Some(snapshot) = current.accepted.as_ref() else {
                return;
            };
            let (layout_left, layout_right, matrix, definitions) = match project_mirrored_pair(
                active.matrix.clone(),
                &active.ids,
                &MirroredPairGeometryInput {
                    left_name: active.request.left_name.clone(),
                    right_name: active.request.right_name.clone(),
                    gap_mm: active.request.gap_mm,
                },
                request.center,
            ) {
                Ok(pair) => (
                    pair.left,
                    pair.right,
                    pair.matrix,
                    active.definitions.clone(),
                ),
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let operation_id = runtime.operation();
            let transaction_id = format!(
                "mirrored-pair-{}-{}-{}",
                editor_instance_id, owner.open_id, operation_id.0
            );
            let outcome = runtime.observe_operation(operation_id);
            let pending_pair = PendingPair {
                owner: owner.clone(),
                operation_id,
                outcome: outcome.clone(),
                left_matrix_id: matrix.id.clone(),
                right_matrix_id: active.ids.right_matrix_id.clone(),
                left_layout: layout_left.clone(),
                right_layout: layout_right.clone(),
                transaction_id: transaction_id.clone(),
            };
            pending.set(Some(pending_pair));
            placement.set(None);
            form_state.write().stage = PairFormStage::Pending;
            status.set(Some("Creating mirrored pair…".into()));
            error.set(None);
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id,
                    phase: EditPhase::Commit,
                    target_ids: vec![
                        matrix.id.clone(),
                        active.ids.right_matrix_id.clone(),
                        layout_left.id.clone(),
                        layout_right.id.clone(),
                    ],
                    operation: EditOperation::CreateMirroredPair {
                        left: layout_left,
                        right: layout_right,
                        matrix: Box::new(matrix),
                        definitions: Some(definitions),
                    },
                },
            });
            // The observer is the sole value captured across waits; this task never touches Editor
            // signals after an await. The Editor-owned effect performs the visible settlement.
            spawn_local(async move {
                while outcome.borrow().is_none() {
                    gloo_timers::future::TimeoutFuture::new(16).await;
                }
            });
        }
    });

    let form_state_value = form_state.read().clone();
    let form = if form_state_value.setup_is_visible() {
        open.read().clone().map(|owner| {
            let editable = form_state_value.setup_is_editable()
                && pending.read().is_none()
                && preparing.read().is_none()
                && pair_owner_is_current(&runtime, &owner, workspace(), scope_generation(), &open);
            MirroredPairFormProjection {
                owner,
                values: form_state_value.values.clone(),
                editable,
                error: error.read().clone(),
                status: status.read().clone(),
            }
        })
    } else {
        None
    };
    let placement_projection = placement
        .read()
        .as_ref()
        .map(|active| active.placement.clone());
    let model = runtime.model();
    let can_open = canvas_interaction.current().is_none()
        && pending.read().is_none()
        && preparing.read().is_none()
        && placement.read().is_none()
        && open.read().is_none()
        && pair_source(&runtime, &model, runtime.scope().as_ref(), workspace()).is_some();
    let owns_canvas = open.read().is_some()
        || preparing.read().is_some()
        || placement.read().is_some()
        || pending.read().is_some();
    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation(), &owns_canvas),
        {
            let canvas_interaction = canvas_interaction.clone();
            move |(_, _, _, owns_canvas)| {
                if !owns_canvas {
                    canvas_interaction.release(CanvasInteractionOwner::MirroredPair);
                }
            }
        },
    ));
    MirroredPairMount {
        form,
        placement: placement_projection,
        can_open,
        owns_canvas,
        on_open,
        on_cancel,
        on_preview,
        on_move,
        on_commit,
        on_created,
    }
}

fn pair_owner_is_current(
    runtime: &Runtime,
    owner: &MirroredPairOwner,
    workspace: &'static str,
    scope_generation: u64,
    open: &Signal<Option<MirroredPairOwner>>,
) -> bool {
    if open.read().as_ref() != Some(owner)
        || workspace != "Layout"
        || scope_generation != owner.scope_generation
        || runtime.scope().as_ref() != Some(&owner.scope)
    {
        return false;
    }
    let model = runtime.model();
    let Some((snapshot, scope, board_id)) =
        pair_source(runtime, &model, runtime.scope().as_ref(), workspace)
    else {
        return false;
    };
    scope == owner.scope
        && board_id == owner.board_id
        && snapshot.token == owner.snapshot_token
        && snapshot.document.revision == owner.revision
}

fn pair_source<'a>(
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

fn next_layout_id(layouts: &[Layout], prefix: &str) -> String {
    let used = layouts
        .iter()
        .map(|layout| layout.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut next = 1u64;
    loop {
        let candidate = format!("{prefix}-{next}");
        if !used.contains(candidate.as_str()) {
            return candidate;
        }
        next = next.checked_add(1).expect("layout identity exhausted");
    }
}

fn clear_preparing(
    preparing: &mut Signal<Option<MirroredPairOwner>>,
    form_state: &mut Signal<PairFormState<MirroredPairFormValues>>,
    owner: &MirroredPairOwner,
    open: &Signal<Option<MirroredPairOwner>>,
    error: &mut Signal<Option<String>>,
    status: &mut Signal<Option<String>>,
) {
    if preparing.read().as_ref() == Some(owner) {
        preparing.set(None);
        form_state.write().stage = PairFormStage::Setup;
        status.set(None);
        if open.read().as_ref() == Some(owner) {
            error.set(Some(
                "The accepted board changed. Cancel this setup and reopen it before placing a pair."
                    .into(),
            ));
        }
    }
}

struct PairSettlement<'a> {
    pending: &'a mut Signal<Option<PendingPair>>,
    open: &'a mut Signal<Option<MirroredPairOwner>>,
    form_state: &'a mut Signal<PairFormState<MirroredPairFormValues>>,
    error: &'a mut Signal<Option<String>>,
    status: &'a mut Signal<Option<String>>,
    placement: &'a mut Signal<Option<ActivePair>>,
    on_created: EventHandler<MirroredPairCreated>,
}

fn settle_pending(
    runtime: &Rc<Runtime>,
    workspace: &'static str,
    scope_generation: u64,
    signals: &mut PairSettlement<'_>,
) {
    let Some(waiting) = signals.pending.read().clone() else {
        return;
    };
    let Some(outcome) = waiting.outcome.borrow().clone() else {
        return;
    };
    let model = runtime.model();
    let same_lineage = runtime.scope().as_ref() == Some(&waiting.owner.scope)
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == waiting.owner.scope.session_epoch
                && snapshot.document.id == waiting.owner.scope.document_id
        });
    if !same_lineage {
        finish_pending(signals.pending, signals.status, &waiting);
        signals.open.set(None);
        signals
            .form_state
            .set(PairFormState::new(MirroredPairFormValues::default()));
        signals.placement.set(None);
        signals.error.set(None);
        return;
    }
    match outcome {
        TerminalOutcome::Completed => {
            let Some(snapshot) = model.accepted.as_ref() else {
                finish_pending(signals.pending, signals.status, &waiting);
                signals.open.set(None);
                signals
                    .form_state
                    .set(PairFormState::new(MirroredPairFormValues::default()));
                return;
            };
            if matches!(model.durability, Durability::Failed { .. })
                || matches!(
                    model.lifecycle,
                    Lifecycle::RecoveryRequired | Lifecycle::Closed
                )
            {
                finish_pending(signals.pending, signals.status, &waiting);
                signals.form_state.write().stage = PairFormStage::Setup;
                if workspace == "Layout"
                    && scope_generation == waiting.owner.scope_generation
                    && signals.open.read().as_ref() == Some(&waiting.owner)
                {
                    signals.placement.set(None);
                    signals.error.set(Some("The pair operation completed, but its document was not saved. Correct the setup and retry after recovery.".into()));
                }
                return;
            }
            let exact_result = crate::mirrored_pair_lifecycle::accepted_saved_result_is_current(
                &PairResultGuard {
                    transaction_id: waiting.transaction_id.clone(),
                    base_token: waiting.owner.snapshot_token,
                    base_revision: waiting.owner.revision,
                },
                &snapshot.scene.transaction_id,
                snapshot.token,
                snapshot.document.revision,
                model.lifecycle == Lifecycle::Ready,
                &model.durability,
            );
            if !exact_result {
                finish_pending(signals.pending, signals.status, &waiting);
                signals.placement.set(None);
                signals.form_state.write().stage = PairFormStage::Setup;
                if workspace == "Layout"
                    && scope_generation == waiting.owner.scope_generation
                    && signals.open.read().as_ref() == Some(&waiting.owner)
                {
                    signals.error.set(Some(
                        "The board advanced after saving this pair, so it was not selected. Review the saved layout before continuing.".into(),
                    ));
                }
                return;
            }
            let created = snapshot
                .document
                .matrices
                .iter()
                .any(|matrix| matrix.id == waiting.left_matrix_id)
                && snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == waiting.right_matrix_id)
                && snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout == &waiting.left_layout)
                && snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout == &waiting.right_layout);
            finish_pending(signals.pending, signals.status, &waiting);
            if !created {
                signals.form_state.write().stage = PairFormStage::Setup;
                if workspace == "Layout"
                    && scope_generation == waiting.owner.scope_generation
                    && signals.open.read().as_ref() == Some(&waiting.owner)
                {
                    signals.placement.set(None);
                    signals.error.set(Some("The saved document does not contain the mirrored pair. Review the board and retry.".into()));
                }
                return;
            }
            signals.placement.set(None);
            if workspace != "Layout"
                || scope_generation != waiting.owner.scope_generation
                || runtime.scope().as_ref() != Some(&waiting.owner.scope)
                || signals.open.read().as_ref() != Some(&waiting.owner)
                || model.active_board_id != waiting.owner.board_id
            {
                signals.open.set(None);
                return;
            }
            signals.open.set(None);
            signals
                .form_state
                .set(PairFormState::new(MirroredPairFormValues::default()));
            signals.error.set(None);
            signals.on_created.call(MirroredPairCreated {
                owner: waiting.owner.clone(),
                result_token: snapshot.token,
                result_revision: snapshot.document.revision,
                scope: waiting.owner.scope.clone(),
                left_layout_id: waiting.left_layout.id.clone(),
                right_layout_id: waiting.right_layout.id.clone(),
                left_matrix_id: waiting.left_matrix_id,
                right_matrix_id: waiting.right_matrix_id,
            });
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => {
            finish_pending(signals.pending, signals.status, &waiting);
            signals.form_state.write().stage = PairFormStage::Setup;
            if workspace == "Layout"
                && scope_generation == waiting.owner.scope_generation
                && runtime.scope().as_ref() == Some(&waiting.owner.scope)
                && signals.open.read().as_ref() == Some(&waiting.owner)
            {
                signals.open.set(Some(waiting.owner));
                signals.error.set(Some(message));
            }
        }
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            finish_pending(signals.pending, signals.status, &waiting);
            signals.form_state.write().stage = PairFormStage::Setup;
            if workspace == "Layout"
                && scope_generation == waiting.owner.scope_generation
                && signals.open.read().as_ref() == Some(&waiting.owner)
            {
                signals.open.set(Some(waiting.owner));
                signals.error.set(Some(
                    "Mirrored-pair creation did not complete. Review the setup and try again."
                        .into(),
                ));
            }
        }
    }
}

fn finish_pending(
    pending: &mut Signal<Option<PendingPair>>,
    status: &mut Signal<Option<String>>,
    waiting: &PendingPair,
) {
    if pending
        .read()
        .as_ref()
        .is_some_and(|current| current.operation_id == waiting.operation_id)
    {
        pending.set(None);
        status.set(None);
    }
}
