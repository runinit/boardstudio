//! Owner-checked Layout action for linking copies of existing unpaired matrices.
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Layout, LayoutMirrorLink, PartKind, ProjectDoc, Vec2,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExistingHalfOption {
    pub matrix_id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistingHalfOwner {
    editor_instance_id: u64,
    open_id: u64,
    scope_generation: u64,
    scope: Scope,
    snapshot_token: SnapshotToken,
    revision: u64,
    options: Vec<ExistingHalfOption>,
    default_axis_x: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistingHalfProjection {
    pub owner: ExistingHalfOwner,
    pub editable: bool,
    pub can_cancel: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistingHalfCreateRequest {
    pub owner: ExistingHalfOwner,
    pub matrix_ids: Vec<String>,
    pub axis_x: f64,
}

#[derive(Clone, PartialEq)]
pub struct ExistingHalfMount {
    pub projection: Option<ExistingHalfProjection>,
    pub visible: bool,
    pub can_open: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<ExistingHalfOwner>,
    pub on_create: EventHandler<ExistingHalfCreateRequest>,
}

#[derive(Clone)]
struct PendingExistingHalf {
    owner: ExistingHalfOwner,
    outcome: OutcomeSlot,
    target_matrix_ids: Vec<String>,
    target_layout_ids: Vec<String>,
}

pub fn use_existing_half(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> ExistingHalfMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let open_id = use_signal(|| 0_u64);
    let open = use_signal(|| None::<ExistingHalfOwner>);
    let error = use_signal(|| None::<String>);
    let status = use_signal(|| None::<String>);
    let pending = use_signal(|| None::<PendingExistingHalf>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut error = error;
            let mut status = status;
            let mut pending = pending;
            move |(_, current_workspace, current_generation)| {
                settle_pending(
                    &runtime,
                    current_workspace,
                    current_generation,
                    &mut pending,
                    &mut open,
                    &mut error,
                    &mut status,
                );
                if pending.read().is_none()
                    && open.read().as_ref().is_some_and(|owner| {
                        current_workspace != "Layout"
                            || owner.scope_generation != current_generation
                            || runtime.scope().as_ref() != Some(&owner.scope)
                    })
                {
                    open.set(None);
                    error.set(None);
                    status.set(None);
                }
            }
        },
    ));

    let current_source = existing_half_source(&runtime, workspace());
    let options = current_source
        .as_ref()
        .map(|(snapshot, scope)| eligible_options(&snapshot.document, &scope.board_id))
        .unwrap_or_default();
    let can_open = !options.is_empty() && pending.read().is_none();

    let on_open = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut open_id = open_id;
        let mut error = error;
        let mut status = status;
        move |_| {
            if pending.read().is_some() {
                return;
            }
            let Some((snapshot, scope)) = existing_half_source(&runtime, workspace()) else {
                error.set(Some(
                    "Open a saved, editable board in Layout before mirroring a half.".into(),
                ));
                return;
            };
            let options = eligible_options(&snapshot.document, &scope.board_id);
            if options.is_empty() {
                error.set(Some("There are no unpaired layouts on this board.".into()));
                return;
            }
            let Some(next_open_id) = open_id().checked_add(1) else {
                error.set(Some("Could not create a new mirror setup.".into()));
                return;
            };
            open_id.set(next_open_id);
            error.set(None);
            status.set(None);
            let default_axis_x = default_axis_x(&snapshot, &scope.board_id);
            open.set(Some(ExistingHalfOwner {
                editor_instance_id,
                open_id: next_open_id,
                scope_generation: scope_generation(),
                scope,
                snapshot_token: snapshot.token,
                revision: snapshot.document.revision,
                options,
                default_axis_x,
            }));
        }
    });

    let on_cancel = use_callback({
        let mut open = open;
        let mut error = error;
        let mut status = status;
        move |owner: ExistingHalfOwner| {
            if pending.read().is_none() && open.read().as_ref() == Some(&owner) {
                open.set(None);
                error.set(None);
                status.set(None);
            }
        }
    });

    let on_create = use_callback({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut error = error;
        let mut status = status;
        move |request: ExistingHalfCreateRequest| {
            if pending.read().is_some()
                || open.read().as_ref() != Some(&request.owner)
                || request.owner.editor_instance_id != editor_instance_id
            {
                return;
            }
            let current_workspace = workspace();
            let current_generation = scope_generation();
            if !owner_is_current(
                &runtime,
                &request.owner,
                current_workspace,
                current_generation,
            ) {
                error.set(Some(
                    "The accepted board changed. Cancel this setup and reopen it before mirroring."
                        .into(),
                ));
                status.set(None);
                return;
            }
            if !request.axis_x.is_finite() {
                error.set(Some("Enter a finite mirror axis.".into()));
                return;
            }
            let Some((snapshot, scope)) = existing_half_source(&runtime, current_workspace) else {
                error.set(Some("The accepted board is no longer editable.".into()));
                return;
            };
            if scope != request.owner.scope
                || snapshot.token != request.owner.snapshot_token
                || snapshot.document.revision != request.owner.revision
            {
                error.set(Some(
                    "The accepted board changed. Cancel this setup and reopen it before mirroring."
                        .into(),
                ));
                return;
            }
            let selected = if request.matrix_ids.is_empty()
                || request.matrix_ids.iter().any(|id| {
                    !request
                        .owner
                        .options
                        .iter()
                        .any(|option| option.matrix_id == *id)
                }) {
                error.set(Some("Choose an available source layout.".into()));
                return;
            } else {
                request.matrix_ids.clone()
            };
            let mut id_factory = crate::runtime::new_project_id;
            let prepared = match prepare_existing_half(
                &snapshot.document,
                &scope.board_id,
                &selected,
                request.axis_x,
                &mut id_factory,
            ) {
                Ok(prepared) => prepared,
                Err(message) => {
                    error.set(Some(message));
                    status.set(None);
                    return;
                }
            };
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            pending.set(Some(PendingExistingHalf {
                owner: request.owner.clone(),
                outcome,
                target_matrix_ids: prepared.target_matrix_ids.clone(),
                target_layout_ids: prepared.target_layout_ids.clone(),
            }));
            error.set(None);
            status.set(Some("Creating linked half…".into()));
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id: format!("mirror-existing-half-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: selected,
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(prepared.document),
                    },
                },
            });
        }
    });

    let projection = open.read().as_ref().map(|owner| {
        let is_pending = pending.read().is_some();
        let editable = owner_is_current(&runtime, owner, workspace(), scope_generation());
        ExistingHalfProjection {
            owner: owner.clone(),
            editable,
            can_cancel: !is_pending,
            error: (!is_pending).then(|| error()).flatten().or_else(|| {
                (!editable).then(|| {
                    "The accepted board changed. Cancel this setup and reopen it before mirroring."
                        .into()
                })
            }),
            status: status(),
        }
    });

    ExistingHalfMount {
        projection,
        visible: workspace() == "Layout",
        can_open,
        on_open,
        on_cancel,
        on_create,
    }
}

