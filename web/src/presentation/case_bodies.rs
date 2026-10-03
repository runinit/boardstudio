//! Private authored Case body list and editor. Root owns the scoped edit adapter.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{CaseBody, CaseKind, Gasket, MountKind};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Props, Clone, PartialEq)]
pub(crate) struct CaseBodiesProps {
    pub board: Option<CaseBoardSummary>,
    pub bodies: Vec<CaseBody>,
    pub scope: Scope,
    pub editor_instance_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub generated_stack: bool,
    pub mismatch: Option<CaseMismatch>,
    pub editable: bool,
    pub feedback: Option<CaseBodyEditFeedback>,
    pub on_edit: EventHandler<CaseBodyRequest>,
    pub on_show_configured_board: EventHandler<String>,
}

#[derive(Clone)]
pub(crate) struct CaseBodyRequest {
    pub editor_instance_id: u64,
    pub request_id: u64,
    /// Stable scope/body/mount/field owner for numeric draft feedback; `None` for buttons.
    pub field_id: Option<String>,
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub edit: CaseBodyEdit,
}

#[derive(Clone, PartialEq)]
pub(crate) struct CaseBodyEditFeedback {
    pub editor_instance_id: u64,
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub request_id: u64,
    /// Echoed from the request so only the owning numeric draft consumes field feedback.
    pub field_id: Option<String>,
    pub state: CaseBodyEditState,
    pub message: Option<String>,
    pub created_body_id: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaseBodyEditState {
    Pending,
    Saved,
    Failed,
}

#[derive(Clone, PartialEq)]
pub(crate) struct CaseBoardSummary {
    pub id: String,
    pub name: String,
}

#[derive(Clone, PartialEq)]
pub(crate) struct CaseMismatch {
    pub board_id: String,
    pub board_name: String,
}

#[derive(Clone)]
pub(crate) enum CaseBodyEdit {
    AddBody,
    SetKind {
        body_id: String,
        kind: CaseKind,
    },
    SetThickness {
        body_id: String,
        value: f64,
    },
    SetClearance {
        body_id: String,
        value: f64,
    },
    SetZ {
        body_id: String,
        value: f64,
    },
    SetWallHeight {
        body_id: String,
        value: f64,
    },
    SetWallThickness {
        body_id: String,
        value: f64,
    },
    AddMount {
        body_id: String,
    },
    SetMountKind {
        body_id: String,
        mount_id: String,
        kind: MountKind,
    },
    SetMountX {
        body_id: String,
        mount_id: String,
        value: f64,
    },
    SetMountY {
        body_id: String,
        mount_id: String,
        value: f64,
    },
    SetMountHoleDiameter {
        body_id: String,
        mount_id: String,
        value: f64,
    },
    SetMountBossDiameter {
        body_id: String,
        mount_id: String,
        value: f64,
    },
    SetMountHeight {
        body_id: String,
        mount_id: String,
        value: f64,
    },
    RemoveMount {
        body_id: String,
        mount_id: String,
    },
    SetGasket {
        body_id: String,
        gasket: Option<Gasket>,
    },
}

#[derive(Clone, PartialEq)]
struct RequestIdentity {
    editor_instance_id: u64,
    scope: Scope,
    snapshot_token: SnapshotToken,
    revision: u64,
    request_id: u64,
    field_id: Option<String>,
}

use super::case_viewer::{BodySelection, CaseSelection};

#[derive(Clone, Copy, PartialEq, Eq)]
enum NumberRule {
    Finite,
    Nonnegative,
    Positive,
}

#[derive(Clone, PartialEq)]
struct CaseNumberCommit {
    field_id: String,
    value: f64,
}

#[derive(Clone, Default)]
struct CaseBodyDisclosureState {
    owner: String,
    mounts_open: bool,
    mounts_chosen: bool,
    gasket_open: bool,
    gasket_chosen: bool,
}

#[component]
pub(crate) fn CaseBodies(props: CaseBodiesProps) -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let pending_request = use_signal(|| None::<RequestIdentity>);
    let submission_busy = use_signal(|| false);
    let mut disclosures = use_signal(CaseBodyDisclosureState::default);
    let selected_body = use_context::<CaseSelection>().body;

