//! Owner-checked Layout action for linking copies of existing unpaired matrices.
use crate::mirror_existing_half::prepare_existing_half;
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::{EditOperation, PartKind, ProjectDoc, Vec2};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{NumericField, SelectField};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::card::Card;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExistingHalfOption {
    pub matrix_id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistingHalfOwner {
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
    let open = use_signal(|| None::<ExistingHalfOwner>);
    let error = use_signal(|| None::<String>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut error = error;
            move |(_, current_workspace, _)| {
                // Commits land synchronously, so the open entry only needs
                // reactive cleanup when its scope leaves the Layout workspace.
                if open.read().as_ref().is_some_and(|owner| {
                    current_workspace != Workspace::Layout
                        || runtime.scope().as_ref() != Some(&owner.scope)
                }) {
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
    let can_open = !options.is_empty();

    let on_open = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut error = error;
        move |_| {
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
            error.set(None);
            let default_axis_x = default_axis_x(&snapshot, &scope.board_id);
            open.set(Some(ExistingHalfOwner {
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
            if open.read().as_ref() == Some(&owner) {
                open.set(None);
                error.set(None);
                super::focus_add_object();
            }
        }
    });

    let on_create = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut error = error;
        move |request: ExistingHalfCreateRequest| {
            if open.read().as_ref() != Some(&request.owner) {
                return;
            }
            let current_workspace = workspace();
            if !owner_is_current(&runtime, &request.owner, current_workspace) {
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
            // The edit commits synchronously; handle its outcome inline and close
            // the setup at once instead of tracking a pending settlement.
            let owner = request.owner.clone();
            match runtime.commit(
                "layout-mirror-existing-half",
                mirror_existing_half_resolver(
                    scope.board_id.clone(),
                    selected,
                    request.axis_x,
                    id_pool,
                ),
            ) {
                Ok(_) => {
                    open.set(None);
                    error.set(None);
                    let model = runtime.model();
                    if current_workspace == Workspace::Layout
                        && runtime.scope().as_ref() == Some(&owner.scope)
                        && model.active_board_id == owner.scope.board_id
                        && model.active_instance_id == owner.scope.instance_id
                    {
                        runtime.submit(Event::SetCamera {
                            operation_id: runtime.operation(),
                            center: Vec2::default(),
                            zoom: 1.0,
                        });
                    }
                }
                Err(message) => {
                    if open.read().as_ref() == Some(&owner) {
                        error.set(Some(message));
                    }
                }
            }
        }
    });

    let projection = open.read().as_ref().map(|owner| {
        let editable = owner_is_current(&runtime, owner, workspace());
        ExistingHalfProjection {
            owner: owner.clone(),
            editable,
            can_cancel: true,
            error: error.read().clone().or_else(|| {
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
    super::mirror_links::unlinked_board_matrices(document, board_id)
        .into_iter()
        .map(|(matrix_id, label)| ExistingHalfOption { matrix_id, label })
        .collect()
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

fn owner_is_current(runtime: &Runtime, owner: &ExistingHalfOwner, workspace: Workspace) -> bool {
    if workspace != Workspace::Layout || runtime.scope().as_ref() != Some(&owner.scope) {
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
    let mut axis = use_signal(|| owner.default_axis_x.to_string());
    let parsed_axis = axis()
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite());
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
                        SelectField {
                            label: "Source layouts".to_owned(),
                            value: choice.clone(),
                            options: std::iter::once((
                                "all".to_owned(),
                                "All unpaired layouts on this board".to_owned(),
                            ))
                            .chain(owner.options.iter().map(|option| {
                                (option.matrix_id.clone(), option.label.clone())
                            }))
                            .collect(),
                            id: format!(
                                "layout-choice-{}-Source-layouts",
                                dioxus::core::current_scope_id().0,
                            ),
                            aria_label: "Source layouts",
                            disabled: !props.projection.editable,
                            on_commit: move |next: String| default_choice.set(next),
                        }
                        NumericField {
                            label: "Mirror axis X".to_owned(),
                            unit: "mm",
                            value: parsed_axis.unwrap_or(owner.default_axis_x),
                            step: 0.1,
                            id: format!(
                                "layout-input-{}-1",
                                dioxus::core::current_scope_id().0,
                            ),
                            aria_label: "Mirror axis X",
                            disabled_reason: (!props.projection.editable).then(|| "Placement is unavailable".to_owned()),
                            on_commit: move |next: f64| axis.set(next.to_string()),
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