#[derive(Clone)]
struct PreparedExistingHalf {
    document: ProjectDoc,
    target_matrix_ids: Vec<String>,
    target_layout_ids: Vec<String>,
}

fn prepare_existing_half(
    document: &ProjectDoc,
    board_id: &str,
    matrix_ids: &[String],
    axis_x: f64,
    new_id: &mut impl FnMut() -> Result<String, String>,
) -> Result<PreparedExistingHalf, String> {
    if matrix_ids.is_empty() || !axis_x.is_finite() {
        return Err("Select a layout and a finite mirror axis.".into());
    }
    let mut next = document.clone();
    let mut target_matrix_ids = Vec::with_capacity(matrix_ids.len());
    let mut target_layout_ids = Vec::with_capacity(matrix_ids.len());
    let mut used_ids = document
        .boards
        .iter()
        .map(|item| item.id.clone())
        .chain(document.matrices.iter().map(|item| item.id.clone()))
        .chain(document.layouts.iter().map(|item| item.id.clone()))
        .chain(document.parts.iter().map(|item| item.id.clone()))
        .collect::<std::collections::BTreeSet<_>>();

    for matrix_id in matrix_ids {
        let source = next
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)
            .cloned()
            .ok_or_else(|| "The selected matrix no longer exists.".to_string())?;
        if source.board_id.as_deref() != Some(board_id) {
            return Err("The layout must belong to the active board.".into());
        }
        if source.mirror == Some(boardstudio_core::model::Mirror::Y) {
            return Err(
                "Y-mirrored layouts cannot be linked. Use an independent layout or an X-mirrored source."
                    .into(),
            );
        }
        if source.part_ids.iter().any(|part_id| {
            next.constraints
                .iter()
                .any(|constraint| constraint.target() == part_id)
        }) {
            return Err("Remove key placement constraints before linking this layout.".into());
        }
        let existing = next
            .layouts
            .iter()
            .find(|layout| layout.matrix_id == source.id && layout.board_id == board_id)
            .cloned();
        if existing.as_ref().is_some_and(|layout| {
            layout.mirror_link.is_some()
                || next.layouts.iter().any(|candidate| {
                    candidate
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                })
        }) {
            return Err("This layout already belongs to a linked pair.".into());
        }
        let source_layout = if let Some(layout) = existing {
            layout
        } else {
            let id = fresh_id(&mut used_ids, new_id)?;
            let name = source
                .name
                .as_deref()
                .filter(|name| !name.is_empty())
                .unwrap_or("Original half")
                .to_owned();
            let layout = Layout {
                id,
                name,
                board_id: board_id.to_owned(),
                matrix_id: source.id.clone(),
                part_ids: Vec::new(),
                mirror_link: None,
            };
            next.layouts.push(layout.clone());
            layout
        };

        let target_matrix_id = fresh_id(&mut used_ids, new_id)?;
        let target_layout_id = fresh_id(&mut used_ids, new_id)?;
        let target_name = format!("{} · mirrored", source_layout.name);
        let mut target_matrix = source.clone();
        target_matrix.id = target_matrix_id.clone();
        target_matrix.name = Some(target_name.clone());
        target_matrix.part_ids.clear();
        for cell in &mut target_matrix.cells {
            for assembly in &mut cell.assemblies {
                assembly.offset.x = -assembly.offset.x;
                assembly.rotation = assembly.rotation.map(|rotation| -rotation);
            }
        }
        next.matrices.push(target_matrix);
        next.layouts.push(Layout {
            id: target_layout_id.clone(),
            name: target_name,
            board_id: board_id.to_owned(),
            matrix_id: target_matrix_id.clone(),
            part_ids: Vec::new(),
            mirror_link: Some(LayoutMirrorLink {
                source_id: source_layout.id,
                axis_x,
            }),
        });
        target_matrix_ids.push(target_matrix_id);
        target_layout_ids.push(target_layout_id);
    }

    Ok(PreparedExistingHalf {
        document: next,
        target_matrix_ids,
        target_layout_ids,
    })
}

