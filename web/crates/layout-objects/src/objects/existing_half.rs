//! Owner-checked Layout action for linking copies of existing unpaired matrices.
use super::matrix_preparation::PendingSettlement;
use crate::mirror_existing_half::prepare_existing_half;
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::{EditOperation, PartKind, ProjectDoc, Vec2};
use boardstudio_web_runtime::operation_outcomes::Settlement;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::TextField;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::card::Card;
use boardstudio_web_ui_shared::ui::{
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldLabel},
    input::InputType,
    select::{Select, SelectOption},
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
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
) -> ExistingHalfMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let open_id = use_signal(|| 0_u64);
    let open = use_signal(|| None::<ExistingHalfOwner>);
    let error = use_signal(|| None::<String>);
    // The committed mirror is observed through its ticket; the waiting entry is the
    // pending state. Availability derives from it, so no separate disabled flag is kept.
    let waiting = use_signal(|| None::<PendingSettlement<ExistingHalfOwner>>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut error = error;
            let mut waiting = waiting;
            move |(_, current_workspace, current_generation)| {
                settle_pending(
                    &runtime,
                    (current_workspace, current_generation, editor_instance_id),
                    &mut waiting,
                    &mut open,
                    &mut error,
                );
                if waiting.read().is_none()
                    && open.read().as_ref().is_some_and(|owner| {
                        current_workspace != Workspace::Layout
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
    let can_open = !options.is_empty() && waiting.read().is_none();

    let on_open = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut open_id = open_id;
        let mut error = error;
        move |_| {
            if waiting.read().is_some() {
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
            if waiting.read().is_none() && open.read().as_ref() == Some(&owner) {
                open.set(None);
                error.set(None);
                super::focus_add_object();
            }
        }
    });

    let on_create = use_callback({
        let runtime = runtime.clone();
        let mut waiting = waiting;
        let mut error = error;
        move |request: ExistingHalfCreateRequest| {
            if waiting.read().is_some()
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
            let ticket = runtime.resolve_edit(
                "layout-mirror-existing-half",
                Some("mirror".into()),
                mirror_existing_half_resolver(
                    scope.board_id.clone(),
                    selected,
                    request.axis_x,
                    id_pool,
                ),
            );
            waiting.set(Some(PendingSettlement {
                ticket,
                owner: request.owner,
            }));
            error.set(None);
        }
    });

    let projection = open.read().as_ref().map(|owner| {
        let is_pending = waiting.read().is_some();
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
        visible: workspace() == Workspace::Layout,
        can_open,
        on_open,
        on_cancel,
        on_create,
    }
}

fn eligible_options(document: &ProjectDoc, board_id: &str) -> Vec<ExistingHalfOption> {
    let mut options = Vec::new();
    for matrix in document
        .matrices
        .iter()
        .filter(|matrix| super::matrix_visible_on_board(document, board_id, &matrix.id))
    {
        let layout = document.layouts.iter().find(|layout| {
            layout.matrix_ids.iter().any(|id| id == &matrix.id) && layout.board_id == board_id
        });
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
                boardstudio_model::model::Vec2 {
                    x: -cap.x / 2.0,
                    y: -cap.y / 2.0,
                },
                boardstudio_model::model::Vec2 {
                    x: cap.x / 2.0,
                    y: -cap.y / 2.0,
                },
                boardstudio_model::model::Vec2 {
                    x: cap.x / 2.0,
                    y: cap.y / 2.0,
                },
                boardstudio_model::model::Vec2 {
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
    workspace: Workspace,
) -> Option<(AcceptedSnapshot, Scope)> {
    if workspace != Workspace::Layout {
        return None;
    }
    let model = runtime.model();
    let scope = runtime.scope()?;
    let snapshot = model.accepted.as_ref()?;
    let durability_ready = matches!(
        model.durability,
        Durability::Saved { revision } | Durability::Saving { revision }
            if revision == snapshot.document.revision
    );
    if runtime.scope().as_ref() != Some(&scope)
        || snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || model.lifecycle != Lifecycle::Ready
        || !durability_ready
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
    workspace: Workspace,
    scope_generation: u64,
) -> bool {
    if workspace != Workspace::Layout
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
        && matches!(
            model.durability,
            Durability::Saved { revision } | Durability::Saving { revision }
                if revision == snapshot.document.revision
        )
        && model.display_preview.is_none()
        && model.gesture.is_none()
}

fn settle_pending(
    runtime: &Rc<Runtime>,
    (workspace, scope_generation, editor_instance_id): (Workspace, u64, u64),
    waiting: &mut Signal<Option<PendingSettlement<ExistingHalfOwner>>>,
    open: &mut Signal<Option<ExistingHalfOwner>>,
    error: &mut Signal<Option<String>>,
) {
    let Some((owner, settlement)) = PendingSettlement::finish(waiting, |owner| {
        owner.editor_instance_id == editor_instance_id
            && workspace == Workspace::Layout
            && scope_generation == owner.scope_generation
    }) else {
        return;
    };
    let model = runtime.model();
    match settlement {
        Settlement::Retired => {
            if open.read().as_ref() == Some(&owner) {
                open.set(None);
                error.set(None);
            }
        }
        Settlement::Landed { .. } => {
            let current_owner = workspace == Workspace::Layout
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
        Settlement::Failed { message } => {
            if open.read().as_ref() == Some(&owner) {
                error.set(Some(message));
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
    let mut axis = use_signal(|| owner.default_axis_x.to_string());
    let axis_value = axis();
    let parsed_axis = axis_value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite());
    let axis_invalid = parsed_axis.is_none() && !axis_value.trim().is_empty();
    let has_sources = !owner.options.is_empty();
    let can_create = props.projection.editable && has_sources && parsed_axis.is_some();
    let on_create = props.on_create;
    let owner_submit = owner.clone();
    let choice_submit = choice.clone();
    let options = owner.options.clone();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        let Some(axis_x) = parsed_axis else {
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
                    header { class: "flex items-baseline justify-between gap-2",
                        h2 { class: "m-0! text-[16px]!", "Mirror existing half" }
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
                                TextField {
                                    id: control_input_id,
                                    label: "Mirror axis X (mm)".to_owned(),
                                    aria_label: "Mirror axis X",
                                    r#type: InputType::Number,
                                    value: axis_value,
                                    disabled: !props.projection.editable,
                                    description: (!props.projection.editable).then(|| "Placement is unavailable".to_owned()),
                                    error: axis_invalid.then(|| "Enter a finite mirror axis in millimetres.".to_owned()),
                                    oninput: move |event: FormEvent| axis.set(event.value()),
                                }
                            }
                        }
                        if let Some(message) = error {
                            Alert {
    variant: AlertVariant::Destructive,AlertDescription {  "{message}"  } }
                        }
                        footer { class: "flex flex-wrap justify-end gap-2",
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
