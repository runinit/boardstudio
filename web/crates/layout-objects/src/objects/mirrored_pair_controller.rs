//! Editor-lifetime owner for mirrored-pair preparation, placement, and exact saved acknowledgement.
use super::matrix_preparation::{
    PendingSettlement, fail_creation, matrix_dimensions_valid, matrix_source, owner_source_matches,
    prepare_matrix_definitions, reversible_layout_param,
};
use super::mirrored_pair::{
    MirroredPairCreated, MirroredPairFormProjection, MirroredPairFormValues, MirroredPairMount,
    MirroredPairOwner, MirroredPairPlacement, MirroredPairRequest,
};
use crate::{
    matrix_setup_operation::{MatrixSetupRequest, next_matrix_id},
    mirrored_pair_geometry::{MirroredPairGeometryInput, MirroredPairIds, project_mirrored_pair},
    mirrored_pair_lifecycle::{PairFormStage, PairFormState},
    presentation::canvas_interaction::{CanvasMode, LayoutInteraction},
    runtime::Runtime,
};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{EditOperation, Layout};
use boardstudio_web_runtime::operation_outcomes::Settlement;
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct PairFollowUp {
    owner: MirroredPairOwner,
    left_matrix_id: String,
    right_matrix_id: String,
    left_layout_id: String,
    right_layout_id: String,
}

/// Resolve creating a mirrored pair against the accepted document at execution time. The
/// pair was prepared from values captured at submit; here the board must still exist and
/// none of the new matrices or layouts may already be in use.
fn mirrored_pair_resolver(
    board_id: String,
    left: Layout,
    right: Layout,
    matrix: boardstudio_model::model::Matrix,
    right_matrix_id: String,
    definitions: Vec<boardstudio_model::model::PartDefinition>,
) -> EditResolver {
    EditResolver::new(
        "layout-mirrored-pair",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if !document.boards.iter().any(|board| board.id == board_id) {
                return Resolution::Retire("The board no longer exists.".into());
            }
            let in_use = |id: &str| {
                document.matrices.iter().any(|matrix| matrix.id == id)
                    || document.layouts.iter().any(|layout| layout.id == id)
            };
            if [&matrix.id, &right_matrix_id, &left.id, &right.id]
                .into_iter()
                .any(|id| in_use(id))
            {
                return Resolution::Retire(
                    "The mirrored pair's identity is already in use.".into(),
                );
            }
            Resolution::submit(
                vec![
                    matrix.id.clone(),
                    right_matrix_id.clone(),
                    left.id.clone(),
                    right.id.clone(),
                ],
                EditOperation::CreateMirroredPair {
                    left: left.clone(),
                    right: right.clone(),
                    matrix: Box::new(matrix.clone()),
                    definitions: Some(definitions.clone()),
                },
            )
        },
    )
}

#[derive(Clone)]
struct ActivePair {
    owner: MirroredPairOwner,
    request: MirroredPairRequest,
    matrix: boardstudio_model::model::Matrix,
    definitions: Vec<boardstudio_model::model::PartDefinition>,
    ids: MirroredPairIds,
    placement: MirroredPairPlacement,
}