fn fresh_id(
    used: &mut std::collections::BTreeSet<String>,
    new_id: &mut impl FnMut() -> Result<String, String>,
) -> Result<String, String> {
    for _ in 0..8 {
        let id = new_id()?;
        if !id.trim().is_empty() && used.insert(id.clone()) {
            return Ok(id);
        }
    }
    Err("Could not allocate a unique layout identity.".into())
}

fn eligible_options(document: &ProjectDoc, board_id: &str) -> Vec<ExistingHalfOption> {
    let mut options = Vec::new();
    for matrix in document
        .matrices
        .iter()
        .filter(|matrix| super::matrix_visible_on_board(document, board_id, &matrix.id))
    {
        let layout = document
            .layouts
            .iter()
            .find(|layout| layout.matrix_id == matrix.id && layout.board_id == board_id);
        let linked = layout.is_some_and(|layout| {
            layout.mirror_link.is_some()
                || document.layouts.iter().any(|candidate| {
                    candidate
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                })
        });
        if !linked {
            options.push(ExistingHalfOption {
                matrix_id: matrix.id.clone(),
                label: matrix
                    .name
                    .as_deref()
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("Matrix {}", options.len() + 1)),
            });
        }
    }
    options
}

fn default_axis_x(snapshot: &AcceptedSnapshot, board_id: &str) -> f64 {
    let document = &snapshot.document;
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return 12.0;
    };
    let member_ids = board
        .part_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let mut max_x: f64 = 0.0;
    for part in document
        .parts
        .iter()
        .filter(|part| member_ids.contains(&part.id))
    {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id);
        let pose = snapshot
            .scene
            .transforms
            .iter()
            .find(|transform| transform.id == part.id)
            .map(|transform| &transform.pose)
            .unwrap_or(&part.pose);
        let cap = definition
            .filter(|definition| matches!(&definition.kind, PartKind::Switch))
            .and_then(|definition| part.keycap.or(definition.keycap));
        let corners = if let Some(cap) = cap {
            vec![
                boardstudio_core::model::Vec2 {
                    x: -cap.x / 2.0,
                    y: -cap.y / 2.0,
                },
                boardstudio_core::model::Vec2 {
                    x: cap.x / 2.0,
                    y: -cap.y / 2.0,
                },
                boardstudio_core::model::Vec2 {
                    x: cap.x / 2.0,
                    y: cap.y / 2.0,
                },
                boardstudio_core::model::Vec2 {
                    x: -cap.x / 2.0,
                    y: cap.y / 2.0,
                },
            ]
        } else {
            definition.map_or_else(Vec::new, |definition| definition.courtyard.clone())
        };
        let radians = pose.rotation.to_radians();
        let (sin, cos) = radians.sin_cos();
        for point in corners {
            let x = pose.at.x + point.x * cos - point.y * sin;
            if x.is_finite() {
                max_x = max_x.max(x);
            }
        }
    }
    max_x + 12.0
}

