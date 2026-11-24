//! Private immediate-origin Matrix Setup form, shared by Layout and the project guide.
use super::matrix_setup_fields::{
    MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview, matrix_setup_preset,
};
use crate::matrix_setup_operation::MatrixSetupPreset;
pub use crate::presentation::parts::MatrixPlacementSource;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use dioxus::prelude::*;
use std::collections::HashSet;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupProjection {
    pub owner: MatrixSetupOwner,
    pub editable: bool,
    pub can_cancel: bool,
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupCreateRequest {
    pub owner: MatrixSetupOwner,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
}

#[derive(Clone, PartialEq)]
pub struct MatrixSetupMount {
    pub projection: Option<MatrixSetupProjection>,
    pub can_open: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixPlacementOwner {
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementProjection {
    pub owner: MatrixPlacementOwner,
    pub matrix: boardstudio_model::model::Matrix,
    pub scene: boardstudio_model::model::MatrixScene,
    pub definitions: Vec<boardstudio_model::model::PartDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementMove {
    pub owner: MatrixPlacementOwner,
    pub center: boardstudio_model::model::Vec2,
}

#[derive(Clone, PartialEq)]
pub struct MatrixPlacementSession {
    pub cancel_owner: Option<MatrixPlacementOwner>,
    pub placement: Option<MatrixPlacementProjection>,
    pub busy: bool,
    pub error: Option<String>,
    pub on_place: EventHandler<MatrixPlacementSource>,
    pub on_move: EventHandler<MatrixPlacementMove>,
    pub on_commit: EventHandler<MatrixPlacementMove>,
    pub on_cancel: EventHandler<MatrixPlacementOwner>,
}

#[derive(Clone)]
pub struct MatrixPlacementInput {
    pub version: Signal<u64>,
    pub selected_context: Signal<Option<super::ScopedTreeContext>>,
    pub anchor_scope: Signal<Option<Scope>>,
    pub workspace: Signal<Workspace>,
    pub scope_generation: Signal<u64>,
    pub assembly_orientation: crate::presentation::parts::PartsAssemblyOrientation,
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixSetupProps {
    pub projection: MatrixSetupProjection,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[component]
pub fn MatrixSetup(props: MatrixSetupProps) -> Element {
    let rows = use_signal(|| "5".to_owned());
    let columns = use_signal(|| "6".to_owned());
    let row_slider = use_signal(|| Some(5.0));
    let column_slider = use_signal(|| Some(6.0));
    let family = use_signal(|| Some(HashSet::from([0usize])));
    let mounting = use_signal(|| Some(HashSet::from([0usize])));
    let lighting = use_signal(|| Some(HashSet::from([0usize])));
    let return_focus = use_signal(|| None::<web_sys::HtmlElement>);
    let rows_value = rows();
    let columns_value = columns();
    let choc = family().is_some_and(|choice| choice.contains(&1));
    let hotswap = mounting().is_some_and(|choice| choice.contains(&1));
    let rgb = lighting().is_some_and(|choice| choice.contains(&1));
    let selected_preset = matrix_setup_preset(choc, hotswap, rgb);
    let owner = props.projection.owner.clone();
    let feedback_error = props.projection.error.clone();
    let feedback_status = props.projection.status.clone();
    let create_handler = props.on_create;
    let submit_owner = owner.clone();
    let submit_rows = rows_value.clone();
    let submit_columns = columns_value.clone();
    let on_submit = move |event: FormEvent| {
        event.prevent_default();
        let Ok(rows) = submit_rows.trim().parse::<u32>() else {
            return;
        };
        let Ok(columns) = submit_columns.trim().parse::<u32>() else {
            return;
        };
        create_handler.call(MatrixSetupCreateRequest {
            owner: submit_owner.clone(),
            rows,
            columns,
            preset: selected_preset,
        });
    };
    let cancel_handler = props.on_cancel;
    let cancel_owner = owner.clone();
    let on_cancel = move |_| {
        cancel_handler.call(cancel_owner.clone());
        let return_focus = return_focus();
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            if let Some(target) = return_focus.filter(|target| target.is_connected()) {
                let _ = target.focus();
            } else if let Some(target) = document
                .get_element_by_id("m1-setup-guide-heading")
                .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = target.focus();
            }
        });
    };
    let mut return_focus_on_mount = return_focus;

    rsx! {
        section {
            class: "absolute inset-x-2 top-[100px] bottom-2 z-6 flex flex-col gap-2.5 overflow-auto rounded-md border border-border-strong bg-card p-3 text-foreground shadow-md [&_form]:grid [&_form]:gap-2 [&_footer]:flex [&_footer]:justify-end [&_footer]:gap-2",
            aria_label: "New matrix",
            onkeydown: super::stop_space_pan,
            header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-[16px]!", h2 { "New matrix" } }
            form { onsubmit: on_submit,
                div {
                    id: "m1-matrix-rows-focus",
                    onmounted: move |_| async move {
                        if let Some(target) = web_sys::window()
                            .and_then(|window| window.document())
                            .and_then(|document| document.active_element())
                            .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                        { return_focus_on_mount.set(Some(target)); }
                        if let Some(target) = web_sys::window()
                            .and_then(|window| window.document())
                            .and_then(|document| document.query_selector("#m1-matrix-rows-focus [role='slider'][aria-label='Rows']").ok().flatten())
                            .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                        { let _ = target.focus(); }
                    },
                    MatrixDimensionSliders {
                        rows, columns, row_slider, column_slider,
                        rows_label: "Rows", columns_label: "Columns", editable: props.projection.editable,
                    }
                }
                MatrixAssemblyPicker { family, mounting, lighting, editable: props.projection.editable }
                MatrixPreview { rows: rows_value.clone(), columns: columns_value.clone() }
                if let Some(message) = feedback_error {
                    Alert { variant: AlertVariant::Destructive, AlertDescription { "{message}" } }
                }
                if let Some(message) = feedback_status {
                    div { role: "status", Callout { variant: CalloutVariant::Info, "{message}" } }
                }
                footer {
                    if props.projection.can_cancel {
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| on_cancel(owner.clone()), "Cancel" }
                    }
                    Button { variant: ButtonVariant::Secondary, button_type: "submit", disabled: !props.projection.editable, "Create matrix" }
                }
            }
        }
    }
}
/// Editor-lifetime owner for placing a single Parts-library matrix assembly.
use super::matrix_preparation::{
    PendingSettlement, owner_session_current, prepare_matrix_definitions, reversible_layout_param,
    session_scope_matches,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{
    matrix_setup_operation::{MatrixSetupRequest, next_matrix_id},
    presentation::canvas_interaction::{CanvasMode, LayoutInteraction},
    runtime::Runtime,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution,
};
use boardstudio_model::model::{EditOperation, Matrix, PartDefinition};
use boardstudio_web_runtime::operation_outcomes::Settlement;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct PlacementFollowUp {
    owner: MatrixPlacementOwner,
    matrix_id: String,
    selected_context: Option<ScopedTreeContext>,
    selected_part_ids: Vec<String>,
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
) -> MatrixPlacementSession {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let MatrixPlacementInput {
        version,
        selected_context,
        anchor_scope,
        workspace,
        scope_generation,
        assembly_orientation,
    } = input;
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();

        move || {
            alive.set(false);
            canvas_interaction.end(CanvasMode::PlacingMatrix);
        }
    });
    let request_id = use_signal(|| 0u64);
    let preparing = use_signal(|| None::<MatrixPlacementOwner>);
    let placement = use_signal(|| None::<MatrixPlacementProjection>);
    // The committed placement is observed through its ticket; the waiting entry (ticket
    // plus the selection it should restore) is the pending state. Availability derives
    // from it, so no separate disabled flag is kept.
    let waiting = use_signal(|| None::<PendingSettlement<PlacementFollowUp>>);
    let error = use_signal(|| None::<String>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();

            let mut preparing = preparing;
            let mut placement = placement;
            let mut waiting = waiting;
            let mut error = error;
            let mut selected_context = selected_context;
            let mut anchor_scope = anchor_scope;
            move |(_, current_workspace, current_generation)| {
                if let Some((follow_up, settlement)) =
                    PendingSettlement::finish(&mut waiting, |follow_up| {
                        session_scope_matches(&runtime, &follow_up.owner.scope)
                    })
                {
                    let visible = current_workspace == Workspace::Layout
                        && current_generation == follow_up.owner.scope_generation
                        && runtime.model().selected_part_ids == follow_up.selected_part_ids
                        && selected_context.peek().as_ref() == follow_up.selected_context.as_ref();
                    match settlement {
                        Settlement::Retired => {
                            error.set(None);
                        }
                        Settlement::Landed { .. } => {
                            let model = runtime.model();
                            let created = model.accepted.as_ref().is_some_and(|snapshot| {
                                snapshot
                                    .document
                                    .matrices
                                    .iter()
                                    .any(|matrix| matrix.id == follow_up.matrix_id)
                            });
                            if !created || !visible {
                                error.set(None);
                            } else {
                                let context = TreeContext::Matrix {
                                    matrix_id: follow_up.matrix_id.clone(),
                                };
                                match super::resolve_selection(&model, &context) {
                                None => error.set(Some("The matrix was saved, but its Layout selection could not be restored.".into())),
                                Some(part_ids) => {
                                    selected_context.set(Some(ScopedTreeContext {
                                        scope: follow_up.owner.scope.clone(),
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
                        Settlement::Failed { message } => {
                            error.set(visible.then_some(message));
                        }
                    }
                }
                let active_owner = preparing
                    .read()
                    .clone()
                    .or_else(|| placement.read().as_ref().map(|active| active.owner.clone()));
                if waiting.read().is_none()
                    && let Some(owner) = active_owner
                    && (current_workspace != Workspace::Layout
                        || current_generation != owner.scope_generation
                        || !owner_session_current(
                            &runtime,
                            &owner.scope,
                            &owner.board_id,
                            owner.snapshot_token,
                            owner.revision,
                        ))
                {
                    preparing.set(None);
                    placement.set(None);
                    error.set(None);
                    canvas_interaction.end(CanvasMode::PlacingMatrix);
                }
            }
        },
    ));

    let on_place = use_callback({
        let runtime = runtime.clone();
        let alive = alive.clone();

        let mut request_id = request_id;
        let mut preparing = preparing;
        let mut error = error;
        let assembly_orientation = assembly_orientation.0;
        move |source: super::MatrixPlacementSource| {
            if waiting.read().is_some() || preparing.read().is_some() || placement.read().is_some()
            {
                return;
            }
            let Some((snapshot, scope, board_id)) = placement_source(&runtime) else {
                error.set(Some(
                    "Open a saved, editable board before placing a key assembly.".into(),
                ));
                return;
            };
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
            let reversible = reversible_layout_param(&accepted.document);
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
            if !canvas_interaction.begin(CanvasMode::PlacingMatrix) {
                return;
            }
            preparing.set(Some(owner.clone()));
            error.set(None);
            let runtime = runtime.clone();
            let alive = alive.clone();
            let mut preparing = preparing;
            let mut placement = placement;
            let mut error = error;

            spawn_local(async move {
                let result = async {
                    let (mut matrix, definitions) = match source {
                        super::MatrixPlacementSource::Preset(preset_id) => {
                            let preset = crate::presentation::parts::matrix_setup_preset(preset_id);
                            let Some(prepared) = prepare_matrix_definitions(
                                matrix_id.clone(),
                                owner.board_id.clone(),
                                MatrixSetupRequest {
                                    rows: 1,
                                    columns: 1,
                                    preset,
                                },
                                reversible,
                                || alive.get(),
                            )
                            .await?
                            else {
                                // A stale owner mid-preparation drops the result: the
                                // watchers below clear `preparing` once the scope moves on.
                                return Ok(None);
                            };
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
                            use boardstudio_model::model::{MatrixCell, Vec2 as SeedVec2};
                            let seed = boardstudio_model::model::Matrix {
                                id: matrix_id.clone(),
                                name: Some(assembly.name.clone()),
                                rows: 1,
                                columns: 1,
                                pitch: SeedVec2 { x: 19.05, y: 19.05 },
                                origin: SeedVec2::default(),
                                definition_id: String::new(),
                                part_ids: Vec::new(),
                                board_id: Some(owner.board_id.clone()),
                                mirror: None,
                                rotation: None,
                                edge_gap: Some(SeedVec2 { x: 1.0, y: 1.0 }),
                                diode_direction: None,
                                row_layout: vec![],
                                column_layout: vec![],
                                anchor: None,
                                cells: vec![MatrixCell {
                                    shift: None,
                                    bend: None,
                                    deleted: false,
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
                            };
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
                    matrix.origin = boardstudio_model::model::Vec2::default();
                    let mut scenes = runtime
                        .project_matrices(accepted.document.revision, vec![matrix.clone()])?;
                    if scenes.len() != 1 {
                        return Err("Core returned an incomplete matrix placement preview.".into());
                    }
                    Ok::<_, String>(Some((matrix, definitions, scenes.remove(0))))
                }
                .await;
                if !alive.get() {
                    return;
                }
                if preparing.read().as_ref() != Some(&owner) {
                    return;
                }
                // A cancellation (Escape, scope or workspace change) may already
                // have ended the canvas mode without clearing `preparing` through
                // this owner; a late result must never re-arm placement then.
                if canvas_interaction.current() != Some(CanvasMode::PlacingMatrix)
                    || workspace() != Workspace::Layout
                    || scope_generation() != owner.scope_generation
                    || !owner_session_current(
                        &runtime,
                        &owner.scope,
                        &owner.board_id,
                        owner.snapshot_token,
                        owner.revision,
                    )
                {
                    preparing.set(None);
                    canvas_interaction.end(CanvasMode::PlacingMatrix);
                    return;
                }
                match result {
                    Ok(None) => {
                        preparing.set(None);
                        canvas_interaction.end(CanvasMode::PlacingMatrix);
                    }
                    Ok(Some((matrix, definitions, scene))) => {
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
                        canvas_interaction.end(CanvasMode::PlacingMatrix);
                    }
                }
            });
        }
    });

    let on_move = use_callback({
        let runtime = runtime.clone();

        let mut placement = placement;
        move |movement: MatrixPlacementMove| {
            if canvas_interaction.current() != Some(CanvasMode::PlacingMatrix)
                || workspace() != Workspace::Layout
                || scope_generation() != movement.owner.scope_generation
                || !movement.center.x.is_finite()
                || !movement.center.y.is_finite()
                || !owner_session_current(
                    &runtime,
                    &movement.owner.scope,
                    &movement.owner.board_id,
                    movement.owner.snapshot_token,
                    movement.owner.revision,
                )
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

        move |owner: MatrixPlacementOwner| {
            if waiting.read().is_some() {
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
                canvas_interaction.end(CanvasMode::PlacingMatrix);
            }
        }
    });

    let on_commit = use_callback({
        let runtime = runtime.clone();

        let mut placement = placement;
        let mut waiting = waiting;
        let mut error = error;
        move |movement: MatrixPlacementMove| {
            if canvas_interaction.current() != Some(CanvasMode::PlacingMatrix)
                || waiting.read().is_some()
                || workspace() != Workspace::Layout
                || scope_generation() != movement.owner.scope_generation
                || !movement.center.x.is_finite()
                || !movement.center.y.is_finite()
                || !owner_session_current(
                    &runtime,
                    &movement.owner.scope,
                    &movement.owner.board_id,
                    movement.owner.snapshot_token,
                    movement.owner.revision,
                )
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
            let ticket = runtime.resolve_edit(
                "layout-matrix-placement",
                Some("matrix placement".into()),
                place_matrix_resolver(active.matrix, active.definitions),
            );
            waiting.set(Some(PendingSettlement {
                ticket,
                owner: follow_up,
            }));
            placement.set(None);
            error.set(None);
            canvas_interaction.end(CanvasMode::PlacingMatrix);
        }
    });

    MatrixPlacementSession {
        cancel_owner: preparing
            .read()
            .clone()
            .or_else(|| placement.read().as_ref().map(|active| active.owner.clone())),
        placement: placement.read().clone(),
        busy: preparing.read().is_some() || waiting.read().is_some(),
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