/// Called once during the Editor lifetime. The parent mounts its form and forwards the typed
/// world-coordinate placement callbacks from the existing SVG event owner.
pub fn use_mirrored_pair(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
    on_created: EventHandler<MirroredPairCreated>,
) -> MirroredPairMount {
    let canvas_interaction = use_context::<LayoutInteraction>();
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
    // The committed pair is observed through its ticket; the waiting entry (ticket plus
    // the identities the Landed follow-up needs) is the pending state. No separate
    // disabled flag is kept: availability derives from it.
    let waiting = use_signal(|| None::<PendingSettlement<PairFollowUp>>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let alive = alive.clone();
            let mut open = open;
            let mut form_state = form_state;
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            let mut placement = placement;
            let mut waiting = waiting;
            move |(_, workspace, scope_generation)| {
                let owner_is_live = alive.get()
                    && workspace == Workspace::Layout
                    && waiting.read().as_ref().is_some_and(|pending| {
                        let follow_up = &pending.owner;
                        follow_up.owner.editor_instance_id == editor_instance_id
                            && follow_up.owner.scope_generation == scope_generation
                    });
                settle_pending(
                    &runtime,
                    workspace,
                    scope_generation,
                    &mut PairSettlement {
                        waiting: &mut waiting,
                        owner_is_live,
                        open: &mut open,
                        form_state: &mut form_state,
                        error: &mut error,
                        status: &mut status,
                        placement: &mut placement,
                        on_created,
                    },
                );
                let create_pending = waiting.read().is_some();
                if !create_pending
                    && open.read().as_ref().is_some_and(|owner| {
                        workspace != Workspace::Layout
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
                if !create_pending
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

        move |_| {
            if waiting.read().is_some()
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
                matrix_source(&runtime, &model, current_scope.as_ref(), current_workspace)
            else {
                error.set(Some(
                    "Open a saved, editable board in Layout before creating a mirrored pair."
                        .into(),
                ));
                return;
            };
            if !canvas_interaction.begin(CanvasMode::PlacingMirroredPair) {
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

        move |owner: MirroredPairOwner| {
            let active = open.read().as_ref() == Some(&owner)
                || placement
                    .read()
                    .as_ref()
                    .is_some_and(|pair| pair.owner == owner);
            let create_pending = waiting.read().is_some();
            if crate::mirrored_pair_lifecycle::pair_cancel_is_allowed(active, create_pending) {
                open.set(None);
                preparing.set(None);
                placement.set(None);
                form_state.set(PairFormState::new(MirroredPairFormValues::default()));
                error.set(None);
                status.set(None);
                canvas_interaction.end(CanvasMode::PlacingMirroredPair);
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

        move |request: MirroredPairRequest| {
            if canvas_interaction.current() != Some(CanvasMode::PlacingMirroredPair)
                || open.read().as_ref() != Some(&request.owner)
                || waiting.read().is_some()
                || preparing.read().is_some()
                || placement.read().is_some()
                || !form_state.read().setup_is_editable()
                || request.owner.editor_instance_id != editor_instance_id
                || workspace() != Workspace::Layout
                || scope_generation() != request.owner.scope_generation
            {
                return;
            }
            if !matrix_dimensions_valid(request.rows, request.columns)
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
                matrix_source(&runtime, &model, live_scope.as_ref(), workspace())
            else {
                error.set(Some(
                    "The selected board is no longer ready for mirrored-pair placement.".into(),
                ));
                return;
            };
            if !owner_source_matches(
                &request.owner.scope,
                request.owner.snapshot_token,
                request.owner.revision,
                &scope,
                snapshot,
            ) {
                error.set(Some("The accepted board changed. Close this setup and reopen it before placing a pair.".into()));
                return;
            }
            let reversible = reversible_layout_param(&snapshot.document);
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

            let mut request_copy = request.clone();
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
                let fail_setup =
                    |form_state: &mut Signal<PairFormState<MirroredPairFormValues>>,
                     preparing: &mut Signal<Option<MirroredPairOwner>>,
                     error: &mut Signal<Option<String>>,
                     status: &mut Signal<Option<String>>,
                     message: String| {
                        form_state.write().stage = PairFormStage::Setup;
                        fail_creation(preparing, error, status, message);
                    };
                let owner_is_current = || {
                    alive.get()
                        && pair_owner_is_current(
                            &runtime,
                            &owner,
                            workspace(),
                            scope_generation(),
                            &open,
                        )
                };
                let prepared = prepare_matrix_definitions(
                    left_matrix_id,
                    board_id,
                    MatrixSetupRequest {
                        rows: request_copy.rows,
                        columns: request_copy.columns,
                        preset: request_copy.preset,
                    },
                    reversible,
                    owner_is_current,
                )
                .await;
                if !alive.get() {
                    return;
                }
                let prepared = match prepared {
                    Ok(Some(prepared)) if owner_is_current() => prepared,
                    Err(message) if owner_is_current() => {
                        fail_setup(
                            &mut form_state,
                            &mut preparing,
                            &mut error,
                            &mut status,
                            message,
                        );
                        return;
                    }
                    _ => {
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
                };
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
                    boardstudio_model::model::Vec2::default(),
                ) {
                    Ok(pair) => pair,
                    Err(message) => {
                        fail_setup(
                            &mut form_state,
                            &mut preparing,
                            &mut error,
                            &mut status,
                            message,
                        );
                        return;
                    }
                };
                // Ticket 11: propose the stored gap before the preview, so the
                // setup preview and the placed pair show the same adapted gap.
                // The adapted value is translation-invariant, so one proposal
                // covers every placement move and the commit stores it as-is.
                let adapted_gap =
                    match boardstudio_core_kernel::matrix::layout::proposed_creation_gap(
                        &accepted.document,
                        &pair.matrix,
                        pair.axis_x,
                    ) {
                        Ok(gap) => gap,
                        Err(message) => {
                            fail_setup(
                                &mut form_state,
                                &mut preparing,
                                &mut error,
                                &mut status,
                                message,
                            );
                            return;
                        }
                    };
                request_copy.gap_mm = adapted_gap;
                let pair = if adapted_gap > geometry.gap_mm {
                    let geometry = MirroredPairGeometryInput {
                        left_name: request_copy.left_name.clone(),
                        right_name: request_copy.right_name.clone(),
                        gap_mm: adapted_gap,
                    };
                    match project_mirrored_pair(
                        prepared.matrix.clone(),
                        &ids,
                        &geometry,
                        boardstudio_model::model::Vec2::default(),
                    ) {
                        Ok(pair) => pair,
                        Err(message) => {
                            fail_setup(
                                &mut form_state,
                                &mut preparing,
                                &mut error,
                                &mut status,
                                message,
                            );
                            return;
                        }
                    }
                } else {
                    pair
                };
                let preview_scenes = match runtime.project_matrices(
                    accepted.document.revision,
                    vec![pair.matrix.clone(), pair.right_preview.clone()],
                ) {
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
                            fail_setup(
                                &mut form_state,
                                &mut preparing,
                                &mut error,
                                &mut status,
                                message,
                            );
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
                    fail_setup(
                        &mut form_state,
                        &mut preparing,
                        &mut error,
                        &mut status,
                        "Core returned an incomplete mirrored-pair preview.".into(),
                    );
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

        move |request: super::mirrored_pair::MirroredPairMove| {
            if canvas_interaction.current() != Some(CanvasMode::PlacingMirroredPair) {
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
        let mut waiting = waiting;
        let mut error = error;

        move |request: super::mirrored_pair::MirroredPairMove| {
            let owner = request.owner;
            if canvas_interaction.current() != Some(CanvasMode::PlacingMirroredPair)
                || waiting.read().is_some()
                || workspace() != Workspace::Layout
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
            let left_matrix_id = matrix.id.clone();
            let ticket = runtime.resolve_edit(
                "layout-mirrored-pair",
                Some("mirrored pair".into()),
                mirrored_pair_resolver(
                    owner.board_id.clone(),
                    layout_left.clone(),
                    layout_right.clone(),
                    matrix,
                    active.ids.right_matrix_id.clone(),
                    definitions,
                ),
            );
            waiting.set(Some(PendingSettlement {
                ticket,
                owner: PairFollowUp {
                    owner: owner.clone(),
                    left_matrix_id,
                    right_matrix_id: active.ids.right_matrix_id.clone(),
                    left_layout_id: layout_left.id.clone(),
                    right_layout_id: layout_right.id.clone(),
                },
            }));
            placement.set(None);
            error.set(None);
        }
    });

    let create_pending = waiting.read().is_some();
    let form_state_value = form_state.read().clone();
    let form = if form_state_value.setup_is_visible() {
        open.read().clone().map(|owner| {
            let editable = form_state_value.setup_is_editable()
                && !create_pending
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
        && !create_pending
        && preparing.read().is_none()
        && placement.read().is_none()
        && open.read().is_none()
        && matrix_source(&runtime, &model, runtime.scope().as_ref(), workspace()).is_some();
    let owns_canvas = open.read().is_some()
        || preparing.read().is_some()
        || placement.read().is_some()
        || create_pending;
    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation(), &owns_canvas),
        {
            move |(_, _, _, owns_canvas)| {
                if !owns_canvas {
                    canvas_interaction.end(CanvasMode::PlacingMirroredPair);
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
    workspace: Workspace,
    scope_generation: u64,
    open: &Signal<Option<MirroredPairOwner>>,
) -> bool {
    if open.read().as_ref() != Some(owner)
        || workspace != Workspace::Layout
        || scope_generation != owner.scope_generation
        || runtime.scope().as_ref() != Some(&owner.scope)
    {
        return false;
    }
    let model = runtime.model();
    let Some((snapshot, scope, _)) =
        matrix_source(runtime, &model, runtime.scope().as_ref(), workspace)
    else {
        return false;
    };
    owner_source_matches(
        &owner.scope,
        owner.snapshot_token,
        owner.revision,
        &scope,
        snapshot,
    )
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
    waiting: &'a mut Signal<Option<PendingSettlement<PairFollowUp>>>,
    owner_is_live: bool,
    open: &'a mut Signal<Option<MirroredPairOwner>>,
    form_state: &'a mut Signal<PairFormState<MirroredPairFormValues>>,
    error: &'a mut Signal<Option<String>>,
    status: &'a mut Signal<Option<String>>,
    placement: &'a mut Signal<Option<ActivePair>>,
    on_created: EventHandler<MirroredPairCreated>,
}

fn settle_pending(
    runtime: &Rc<Runtime>,
    workspace: Workspace,
    scope_generation: u64,
    signals: &mut PairSettlement<'_>,
) {
    let Some((follow_up, settlement)) =
        PendingSettlement::finish(signals.waiting, |_| signals.owner_is_live)
    else {
        return;
    };
    let model = runtime.model();
    let owner_is_visible = workspace == Workspace::Layout
        && scope_generation == follow_up.owner.scope_generation
        && signals.open.read().as_ref() == Some(&follow_up.owner);
    signals.status.set(None);
    match settlement {
        Settlement::Landed { revision } => {
            let token = model.accepted.as_ref().map(|snapshot| snapshot.token);
            signals.placement.set(None);
            let (Some(token), true) = (
                token,
                owner_is_visible && model.active_board_id == follow_up.owner.board_id,
            ) else {
                signals.open.set(None);
                return;
            };
            signals.open.set(None);
            signals
                .form_state
                .set(PairFormState::new(MirroredPairFormValues::default()));
            signals.error.set(None);
            signals.on_created.call(MirroredPairCreated {
                owner: follow_up.owner.clone(),
                result_token: token,
                result_revision: revision,
                scope: follow_up.owner.scope.clone(),
                left_layout_id: follow_up.left_layout_id.clone(),
                right_layout_id: follow_up.right_layout_id.clone(),
                left_matrix_id: follow_up.left_matrix_id,
                right_matrix_id: follow_up.right_matrix_id,
            });
        }
        Settlement::Failed { message } => {
            signals.form_state.write().stage = PairFormStage::Setup;
            if owner_is_visible {
                signals.open.set(Some(follow_up.owner));
                signals.error.set(Some(message));
            }
        }
        Settlement::Retired => {
            signals.form_state.write().stage = PairFormStage::Setup;
            if owner_is_visible {
                signals.open.set(None);
                signals
                    .form_state
                    .set(PairFormState::new(MirroredPairFormValues::default()));
                signals.error.set(None);
            }
        }
    }
}