fn existing_half_source(
    runtime: &Runtime,
    workspace: &'static str,
) -> Option<(AcceptedSnapshot, Scope)> {
    if workspace != "Layout" {
        return None;
    }
    let model = runtime.model();
    let scope = runtime.scope()?;
    let snapshot = model.accepted.as_ref()?;
    if runtime.scope().as_ref() != Some(&scope)
        || snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
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
    Some((snapshot.clone(), scope))
}

fn owner_is_current(
    runtime: &Runtime,
    owner: &ExistingHalfOwner,
    workspace: &'static str,
    scope_generation: u64,
) -> bool {
    if workspace != "Layout"
        || owner.scope_generation != scope_generation
        || runtime.scope().as_ref() != Some(&owner.scope)
    {
        return false;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    snapshot.session_epoch == owner.scope.session_epoch
        && snapshot.document.id == owner.scope.document_id
        && snapshot.document.revision == owner.revision
        && snapshot.token == owner.snapshot_token
        && model.active_board_id == owner.scope.board_id
        && model.active_instance_id == owner.scope.instance_id
        && model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: snapshot.document.revision,
            })
        && model.display_preview.is_none()
        && model.gesture.is_none()
}

fn settle_pending(
    runtime: &Rc<Runtime>,
    workspace: &'static str,
    scope_generation: u64,
    pending: &mut Signal<Option<PendingExistingHalf>>,
    open: &mut Signal<Option<ExistingHalfOwner>>,
    error: &mut Signal<Option<String>>,
    status: &mut Signal<Option<String>>,
) {
    let Some(waiting) = pending.read().clone() else {
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
        pending.set(None);
        open.set(None);
        error.set(None);
        status.set(None);
        return;
    }
    match outcome {
        TerminalOutcome::Completed => {
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let saved = model.durability
                == (Durability::Saved {
                    revision: snapshot.document.revision,
                });
            let created = waiting.target_matrix_ids.iter().all(|id| {
                snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == *id)
            }) && waiting.target_layout_ids.iter().all(|id| {
                snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == *id)
            });
            let current_owner = workspace == "Layout"
                && scope_generation == waiting.owner.scope_generation
                && runtime.scope().as_ref() == Some(&waiting.owner.scope)
                && model.active_board_id == waiting.owner.scope.board_id
                && model.active_instance_id == waiting.owner.scope.instance_id
                && model.lifecycle == Lifecycle::Ready
                && model.display_preview.is_none()
                && model.gesture.is_none();
            if saved
                && created
                && snapshot.token != waiting.owner.snapshot_token
                && snapshot.document.revision == waiting.owner.revision.saturating_add(1)
                && current_owner
            {
                pending.set(None);
                open.set(None);
                error.set(None);
                status.set(None);
                runtime.submit(Event::SetCamera {
                    operation_id: runtime.operation(),
                    center: Vec2::default(),
                    zoom: 1.0,
                });
            } else if snapshot.document.revision > waiting.owner.revision.saturating_add(1)
                || workspace != "Layout"
                || scope_generation != waiting.owner.scope_generation
            {
                pending.set(None);
                open.set(None);
                error.set(None);
                status.set(None);
            }
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => {
            pending.set(None);
            if open.read().as_ref() == Some(&waiting.owner) {
                error.set(Some(message));
                status.set(None);
            }
        }
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            pending.set(None);
            if open.read().as_ref() == Some(&waiting.owner) {
                error.set(Some(
                    "The mirror operation did not complete. Reopen the form and try again.".into(),
                ));
                status.set(None);
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct ExistingHalfProps {
    pub projection: ExistingHalfProjection,
    pub on_cancel: EventHandler<ExistingHalfOwner>,
    pub on_create: EventHandler<ExistingHalfCreateRequest>,
}

#[component]
pub fn ExistingHalfSetup(props: ExistingHalfProps) -> Element {
    let owner = props.projection.owner.clone();
    let mut default_choice = use_signal(|| "all".to_owned());
    let choice = default_choice();
    let default_axis = format!("{:.2}", owner.default_axis_x);
    let mut axis = use_signal(|| default_axis.clone());
    let axis_value = axis();
    let axis_number = axis_value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite());
    let has_sources = !owner.options.is_empty();
    let can_create = props.projection.editable && has_sources && axis_number.is_some();
    let on_create = props.on_create;
    let owner_submit = owner.clone();
    let axis_submit = axis_value.clone();
    let choice_submit = choice.clone();
    let options = owner.options.clone();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        let Some(axis_x) = axis_submit
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
        else {
            return;
        };
        let matrix_ids = if choice_submit == "all" {
            options
                .iter()
                .map(|option| option.matrix_id.clone())
                .collect()
        } else {
            options
                .iter()
                .find(|option| option.matrix_id == choice_submit)
                .map(|option| vec![option.matrix_id.clone()])
                .unwrap_or_default()
        };
        on_create.call(ExistingHalfCreateRequest {
            owner: owner_submit.clone(),
            matrix_ids,
            axis_x,
        });
    };
    let cancel = props.on_cancel;
    let owner_escape = owner.clone();
    let error = props.projection.error.clone();
    let status = props.projection.status.clone();

    rsx! {
        div {
            role: "dialog",
            aria_label: "Mirror existing half",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == "Escape" && props.projection.can_cancel {
                    event.prevent_default();
                    event.stop_propagation();
                    cancel.call(owner_escape.clone());
                }
            },
            section { class: "m1-matrix-setup m1-mirrored-pair-setup",
                header { class: "m1-matrix-setup-heading", h2 { "Mirror existing half" } }
                p { class: "m1-mirrored-pair-intro", "Keep the original in place and mirror its components too. Replace a component on one side when the halves need different hardware." }
                form { onsubmit: submit,
                    label { "Source layouts"
                        select {
                            aria_label: "Source layouts",
                            value: "{choice}",
                            disabled: !props.projection.editable,
                            onchange: move |event| default_choice.set(event.value()),
                            option { value: "all", "All unpaired layouts on this board" }
                            for option in owner.options.iter() {
                                option { key: "{option.matrix_id}", value: "{option.matrix_id}", "{option.label}" }
                            }
                        }
                    }
                    label { "Mirror axis X (mm)"
                        input {
                            aria_label: "Mirror axis X",
                            r#type: "number",
                            step: "any",
                            required: true,
                            value: "{axis_value}",
                            disabled: !props.projection.editable,
                            oninput: move |event| axis.set(event.value()),
                        }
                    }
                    if let Some(message) = error { p { class: "m1-mirrored-pair-error", role: "alert", "{message}" } }
                    if let Some(message) = status { p { class: "m1-mirrored-pair-status", role: "status", "{message}" } }
                    footer { class: "m1-mirrored-pair-actions",
                        button { class: "wb-secondary", r#type: "button", disabled: !props.projection.can_cancel, onclick: move |_| cancel.call(owner.clone()), "Cancel" }
                        button { class: "wb-primary", r#type: "submit", disabled: !can_create, "Create linked half" }
                    }
                }
            }
        }
    }
}
