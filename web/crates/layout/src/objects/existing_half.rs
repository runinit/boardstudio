//! Owner-checked Layout action for linking copies of existing unpaired matrices.
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{
    EditOperation, Layout, LayoutMirrorLink, PartKind, ProjectDoc, Vec2,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::card::Card;
use boardstudio_web_ui_shared::ui::{
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldLabel},
    select::{Select, SelectOption},
};
use boardstudio_web_ui_shared::ui::{
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExistingHalfAction {
    Create,
}

/// Resolve mirroring existing halves against the accepted document at execution time. The
/// replacement document is built from the snapshot the resolver is handed; the identities
/// it needs were allocated when the user submitted, so the resolver stays pure.
fn mirror_existing_half_resolver(
    board_id: String,
    matrix_ids: Vec<String>,
    axis_x: f64,
    id_pool: Vec<String>,
) -> EditResolver {
    EditResolver::new(
        "layout-mirror-existing-half",
        move |accepted: &AcceptedSnapshot| {
            let mut pool = id_pool.iter();
            let mut id_factory = || {
                pool.next()
                    .cloned()
                    .ok_or_else(|| "Could not allocate a unique layout identity.".to_owned())
            };
            match prepare_existing_half(
                &accepted.document,
                &board_id,
                &matrix_ids,
                axis_x,
                &mut id_factory,
            ) {
                Ok(prepared) => Resolution::submit(
                    matrix_ids.clone(),
                    EditOperation::ReplaceDocument {
                        document: Box::new(prepared.document),
                    },
                ),
                Err(message) => Resolution::Retire(message),
            }
        },
    )
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
    let pending = use_hook(PendingEditSignals::<ExistingHalfAction>::new);
    let pending_create = use_signal(|| false);
    pending.bind_one_shot(ExistingHalfAction::Create, pending_create);
    let create_owner = use_signal(|| None::<ExistingHalfOwner>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut error = error;
            let pending = pending.clone();
            let mut create_owner = create_owner;
            move |(_, current_workspace, current_generation)| {
                settle_pending(
                    &runtime,
                    (current_workspace, current_generation, editor_instance_id),
                    &pending,
                    &mut create_owner,
                    &mut open,
                    &mut error,
                );
                if create_owner.read().is_none()
                    && open.read().as_ref().is_some_and(|owner| {
                        current_workspace != "Layout"
                            || owner.scope_generation != current_generation
                            || runtime.scope().as_ref() != Some(&owner.scope)
                    })
                {
                    open.set(None);
                    error.set(None);
                }
            }
        },
    ));

    let current_source = existing_half_source(&runtime, workspace());
    let options = current_source
        .as_ref()
        .map(|(snapshot, scope)| eligible_options(&snapshot.document, &scope.board_id))
        .unwrap_or_default();
    let can_open = !options.is_empty() && !pending_create();

    let on_open = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut open_id = open_id;
        let mut error = error;
        move |_| {
            if pending_create() {
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
        move |owner: ExistingHalfOwner| {
            if !pending_create() && open.read().as_ref() == Some(&owner) {
                open.set(None);
                error.set(None);
                super::focus_add_object();
            }
        }
    });

    let on_create = use_callback({
        let runtime = runtime.clone();
        let pending = pending.clone();
        let mut create_owner = create_owner;
        let mut error = error;
        move |request: ExistingHalfCreateRequest| {
            if pending_create()
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
            // Validate against the accepted document now so a bad request is explained
            // inline; the resolver repeats the check against whatever is accepted when the
            // edit runs.
            let mut id_pool = Vec::with_capacity(selected.len() * 3);
            for _ in 0..selected.len() * 3 {
                match crate::runtime::new_project_id() {
                    Ok(id) => id_pool.push(id),
                    Err(message) => {
                        error.set(Some(message));
                        return;
                    }
                }
            }
            let mut pool = id_pool.iter();
            let mut id_factory = || {
                pool.next()
                    .cloned()
                    .ok_or_else(|| "Could not allocate a unique layout identity.".to_owned())
            };
            if let Err(message) = prepare_existing_half(
                &snapshot.document,
                &scope.board_id,
                &selected,
                request.axis_x,
                &mut id_factory,
            ) {
                error.set(Some(message));
                return;
            }
            pending.begin_one_shot(
                &runtime,
                ExistingHalfAction::Create,
                "layout-mirror-existing-half",
                Some("mirror".into()),
                mirror_existing_half_resolver(
                    scope.board_id.clone(),
                    selected,
                    request.axis_x,
                    id_pool,
                ),
            );
            create_owner.set(Some(request.owner.clone()));
            error.set(None);
        }
    });

    let projection = open.read().as_ref().map(|owner| {
        let is_pending = pending_create();
        let editable =
            !is_pending && owner_is_current(&runtime, owner, workspace(), scope_generation());
        ExistingHalfProjection {
            owner: owner.clone(),
            editable,
            can_cancel: !is_pending,
            error: (!is_pending).then(&*error).flatten().or_else(|| {
                (!editable).then(|| {
                    "The accepted board changed. Cancel this setup and reopen it before mirroring."
                        .into()
                })
            }),
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
        if source
            .part_ids
            .iter()
            .any(|part_id| next.constraint_driving_part(part_id) != Ok(None))
        {
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
    }

    Ok(PreparedExistingHalf { document: next })
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
    (workspace, scope_generation, editor_instance_id): (&'static str, u64, u64),
    pending: &PendingEditSignals<ExistingHalfAction>,
    create_owner: &mut Signal<Option<ExistingHalfOwner>>,
    open: &mut Signal<Option<ExistingHalfOwner>>,
    error: &mut Signal<Option<String>>,
) {
    let Some(owner) = create_owner.read().clone() else {
        return;
    };
    let model = runtime.model();
    let owner_is_live = owner.editor_instance_id == editor_instance_id
        && workspace == "Layout"
        && scope_generation == owner.scope_generation;
    let results = pending.settle(owner_is_live, |_| String::new());
    let Some(result) = results.into_iter().next() else {
        return;
    };
    create_owner.set(None);
    match result {
        PendingEditResult::Landed { .. } => {
            let current_owner = workspace == "Layout"
                && scope_generation == owner.scope_generation
                && model.active_board_id == owner.scope.board_id
                && model.active_instance_id == owner.scope.instance_id;
            open.set(None);
            error.set(None);
            if current_owner {
                runtime.submit(Event::SetCamera {
                    operation_id: runtime.operation(),
                    center: Vec2::default(),
                    zoom: 1.0,
                });
            }
        }
        PendingEditResult::Failed { message, .. } => {
            if open.read().as_ref() == Some(&owner) {
                error.set(Some(message));
            }
        }
        PendingEditResult::Retired { .. } => {
            if open.read().as_ref() == Some(&owner) {
                open.set(None);
                error.set(None);
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
    let source_choice = use_memo(move || Some(default_choice()));
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
    let can_cancel = props.projection.can_cancel;
    let error = props.projection.error.clone();

    rsx! {
                section {
                    role: "region",
                    aria_label: "Mirror existing half",
                    onkeydown: move |event: KeyboardEvent| {
                        if event.key() == Key::Escape && can_cancel {
                            event.prevent_default();
                            event.stop_propagation();
                            cancel.call(owner_escape.clone());
                        }
                    },
                    Card { class: "layout-mirror-dock gap-4 p-4",
                        header { class: "m1-matrix-setup-heading",
                            h2 { "Mirror existing half" }
                        }
                        Callout { variant: CalloutVariant::Info,
                            "Keep the original in place and mirror its components too. Replace a component on one side when the halves need different hardware."
                        }
                        form { class: "grid gap-4", onsubmit: submit,
                            {
                                let control_label = "Source layouts".to_owned();
                                let control_aria_label = "Source layouts".to_owned();
                                let control_id = format!(
                                    "layout-choice-{}-{}",
                                    dioxus::core::current_scope_id().0,
                                    control_aria_label.replace(' ', "-"),
                                );
                                let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                                control_choices
                                    .push((
                                        "all".to_owned(),
                                        "All unpaired layouts on this board".to_owned(),
                                        false,
                                    ));
                                for option in owner.options.iter() {
                                    control_choices
                                        .push((
                                            option.matrix_id.to_string(),
                                            option.label.to_string(),
                                            false,
                                        ));
                                }
                                let on_value_change = EventHandler::new(move |choice: Option<String>| {
            let Some(choice) = choice else {
                return;
            };
            default_choice.set(choice)
        });
        rsx! {

                                    Field { class: "min-w-0",
                                        FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                        if control_choices.len() > 12 {
                                            Combobox::<String> { class: "max-[760px]:[&_[data-slot=combobox-input]]:min-h-control-touch",
                                                id: control_id.clone(),
                                                aria_label: control_aria_label.clone(),
                                                list_aria_label: control_aria_label.clone(),
                                                value: Some(source_choice.into()),
                                                disabled: !props.projection.editable,
                                                on_value_change,
                                                for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                    ComboboxOption::<String> {
                                                        index,
                                                        value: value.clone(),
                                                        text_value: label.clone(),
                                                        disabled: *disabled,
                                                        "{label}"
                                                    }
                                                }
                                            }
                                        } else {
                                            Select::<String> { class: "min-w-0 w-full [&_[data-placeholder]]:min-w-0 [&_[data-placeholder]]:truncate max-[760px]:[&_[data-slot=select-trigger]]:min-h-control-touch",
                                                id: control_id.clone(),
                                                aria_label: control_aria_label.clone(),
                                                value: Some(source_choice.into()),
                                                disabled: !props.projection.editable,
                                                on_value_change,
                                                for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                    SelectOption::<String> {
                                                        index,
                                                        value: value.clone(),
                                                        text_value: label.clone(),
                                                        disabled: *disabled,
                                                        "{label}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-1",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    Field {
                                        FieldLabel { html_for: control_input_id.clone(), "Mirror axis X (mm)" }
                                        InputGroup { class: "max-[760px]:h-control-touch",
                                            InputGroupInput { class: "max-[760px]:h-control-touch", aria_label: "Mirror axis X",
    r#type: InputType::Number,
    step: "any",
    required: true,
    value: "{axis_value}",
    disabled: !props.projection.editable,
    oninput: move |event: FormEvent| axis.set(event.value()),
    id: control_input_id.clone(), }
                                        }
                                    }
                                }
                            }
                            if let Some(message) = error {
                                Alert { class: "m1-mirrored-pair-error",
        variant: AlertVariant::Destructive,AlertDescription {  "{message}"  } }
                            }
                            footer { class: "m1-mirrored-pair-actions flex flex-wrap justify-end gap-2",
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    class: "max-[760px]:min-h-control-touch",
                                    button_type: "button",
                                    disabled: !props.projection.can_cancel,
                                    onclick: move |_| cancel.call(owner.clone()),
                                    "Cancel"
                                }
                                Button {
                                    variant: ButtonVariant::Default,
                                    class: "max-[760px]:min-h-control-touch",
                                    button_type: "submit",
                                    disabled: !can_create,
                                    "Create linked half"
                                }
                            }
                        }
                    }
                }
            }
}

#[cfg(test)]
mod adoption_tests {
    use super::*;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen::JsCast;

    fn form() -> Element {
        let owner = ExistingHalfOwner {
            editor_instance_id: 1,
            open_id: 1,
            scope_generation: 1,
            scope: Scope {
                session_epoch: boardstudio_application::SessionEpoch(1),
                document_id: "dock".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            snapshot_token: SnapshotToken(1),
            revision: 1,
            options: vec![ExistingHalfOption {
                matrix_id: "matrix".into(),
                label: "Left".into(),
            }],
            default_axis_x: 12.125,
        };
        let mut submitted = use_signal(|| None::<f64>);
        let mut cancellable = use_signal(|| true);
        let mut cancelled = use_signal(Vec::<u64>::new);
        rsx! {
            ExistingHalfSetup {
                projection: ExistingHalfProjection {
                    owner,
                    editable: true,
                    can_cancel: cancellable(),
                    error: None,
                },
                on_cancel: move |owner: ExistingHalfOwner| cancelled.write().push(owner.open_id),
                on_create: move |request: ExistingHalfCreateRequest| submitted.set(Some(request.axis_x)),
            }
            output { role: "status", aria_label: "Submitted axis", "{submitted:?}" }
            output { role: "status", aria_label: "Cancelled owners", "{cancelled:?}" }
            Button { onclick: move |_| cancellable.set(false), "Hold Mirror cancellation" }
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn mirror_existing_half_is_an_in_dock_view_with_its_own_precise_axis_draft() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(form),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("actual Mirror form mounts", || {
            by_role_name(&root, "spinbutton", "Mirror axis X").is_some()
        })
        .await;
        assert!(
            by_role_name(&root, "dialog", "Mirror existing half").is_none(),
            "Mirror must stay in the Objects dock, without dialog semantics"
        );
        assert!(
            by_role_name(&root, "region", "Mirror existing half").is_some(),
            "the dock view has a name"
        );
        let axis = by_role_name(&root, "spinbutton", "Mirror axis X")
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let create = by_role_name(&root, "button", "Create linked half").unwrap();
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        axis.set_value("");
        axis.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
        wait_until("empty local axis disables creation", || {
            create.has_attribute("disabled")
        })
        .await;
        axis.set_value("12.123456789");
        axis.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
        wait_until("valid local axis enables creation after render", || {
            !create.has_attribute("disabled")
        })
        .await;
        assert_eq!(axis.value(), "12.123456789");
        create.click();
        wait_until("precise draft submitted", || {
            by_role_name(&root, "status", "Submitted axis")
                .unwrap()
                .text_content()
                .as_deref()
                == Some("Some(12.123456789)")
        })
        .await;
        let escape = web_sys::KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        escape.set_cancelable(true);
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap();
        axis.dispatch_event(&event).unwrap();
        assert!(event.default_prevented());
        wait_until("Escape calls the same cancel owner once", || {
            by_role_name(&root, "status", "Cancelled owners")
                .unwrap()
                .text_content()
                .as_deref()
                == Some("[1]")
        })
        .await;
        by_role_name(&root, "button", "Hold Mirror cancellation")
            .unwrap()
            .click();
        wait_until("the pending owner disables Cancel", || {
            by_role_name(&root, "button", "Cancel")
                .unwrap()
                .has_attribute("disabled")
        })
        .await;
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap();
        axis.dispatch_event(&event).unwrap();
        assert!(!event.default_prevented());
        assert_eq!(
            by_role_name(&root, "status", "Cancelled owners")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("[1]")
        );
        assert_eq!(
            axis.value(),
            "12.123456789",
            "the actual local draft survives parent availability changes"
        );
        root.remove();
    }
}