    let board = props
        .board
        .as_ref()
        .filter(|board| board.id == props.scope.board_id);
    let bodies = props
        .bodies
        .iter()
        .filter(|body| board.is_some_and(|board| body.board_id == board.id))
        .collect::<Vec<_>>();
    let selection = selected_body
        .read()
        .clone()
        .filter(|selected| selected.scope == props.scope);
    let selected = selection.as_ref().and_then(|selection| {
        bodies
            .iter()
            .copied()
            .find(|body| body.id == selection.body_id)
    });
    let active_body = selected.or_else(|| bodies.first().copied());

    let emit_edit_with_field: Rc<dyn Fn(CaseBodyEdit, Option<String>)> = {
        let on_edit = props.on_edit;
        let scope = props.scope.clone();
        let editor_instance_id = props.editor_instance_id;
        let snapshot_token = props.snapshot_token;
        let revision = props.revision;
        Rc::new(move |edit, field_id| {
            let mut request_sequence = request_sequence;
            let mut pending_request = pending_request;
            let mut submission_busy = submission_busy;
            if submission_busy() {
                return;
            }
            let request_id = request_sequence()
                .checked_add(1)
                .expect("Case edit request identity exhausted");
            request_sequence.set(request_id);
            let identity = RequestIdentity {
                editor_instance_id,
                scope: scope.clone(),
                snapshot_token,
                revision,
                request_id,
                field_id: field_id.clone(),
            };
            pending_request.set(Some(identity));
            submission_busy.set(true);
            on_edit.call(CaseBodyRequest {
                editor_instance_id,
                request_id,
                field_id,
                scope: scope.clone(),
                snapshot_token,
                revision,
                edit,
            });
        })
    };
    let emit_edit = {
        let emit = emit_edit_with_field.clone();
        Rc::new(move |edit| emit(edit, None)) as Rc<dyn Fn(CaseBodyEdit)>
    };

    let feedback = props.feedback.clone().filter(|feedback| {
        let pending_guard = pending_request.read();
        let Some(pending) = pending_guard.as_ref() else {
            return false;
        };
        props.editor_instance_id == pending.editor_instance_id
            && props.scope == pending.scope
            && feedback.editor_instance_id == pending.editor_instance_id
            && feedback.scope == pending.scope
            && feedback.snapshot_token == pending.snapshot_token
            && feedback.revision == pending.revision
            && feedback.request_id == pending.request_id
            && feedback.field_id == pending.field_id
    });

    let terminal_feedback = feedback
        .as_ref()
        .map(|feedback| (request_identity(feedback), feedback.state));
    let pending_for_effect = pending_request;
    let mut busy_for_effect = submission_busy;
    use_effect(use_reactive((&terminal_feedback,), move |(terminal,)| {
        if let Some((identity, state)) = terminal
            && matches!(state, CaseBodyEditState::Saved | CaseBodyEditState::Failed)
            && pending_for_effect.read().as_ref() == Some(&identity)
        {
            busy_for_effect.set(false);
        }
    }));

    let mut selection_for_effect = selected_body;
    let scope = props.scope.clone();
    let saved_add = feedback
        .as_ref()
        .filter(|feedback| feedback.state == CaseBodyEditState::Saved)
        .and_then(|feedback| feedback.created_body_id.clone());
    use_effect(use_reactive((&saved_add, &scope), {
        move |(created_body_id, scope)| {
            if let Some(body_id) = created_body_id {
                selection_for_effect.set(Some(BodySelection {
                    scope: scope.clone(),
                    body_id: body_id.clone(),
                }));
            }
        }
    }));

    if props.generated_stack {
        return rsx! {
            section { class: "m1-case-bodies-generated", "aria-label": "Authored case bodies",
                p { class: "m1-case-bodies-note", role: "status",
                    "Generated assembly preview · {bodies.len()} authored case bodies remain saved. Disable the mechanical stack to preview and edit them."
                }
            }
        };
    }

    let can_edit = props.editable && !submission_busy();
    let can_add = can_edit && board.is_some();
    let body_id = active_body.map(|body| body.id.clone());
    let editor_key = body_id
        .as_ref()
        .map(|body_id| {
            case_field_id(
                props.editor_instance_id,
                &props.scope,
                body_id,
                None,
                "editor",
            )
        })
        .unwrap_or_default();
    let mounts_default_open = active_body.is_some_and(|body| {
        body.mounts
            .as_ref()
            .is_some_and(|mounts| !mounts.is_empty())
    });
    let gasket_default_open = active_body.is_some_and(|body| body.gasket.is_some());
    let disclosure_state = disclosures();
    let (mounts_open, gasket_open) = if disclosure_state.owner == editor_key {
        (
            if disclosure_state.mounts_chosen {
                disclosure_state.mounts_open
            } else {
                mounts_default_open
            },
            if disclosure_state.gasket_chosen {
                disclosure_state.gasket_open
            } else {
                gasket_default_open
            },
        )
    } else {
        (mounts_default_open, gasket_default_open)
    };
    let emit_edit = emit_edit.clone();
    let global_feedback = feedback
        .as_ref()
        .filter(|feedback| feedback.field_id.is_none());
    let global_feedback_pending =
        global_feedback.is_some_and(|feedback| feedback.state == CaseBodyEditState::Pending);
    let global_feedback_saved =
        global_feedback.is_some_and(|feedback| feedback.state == CaseBodyEditState::Saved);
    let global_feedback_error = global_feedback
        .filter(|feedback| feedback.state == CaseBodyEditState::Failed)
        .and_then(|feedback| feedback.message.clone());

    rsx! {
        section { class: "m1-case-bodies", "aria-label": "Authored case bodies",
            if let Some(mismatch) = props.mismatch.as_ref() {
                section { class: "m1-case-mismatch", "aria-label": "Mechanical stack board mismatch",
                    h3 { "Mechanical stack belongs to {mismatch.board_name}" }
                    p { "This board shows its authored case bodies. Select the configured board to inspect that mechanical stack." }
                    button {
                        r#type: "button",
                        disabled: mismatch.board_id.is_empty(),
                        onclick: {
                            let on_show = props.on_show_configured_board;
                            let board_id = mismatch.board_id.clone();
                            move |_| on_show.call(board_id.clone())
                        },
                        "Show configured board"
                    }
                }
            }
            header { class: "m1-case-bodies-heading",
                h2 { "Case stack" }
                button {
                    class: "m1-case-new-body",
                    r#type: "button",
                    disabled: !can_add,
                    onclick: move |_| emit_edit(CaseBodyEdit::AddBody),
                    "+ New case body"
                }
            }
            if !bodies.is_empty() {
                div { class: "m1-case-body-list", role: "group", "aria-label": "Case bodies",
                    for (index, body) in bodies.iter().enumerate() {
                        {
                            let id = body.id.clone();
                            let is_active = active_body.is_some_and(|active| active.id == body.id);
                            let kind = case_kind_label(&body.kind);
                            let order = format!("{:02}", index + 1);
                            let mut selected_body = selected_body;
                            let scope = props.scope.clone();
                            rsx! {
                                button {
                                    key: "{id}",
                                    class: if is_active { "m1-case-body-tab is-active" } else { "m1-case-body-tab" },
                                    r#type: "button",
                                    disabled: !can_edit,
                                    "aria-pressed": is_active,
                                    onclick: move |_| selected_body.set(Some(BodySelection {
                                        scope: scope.clone(),
                                        body_id: id.clone(),
                                    })),
                                    span { "{order}" }
                                    strong { "{body.name}" }
                                    small { "{kind}" }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(body) = active_body {
                if let Some(body_id) = body_id {
                    div { key: "{editor_key}", class: "m1-case-body-editor",
                        label { class: "m1-case-select", "Body type"
                            select {
                                value: case_kind_value(&body.kind),
                                disabled: !can_edit,
                                onchange: {
                                    let submit = emit_edit.clone();
                                    let body_id = body_id.clone();
                                    move |event: FormEvent| {
                                        if let Some(kind) = parse_case_kind(&event.value()) {
                                            submit(CaseBodyEdit::SetKind { body_id: body_id.clone(), kind });
                                        }
                                    }
                                },
                                option { value: "plate", "Plate" }
                                option { value: "tray", "Tray" }
                                option { value: "lid", "Lid" }
                            }
                        }
                        div { class: "m1-case-measures",
                            CaseNumberField {
                                label: "Thickness", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "thickness"), value: body.thickness, unit: "mm", rule: NumberRule::Positive,
                                editable: can_edit, submission_busy, feedback: feedback.clone(),
                                on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetThickness { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                            }
                            CaseNumberField {
                                label: "Clearance", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "clearance"), value: body.clearance, unit: "mm", rule: NumberRule::Nonnegative,
                                editable: can_edit, submission_busy, feedback: feedback.clone(),
                                on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetClearance { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                            }
                            CaseNumberField {
                                label: "Z offset", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "z"), value: body.z.unwrap_or(0.0), unit: "mm", rule: NumberRule::Finite,
                                editable: can_edit, submission_busy, feedback: feedback.clone(),
                                on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetZ { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                            }
                            if !matches!(&body.kind, CaseKind::Plate) {
                                CaseNumberField {
                                    label: "Wall height", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "wall-height"), value: body.wall_height.unwrap_or(14.0), unit: "mm", rule: NumberRule::Positive,
                                    editable: can_edit, submission_busy, feedback: feedback.clone(),
                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetWallHeight { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                                }
                                CaseNumberField {
                                    label: "Wall thickness", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "wall-thickness"), value: body.wall_thickness.unwrap_or(2.0), unit: "mm", rule: NumberRule::Positive,
                                    editable: can_edit, submission_busy, feedback: feedback.clone(),
                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetWallThickness { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                                }
                            }
                        }
                        details { class: "m1-case-subsection", "aria-label": "Mounting", open: mounts_open,
                            summary { class: "m1-case-subsection-heading", onclick: {
                                let mut disclosures = disclosures;
                                let owner = editor_key.clone();
                                move |event: MouseEvent| {
                                    event.prevent_default();
                                    let current = disclosures();
                                    let current_open = if current.owner == owner && current.mounts_chosen { current.mounts_open } else { mounts_default_open };
                                    disclosures.set(CaseBodyDisclosureState {
                                        owner: owner.clone(),
                                        mounts_open: !current_open,
                                        mounts_chosen: true,
                                        gasket_open: if current.owner == owner && current.gasket_chosen { current.gasket_open } else { gasket_default_open },
                                        gasket_chosen: current.owner == owner && current.gasket_chosen,
                                    });
                                }
                            },
                                h3 { "Mounting" }
                                small { "{body.mounts.as_ref().map_or(0, Vec::len)} mounts" }
                            }
                            div { class: "m1-case-subsection-content",
                                button {
                                    r#type: "button", disabled: !can_edit,
                                    onclick: { let submit = emit_edit.clone(); let body_id = body_id.clone(); move |_| submit(CaseBodyEdit::AddMount { body_id: body_id.clone() }) },
                                    "+ Add mount"
                                }
                            for (index, mount) in body.mounts.iter().flatten().enumerate() {
                                {
                                    let mount_id = mount.id.clone();
                                    let mount_label = format!("Mount {}", index + 1);
                                    let submit_kind = emit_edit.clone();
                                    let submit_remove = emit_edit.clone();
                                    let body_id = body_id.clone();
                                    rsx! {
                                        fieldset { key: "{mount_id}", class: "m1-case-mount-editor",
                                            legend { "{mount_label}" }
                                            button {
                                                r#type: "button", disabled: !can_edit,
                                                "aria-label": "Remove {mount_label}",
                                                onclick: { let submit = submit_remove.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |_| submit(CaseBodyEdit::RemoveMount { body_id: body_id.clone(), mount_id: mount_id.clone() }) },
                                                "Remove"
                                            }
                                            label { class: "m1-case-select", "Type"
                                                select {
                                                    value: mount_kind_value(&mount.kind), disabled: !can_edit,
                                                    onchange: { let submit = submit_kind.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |event: FormEvent| {
                                                        if let Some(kind) = parse_mount_kind(&event.value()) { submit(CaseBodyEdit::SetMountKind { body_id: body_id.clone(), mount_id: mount_id.clone(), kind }); }
                                                    } },
                                                    option { value: "hole", "Hole" }
                                                    option { value: "boss", "Boss" }
                                                }
                                            }
                                            div { class: "m1-case-measures",
                                                CaseNumberField {
                                                    label: "X position", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-x"), value: mount.at.x, unit: "mm", rule: NumberRule::Finite,
                                                    editable: can_edit, submission_busy, feedback: feedback.clone(),
                                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountX { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                }
                                                CaseNumberField {
                                                    label: "Y position", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-y"), value: mount.at.y, unit: "mm", rule: NumberRule::Finite,
                                                    editable: can_edit, submission_busy, feedback: feedback.clone(),
                                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountY { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                }
                                                CaseNumberField {
                                                    label: "Hole diameter", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-hole-diameter"), value: mount.hole_diameter, unit: "mm", rule: NumberRule::Positive,
                                                    editable: can_edit, submission_busy, feedback: feedback.clone(),
                                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountHoleDiameter { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                }
                                                if matches!(&mount.kind, MountKind::Boss) {
                                                    CaseNumberField {
                                                        label: "Boss diameter", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-boss-diameter"), value: mount.boss_diameter.unwrap_or(5.0), unit: "mm", rule: NumberRule::Positive,
                                                        editable: can_edit, submission_busy, feedback: feedback.clone(),
                                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountBossDiameter { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                    }
                                                    CaseNumberField {
                                                        label: "Height", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-height"), value: mount.height.unwrap_or(5.0), unit: "mm", rule: NumberRule::Positive,
                                                        editable: can_edit, submission_busy, feedback: feedback.clone(),
                                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountHeight { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            }
                        }
                        details { class: "m1-case-subsection", "aria-label": "Gasket channel", open: gasket_open,
                            summary { class: "m1-case-subsection-heading", onclick: {
                                let mut disclosures = disclosures;
                                let owner = editor_key.clone();
                                move |event: MouseEvent| {
                                    event.prevent_default();
                                    let current = disclosures();
                                    let current_open = if current.owner == owner && current.gasket_chosen { current.gasket_open } else { gasket_default_open };
                                    disclosures.set(CaseBodyDisclosureState {
                                        owner: owner.clone(),
                                        mounts_open: if current.owner == owner && current.mounts_chosen { current.mounts_open } else { mounts_default_open },
                                        mounts_chosen: current.owner == owner && current.mounts_chosen,
                                        gasket_open: !current_open,
                                        gasket_chosen: true,
                                    });
                                }
                            },
                                h3 { "Gasket channel" }
                                small { if body.gasket.is_some() { "Configured" } else { "Optional" } }
                            }
                            div { class: "m1-case-subsection-content",
                                if body.gasket.is_some() {
                                    button { r#type: "button", disabled: !can_edit,
                                        onclick: { let submit = emit_edit.clone(); let body_id = body_id.clone(); move |_| submit(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: None }) },
                                        "Remove"
                                    }
                                } else {
                                    button { r#type: "button", disabled: !can_edit,
                                        onclick: { let submit = emit_edit.clone(); let body_id = body_id.clone(); move |_| submit(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: Some(Gasket { inset: 2.0, width: 2.0, depth: 1.5 }) }) },
                                        "+ Add gasket"
                                    }
                                }
                            if let Some(gasket) = body.gasket.as_ref() {
                                div { class: "m1-case-measures m1-case-gasket-measures",
                                    CaseNumberField {
                                        label: "Inset", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "gasket-inset"), value: gasket.inset, unit: "mm", rule: NumberRule::Nonnegative,
                                        editable: can_edit, submission_busy, feedback: feedback.clone(),
                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let gasket = gasket.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: Some(Gasket { inset: change.value, ..gasket.clone() }) }, Some(change.field_id)) }
                                    }
                                    CaseNumberField {
                                        label: "Width", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "gasket-width"), value: gasket.width, unit: "mm", rule: NumberRule::Positive,
                                        editable: can_edit, submission_busy, feedback: feedback.clone(),
                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let gasket = gasket.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: Some(Gasket { width: change.value, ..gasket.clone() }) }, Some(change.field_id)) }
                                    }
                                    CaseNumberField {
                                        label: "Depth", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "gasket-depth"), value: gasket.depth, unit: "mm", rule: NumberRule::Positive,
                                        editable: can_edit, submission_busy, feedback: feedback.clone(),
                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let gasket = gasket.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: Some(Gasket { depth: change.value, ..gasket.clone() }) }, Some(change.field_id)) }
                                    }
                                }
                            }
                            }
                        }
                    }
                }
            } else if board.is_some() {
                p { class: "m1-case-bodies-empty", role: "status", "Add a plate, tray, or lid to begin the case stack." }
            } else {
                p { class: "m1-case-bodies-empty", role: "status", "Add a board before creating a case body." }
            }
            if global_feedback_pending {
                p { class: "m1-case-edit-pending", role: "status", "Saving case body… Other changes are paused; drafts are preserved." }
            }
            if global_feedback_saved {
                p { class: "m1-case-edit-saved", role: "status", "Case body saved." }
            }
            if let Some(error) = global_feedback_error {
                p { class: "m1-case-edit-error", role: "alert", "{error}" }
            }
            if submission_busy() {
                p { class: "m1-case-edit-pending", role: "status", "A Case edit is still saving. Other edits are paused; numeric drafts are preserved." }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct CaseNumberFieldProps {
    label: &'static str,
    field_id: String,
    value: f64,
    unit: &'static str,
    rule: NumberRule,
    editable: bool,
    submission_busy: Signal<bool>,
    feedback: Option<CaseBodyEditFeedback>,
    on_commit: EventHandler<CaseNumberCommit>,
}

#[component]
fn CaseNumberField(props: CaseNumberFieldProps) -> Element {
    let draft = use_signal(|| props.value.to_string());
    let error = use_signal(|| None::<String>);
    let submitted_draft = use_signal(|| None::<String>);
    let mut ignored_feedback_request = use_signal(|| None::<RequestIdentity>);
    let dirty = use_signal(|| false);
    let blocked_attempt = use_signal(|| false);

    let accepted_value = props.value;
    let saved_ack = props
        .feedback
        .as_ref()
        .filter(|feedback| {
            feedback.field_id.as_deref() == Some(props.field_id.as_str())
                && feedback.state == CaseBodyEditState::Saved
        })
        .map(request_identity);
    let consumed_saved_ack = use_signal(|| None::<RequestIdentity>);
    let saved_ack_pending = saved_ack
        .as_ref()
        .is_some_and(|identity| consumed_saved_ack.read().as_ref() != Some(identity));
    let mut draft_for_effect = draft;
    let mut error_for_effect = error;
    let mut submitted_for_effect = submitted_draft;
    let mut dirty_for_effect = dirty;
    let mut consumed_ack_for_effect = consumed_saved_ack;
    use_effect(use_reactive(
        (&accepted_value, &saved_ack_pending, &saved_ack),
        move |(value, ack_pending, ack)| {
            if ack_pending {
                if let Some(identity) = ack {
                    consumed_ack_for_effect.set(Some(identity.clone()));
                }
                draft_for_effect.set(value.to_string());
                error_for_effect.set(None);
                submitted_for_effect.set(None);
                dirty_for_effect.set(false);
            } else if !dirty_for_effect() {
                draft_for_effect.set(value.to_string());
                error_for_effect.set(None);
                submitted_for_effect.set(None);
            }
        },
    ));

    let owned_feedback = props
        .feedback
        .as_ref()
        .filter(|feedback| feedback.field_id.as_deref() == Some(props.field_id.as_str()));
    let feedback_identity = owned_feedback.map(request_identity);
    let feedback_key = owned_feedback.map(|feedback| (request_identity(feedback), feedback.state));
    let mut submitted_for_feedback = submitted_draft;
    let mut ignored_for_feedback = ignored_feedback_request;
    use_effect(use_reactive((&feedback_identity,), move |(_identity,)| {
        ignored_for_feedback.set(None);
    }));
    use_effect(use_reactive((&feedback_key,), move |(key,)| {
        if key.is_some_and(|(_, state)| state == CaseBodyEditState::Failed) {
            submitted_for_feedback.set(None);
        }
    }));

    let commit: Rc<dyn Fn()> = Rc::new({
        let on_commit = props.on_commit;
        let field_id = props.field_id.clone();
        let accepted = props.value;
        let rule = props.rule;
        let submission_busy = props.submission_busy;
        move || {
            let mut draft = draft;
            let mut error = error;
            let mut submitted_draft = submitted_draft;
            let mut blocked_attempt = blocked_attempt;
            let mut dirty = dirty;
            if submission_busy() {
                blocked_attempt.set(true);
                return;
            }
            let value_text = draft();
            if submitted_draft().as_deref() == Some(value_text.as_str()) {
                return;
            }
            let Ok(value) = value_text.trim().parse::<f64>() else {
                error.set(Some(number_error(rule).to_string()));
                return;
            };
            if !value.is_finite()
                || (rule == NumberRule::Nonnegative && value < 0.0)
                || (rule == NumberRule::Positive && value <= 0.0)
            {
                error.set(Some(number_error(rule).to_string()));
                return;
            }
            error.set(None);
            blocked_attempt.set(false);
            submitted_draft.set(Some(value_text));
            if value != accepted {
                dirty.set(true);
                on_commit.call(CaseNumberCommit {
                    field_id: field_id.clone(),
                    value,
                });
            } else {
                draft.set(accepted.to_string());
                dirty.set(false);
            }
        }
    });

    let min = match props.rule {
        NumberRule::Finite => None,
        NumberRule::Nonnegative => Some("0"),
        NumberRule::Positive => Some("0.001"),
    };
    let error_text = error();
    let invalid = error_text.is_some();
    let feedback = props
        .feedback
        .as_ref()
        .filter(|feedback| feedback.field_id.as_deref() == Some(props.field_id.as_str()));
    let feedback_error = feedback
        .filter(|feedback| feedback.state == CaseBodyEditState::Failed)
        .filter(|feedback| ignored_feedback_request().as_ref() != Some(&request_identity(feedback)))
        .and_then(|feedback| feedback.message.clone());
    let feedback_pending =
        feedback.is_some_and(|feedback| feedback.state == CaseBodyEditState::Pending);
    let feedback_saved =
        feedback.is_some_and(|feedback| feedback.state == CaseBodyEditState::Saved);
    let input_feedback_identity = feedback.map(request_identity);
    let escape_feedback_identity = input_feedback_identity.clone();

    rsx! {
        label { class: if invalid { "m1-case-number has-error" } else { "m1-case-number" },
            span { "{props.label}" }
            span { class: "m1-case-number-input",
                input {
                    r#type: "number",
                    step: "0.1",
                    min: min,
                    value: "{draft}",
                    disabled: !props.editable || (props.submission_busy)(),
                    "aria-invalid": invalid,
                    oninput: move |event: FormEvent| {
                        let mut draft = draft;
                        let mut dirty = dirty;
                        let mut blocked_attempt = blocked_attempt;
                        let mut submitted_draft = submitted_draft;
                        let mut error = error;
                        let mut ignored_feedback_request = ignored_feedback_request;
                        draft.set(event.value());
                        dirty.set(true);
                        blocked_attempt.set(false);
                        submitted_draft.set(None);
                        error.set(None);
                        ignored_feedback_request.set(input_feedback_identity.clone());
                    },
                    onblur: {
                        let commit = commit.clone();
                        move |_| commit()
                    },
                    onkeydown: {
                        let commit = commit.clone();
                        let mut dirty = dirty;
                        let mut blocked_attempt = blocked_attempt;
                        let mut draft = draft;
                        let mut error = error;
                        let mut submitted_draft = submitted_draft;
                        let accepted = props.value;
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            if key == "Enter" {
                                event.prevent_default();
                                commit();
                            } else if key == "Escape" {
                                event.prevent_default();
                                draft.set(accepted.to_string());
                                dirty.set(false);
                                blocked_attempt.set(false);
                                error.set(None);
                                submitted_draft.set(None);
                                ignored_feedback_request.set(escape_feedback_identity.clone());
                            }
                        }
                    }
                }
                small { "{props.unit}" }
            }
            if let Some(error) = error_text {
                small { class: "m1-case-field-error", role: "alert", "{error}" }
            } else if let Some(error) = feedback_error {
                small { class: "m1-case-field-error", role: "alert", "{error}" }
            } else if feedback_pending {
                small { role: "status", "Saving…" }
            } else if feedback_saved {
                small { role: "status", "Saved" }
            }
            if blocked_attempt() {
                small { class: "m1-case-field-error", role: "status", "Another Case edit is saving. Your draft is preserved; retry after it finishes." }
            }
        }
    }
}

fn number_error(rule: NumberRule) -> &'static str {
    match rule {
        NumberRule::Positive => "Enter a value above 0.",
        NumberRule::Nonnegative => "Enter 0 or greater.",
        NumberRule::Finite => "Enter a valid number.",
    }
}

fn case_field_id(
    editor_instance_id: u64,
    scope: &Scope,
    body_id: &str,
    mount_id: Option<&str>,
    field: &str,
) -> String {
    format!(
        "{editor_instance_id}|{scope:?}|{body_id}|{}|{field}",
        mount_id.unwrap_or_default()
    )
}

fn request_identity(feedback: &CaseBodyEditFeedback) -> RequestIdentity {
    RequestIdentity {
        editor_instance_id: feedback.editor_instance_id,
        scope: feedback.scope.clone(),
        snapshot_token: feedback.snapshot_token,
        revision: feedback.revision,
        request_id: feedback.request_id,
        field_id: feedback.field_id.clone(),
    }
}

fn case_kind_label(kind: &CaseKind) -> &'static str {
    match kind {
        CaseKind::Plate => "plate",
        CaseKind::Tray => "tray",
        CaseKind::Lid => "lid",
    }
}

fn case_kind_value(kind: &CaseKind) -> &'static str {
    case_kind_label(kind)
}

fn parse_case_kind(value: &str) -> Option<CaseKind> {
    match value {
        "plate" => Some(CaseKind::Plate),
        "tray" => Some(CaseKind::Tray),
        "lid" => Some(CaseKind::Lid),
        _ => None,
    }
}

fn mount_kind_value(kind: &MountKind) -> &'static str {
    match kind {
        MountKind::Hole => "hole",
        MountKind::Boss => "boss",
    }
}

fn parse_mount_kind(value: &str) -> Option<MountKind> {
    match value {
        "hole" => Some(MountKind::Hole),
        "boss" => Some(MountKind::Boss),
        _ => None,
    }
}
