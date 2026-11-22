//! Private authored Case body list and editor. Root owns the scoped edit adapter.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{CaseBody, CaseKind, Gasket, MountKind, Vec2};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    callout::Callout,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput, InputGroupText},
    select::{Select, SelectOption},
    toggle_group::{ToggleGroup, ToggleItem},
};
use dioxus::prelude::*;
use std::{cell::Cell, collections::HashSet, rc::Rc};

#[derive(Props, Clone, PartialEq)]
pub struct CaseBodiesProps {
    pub board: Option<CaseBoardSummary>,
    pub bodies: Vec<CaseBody>,
    pub scope: Scope,
    pub editor_instance_id: u64,
    pub request_sequence: Signal<u64>,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub generated_stack: bool,
    pub mismatch: Option<CaseMismatch>,
    pub editable: bool,
    pub feedback: Vec<CaseBodyEditFeedback>,
    pub on_edit: EventHandler<CaseBodyRequest>,
    pub on_show_configured_board: EventHandler<String>,
}

#[derive(Clone)]
pub struct CaseBodyRequest {
    pub editor_instance_id: u64,
    pub request_id: u64,
    /// Stable field or one-shot action owner for feedback.
    pub field_id: Option<String>,
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub edit: CaseBodyEdit,
}

impl CaseBodyEdit {
    pub(crate) fn action_id(&self) -> Option<String> {
        match self {
            Self::AddBody => Some("action:add-body".into()),
            Self::AddMount { body_id } => Some(format!("action:body:{body_id}:add-mount")),
            Self::SetGasket { body_id, .. } => Some(format!("action:body:{body_id}:gasket")),
            Self::RemoveMount { body_id, mount_id } => {
                Some(format!("action:body:{body_id}:mount:{mount_id}:remove"))
            }
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct CaseBodyEditFeedback {
    pub editor_instance_id: u64,
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub request_id: u64,
    /// Echoed from the request so only the owning control consumes feedback.
    pub field_id: Option<String>,
    /// The settled edit's failure, shown inline at its field.
    pub failure: Option<String>,
    pub created_body_id: Option<String>,
}

impl CaseBodyEditFeedback {
    /// The edit landed: the field shows the accepted value again.
    fn landed(&self) -> bool {
        self.failure.is_none()
    }
}

#[derive(Clone, PartialEq)]
pub struct CaseBoardSummary {
    pub id: String,
    pub name: String,
}

#[derive(Clone, PartialEq)]
pub struct CaseMismatch {
    pub board_id: String,
    pub board_name: String,
}

#[derive(Clone)]
pub enum CaseBodyEdit {
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
    SetMountPosition {
        body_id: String,
        mount_id: String,
        at: Vec2,
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
    SetGasketInset {
        body_id: String,
        value: f64,
    },
    SetGasketWidth {
        body_id: String,
        value: f64,
    },
    SetGasketDepth {
        body_id: String,
        value: f64,
    },
    SetGasket {
        body_id: String,
        gasket: Option<Gasket>,
    },
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
pub fn CaseBodies(props: CaseBodiesProps) -> Element {
    let request_sequence = props.request_sequence;
    let disclosures = use_signal(CaseBodyDisclosureState::default);
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
            let request_id = request_sequence()
                .checked_add(1)
                .expect("Case edit request identity exhausted");
            request_sequence.set(request_id);
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
        Rc::new(move |edit: CaseBodyEdit| {
            let field_id = edit.action_id();
            emit(edit, field_id);
        }) as Rc<dyn Fn(CaseBodyEdit)>
    };

    let feedback = props
        .feedback
        .iter()
        .filter(|entry| {
            entry.editor_instance_id == props.editor_instance_id && entry.scope == props.scope
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut selection_for_effect = selected_body;
    let scope = props.scope.clone();
    let saved_add = feedback
        .iter()
        .rev()
        .find(|feedback| feedback.landed())
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
            section { class: "flex min-h-0 min-w-0 flex-1 flex-col content-start justify-start gap-3.5 overflow-auto bg-card px-3 py-4 text-foreground", "aria-label": "Authored case bodies",
                Callout { "Generated assembly preview · {bodies.len()} authored case bodies remain saved. Disable the mechanical stack to preview and edit them." }
            }
        };
    }

    let can_edit = props.editable;
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
        .iter()
        .filter(|feedback| {
            feedback
                .field_id
                .as_deref()
                .is_none_or(|id| id.starts_with("action:"))
        })
        .max_by_key(|feedback| feedback.request_id);
    let global_feedback_error = global_feedback.and_then(|feedback| feedback.failure.clone());

    rsx! {
        section { class: "flex min-h-0 min-w-0 flex-1 flex-col content-start gap-3.5 overflow-auto bg-card px-3 py-4 text-foreground [&_h2]:m-0 [&_h3]:m-0", "aria-label": "Authored case bodies",
            if let Some(mismatch) = props.mismatch.as_ref() {
                section { class: "grid gap-2 rounded-[5px] border border-border bg-surface p-3", "aria-label": "Mechanical stack board mismatch",
                    h3 { "Mechanical stack belongs to {mismatch.board_name}" }
                    Callout { "This board shows its authored case bodies. Select the configured board to inspect that mechanical stack." }
                    Button {
                        variant: ButtonVariant::Secondary, button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
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
            header { class: "flex items-center justify-between gap-2 border-b border-border pb-2.5",
                h2 { "Case stack" }
                Button {
                    variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !can_add,
                    onclick: { let submit = emit_edit.clone(); move |_| submit(CaseBodyEdit::AddBody) },
                    "+ New case body"
                }
            }
            if !bodies.is_empty() {
                {
                    let tab_ids: Vec<String> = bodies.iter().map(|body| body.id.clone()).collect();
                    let active_index = active_body
                        .and_then(|active| tab_ids.iter().position(|id| id == &active.id));
                    let tabs_scope = props.scope.clone();
                    let tabs_selection = selected_body;
                    rsx! {
                        ToggleGroup {
                            aria_label: "Case bodies",
                            class: "grid grid-cols-[repeat(auto-fit,minmax(112px,1fr))] gap-1.5",
                            style: "width: 100%",
                            disabled: !can_edit,
                            pressed: Some(active_index.map(|index| HashSet::from([index])).unwrap_or_default()),
                            on_pressed_change: move |next: HashSet<usize>| {
                                if let Some(index) = next.iter().next()
                                    && let Some(id) = tab_ids.get(*index)
                                {
                                    let mut selected_body = tabs_selection;
                                    selected_body.set(Some(BodySelection {
                                        scope: tabs_scope.clone(),
                                        body_id: id.clone(),
                                    }));
                                }
                            },
                            for (index, body) in bodies.iter().enumerate() {
                                {
                                    let id = body.id.clone();
                                    let kind = case_kind_label(&body.kind);
                                    let order = format!("{:02}", index + 1);
                                    let name = body.name.clone();
                                    rsx! {
                                        ToggleItem {
                                            key: "{id}",
                                            index,
                                            class: "grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2 gap-y-0.5 text-left [&>span]:row-span-2 [&>span]:text-muted-foreground [&>span]:tabular-nums [&_strong]:truncate [&_small]:text-xs [&_small]:uppercase [&_small]:text-muted-foreground max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                            disabled: !can_edit,
                                            span { "{order}" }
                                            strong { "{name}" }
                                            small { "{kind}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(body) = active_body {
                if let Some(body_id) = body_id {
                    div { key: "{editor_key}", class: "grid min-w-0 gap-2.5 border-t border-border py-3",
                        CaseStaticSelect {
                            label: "Body type",
                            value: (case_kind_value(&body.kind)).to_string(),
                            disabled: !can_edit,
                            on_change: {
                                    let submit = emit_edit.clone();
                                    let body_id = body_id.clone();
                                    move |value: Option<String>| {
                                        if let Some(kind) = value.as_deref().and_then(parse_case_kind) {
                                            submit(CaseBodyEdit::SetKind { body_id: body_id.clone(), kind });
                                        }
                                    }
                                },
                            SelectOption::<String> { index: 0usize, value: "plate".to_owned(), text_value: "Plate".to_owned(), "Plate" }
                                SelectOption::<String> { index: 1usize, value: "tray".to_owned(), text_value: "Tray".to_owned(), "Tray" }
                                SelectOption::<String> { index: 2usize, value: "lid".to_owned(), text_value: "Lid".to_owned(), "Lid" }
                        }
                        div { class: "grid grid-cols-2 gap-2.5 max-[760px]:grid-cols-1",
                            CaseNumberField {
                                label: "Thickness", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "thickness"), value: body.thickness, unit: "mm", rule: NumberRule::Positive,
                                editable: can_edit,
                                on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetThickness { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                            }
                            CaseNumberField {
                                label: "Clearance", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "clearance"), value: body.clearance, unit: "mm", rule: NumberRule::Nonnegative,
                                editable: can_edit,
                                on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetClearance { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                            }
                            CaseNumberField {
                                label: "Z offset", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "z"), value: body.z.unwrap_or(0.0), unit: "mm", rule: NumberRule::Finite,
                                editable: can_edit,
                                on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetZ { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                            }
                            if !matches!(&body.kind, CaseKind::Plate) {
                                CaseNumberField {
                                    label: "Wall height", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "wall-height"), value: body.wall_height.unwrap_or(14.0), unit: "mm", rule: NumberRule::Positive,
                                    editable: can_edit,
                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetWallHeight { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                                }
                                CaseNumberField {
                                    label: "Wall thickness", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "wall-thickness"), value: body.wall_thickness.unwrap_or(2.0), unit: "mm", rule: NumberRule::Positive,
                                    editable: can_edit,
                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetWallThickness { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                                }
                            }
                        }
                        Collapsible { class: "grid min-w-0 gap-2.5 border-t border-border py-3", keep_mounted: true, open: Some(mounts_open),
                            on_open_change: {
                                let mut disclosures = disclosures;
                                let owner = editor_key.clone();
                                move |next: bool| {
                                    let current = disclosures();
                                    disclosures.set(CaseBodyDisclosureState {
                                        owner: owner.clone(),
                                        mounts_open: next,
                                        mounts_chosen: true,
                                        gasket_open: if current.owner == owner && current.gasket_chosen { current.gasket_open } else { gasket_default_open },
                                        gasket_chosen: current.owner == owner && current.gasket_chosen,
                                    });
                                }
                            },
                            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground justify-between", aria_label: "Mounting",
                                span { class: "font-semibold", "Mounting" }
                                small { class: "text-xs text-muted-foreground", "{body.mounts.as_ref().map_or(0, Vec::len)} mounts" }
                            }
                            CollapsibleContent { class: "data-[open=false]:hidden",
                            div { class: "grid min-w-0 gap-2.5",
                                Button {
                                    variant: ButtonVariant::Secondary, button_type: "button",
                                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                    disabled: !can_edit,
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
                                        fieldset { key: "{mount_id}", class: "grid min-w-0 grid-cols-[minmax(0,1fr)_auto] gap-2.5 rounded-[5px] border border-border p-2.5 [&_legend]:font-semibold [&_legend]:text-foreground",
                                            legend { "{mount_label}" }
                                            Button {
                                                variant: ButtonVariant::Secondary, button_type: "button",
                                                class: "justify-self-end max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                                disabled: !can_edit,
                                                aria_label: "Remove {mount_label}",
                                                onclick: { let submit = submit_remove.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |_| submit(CaseBodyEdit::RemoveMount { body_id: body_id.clone(), mount_id: mount_id.clone() }) },
                                                "Remove"
                                            }
                                            CaseStaticSelect {
                                                label: "Type",
                                                value: (mount_kind_value(&mount.kind)).to_string(),
                                                disabled: !can_edit,
                                                class: "col-span-full",
                                                on_change: { let submit = submit_kind.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |value: Option<String>| {
                                                        if let Some(kind) = value.as_deref().and_then(parse_mount_kind) { submit(CaseBodyEdit::SetMountKind { body_id: body_id.clone(), mount_id: mount_id.clone(), kind }); }
                                                    } },
                                                SelectOption::<String> { index: 0usize, value: "hole".to_owned(), text_value: "Hole".to_owned(), "Hole" }
                                                    SelectOption::<String> { index: 1usize, value: "boss".to_owned(), text_value: "Boss".to_owned(), "Boss" }
                                            }
                                            div { class: "col-span-full grid grid-cols-2 gap-2.5 max-[760px]:grid-cols-1",
                                                CaseNumberField {
                                                    label: "X position", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-x"), value: mount.at.x, unit: "mm", rule: NumberRule::Finite,
                                                    editable: can_edit,
                                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountX { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                }
                                                CaseNumberField {
                                                    label: "Y position", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-y"), value: mount.at.y, unit: "mm", rule: NumberRule::Finite,
                                                    editable: can_edit,
                                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountY { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                }
                                                CaseNumberField {
                                                    label: "Hole diameter", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-hole-diameter"), value: mount.hole_diameter, unit: "mm", rule: NumberRule::Positive,
                                                    editable: can_edit,
                                                    on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountHoleDiameter { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                }
                                                if matches!(&mount.kind, MountKind::Boss) {
                                                    CaseNumberField {
                                                        label: "Boss diameter", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-boss-diameter"), value: mount.boss_diameter.unwrap_or(5.0), unit: "mm", rule: NumberRule::Positive,
                                                        editable: can_edit,
                                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); let mount_id = mount_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetMountBossDiameter { body_id: body_id.clone(), mount_id: mount_id.clone(), value: change.value }, Some(change.field_id)) }
                                                    }
                                                    CaseNumberField {
                                                        label: "Height", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, Some(&mount_id), "mount-height"), value: mount.height.unwrap_or(5.0), unit: "mm", rule: NumberRule::Positive,
                                                        editable: can_edit,
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
                        }
                        Collapsible { class: "grid min-w-0 gap-2.5 border-t border-border py-3", keep_mounted: true, open: Some(gasket_open),
                            on_open_change: {
                                let mut disclosures = disclosures;
                                let owner = editor_key.clone();
                                move |next: bool| {
                                    let current = disclosures();
                                    disclosures.set(CaseBodyDisclosureState {
                                        owner: owner.clone(),
                                        mounts_open: if current.owner == owner && current.mounts_chosen { current.mounts_open } else { mounts_default_open },
                                        mounts_chosen: current.owner == owner && current.mounts_chosen,
                                        gasket_open: next,
                                        gasket_chosen: true,
                                    });
                                }
                            },
                            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground justify-between", aria_label: "Gasket channel",
                                span { class: "font-semibold", "Gasket channel" }
                                small { class: "text-xs text-muted-foreground", if body.gasket.is_some() { "Configured" } else { "Optional" } }
                            }
                            CollapsibleContent { class: "data-[open=false]:hidden",
                            div { class: "grid min-w-0 gap-2.5",
                                if body.gasket.is_some() {
                                    Button { variant: ButtonVariant::Secondary, button_type: "button",
                                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                        disabled: !can_edit,
                                        onclick: { let submit = emit_edit.clone(); let body_id = body_id.clone(); move |_| submit(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: None }) },
                                        "Remove"
                                    }
                                } else {
                                    Button { variant: ButtonVariant::Secondary, button_type: "button",
                                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                        disabled: !can_edit,
                                        onclick: { let submit = emit_edit.clone(); let body_id = body_id.clone(); move |_| submit(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: Some(Gasket { inset: 2.0, width: 2.0, depth: 1.5 }) }) },
                                        "+ Add gasket"
                                    }
                                }
                            if let Some(gasket) = body.gasket.as_ref() {
                                div { class: "grid grid-cols-2 gap-2.5 max-[760px]:grid-cols-1",
                                    CaseNumberField {
                                        label: "Inset", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "gasket-inset"), value: gasket.inset, unit: "mm", rule: NumberRule::Nonnegative,
                                        editable: can_edit,
                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetGasketInset { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                                    }
                                    CaseNumberField {
                                        label: "Width", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "gasket-width"), value: gasket.width, unit: "mm", rule: NumberRule::Positive,
                                        editable: can_edit,
                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetGasketWidth { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                                    }
                                    CaseNumberField {
                                        label: "Depth", field_id: case_field_id(props.editor_instance_id, &props.scope, &body_id, None, "gasket-depth"), value: gasket.depth, unit: "mm", rule: NumberRule::Positive,
                                        editable: can_edit,
                                        on_commit: { let submit = emit_edit_with_field.clone(); let body_id = body_id.clone(); move |change: CaseNumberCommit| submit(CaseBodyEdit::SetGasketDepth { body_id: body_id.clone(), value: change.value }, Some(change.field_id)) }
                                    }
                                }
                            }
                            }
                            }
                        }
                    }
                }
            } else if board.is_some() {
                Callout { "Add a plate, tray, or lid to begin the case stack." }
            } else {
                Callout { "Add a board before creating a case body." }
            }
            if let Some(error) = global_feedback_error {
                Alert { variant: AlertVariant::Destructive, AlertDescription { "{error}" } }
            }
        }
    }
}

/// Presentation-only composition; the caller owns the accepted value and domain request.
#[component]
fn CaseStaticSelect(
    label: String,
    value: String,
    #[props(default)] class: String,
    #[props(default)] disabled: bool,
    on_change: EventHandler<Option<String>>,
    children: Element,
) -> Element {
    let id = format!("case-body-select-{}", dioxus::core::current_scope_id().0);
    let selected = use_memo(move || Some(value.clone()));
    rsx! {
        Field { class: "gap-1.5 {class}", disabled,
            FieldLabel { html_for: id.clone(), "{label}" }
            Select::<String> { class: "max-[760px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch", id, aria_label: label, value: Some(selected.into()), disabled, on_value_change: on_change, {children} }
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
    on_commit: EventHandler<CaseNumberCommit>,
}

#[component]
fn CaseNumberField(props: CaseNumberFieldProps) -> Element {
    let draft = use_signal(|| props.value.to_string());
    let error = use_signal(|| None::<String>);
    let dirty = use_signal(|| false);
    let skip_enter_blur = use_hook(|| Rc::new(Cell::new(false)));

    let field_failure = try_consume_context::<super::case_controller::BodyFieldFeedback>()
        .and_then(|feedback| {
            feedback
                .0
                .read()
                .iter()
                .find(|entry| entry.field_id.as_deref() == Some(props.field_id.as_str()))
                .and_then(|entry| entry.failure.clone())
        });
    let accepted_text = props.value.to_string();
    let current_draft = draft();
    let is_dirty = dirty();
    let mut draft_for_effect = draft;
    let mut error_for_effect = error;
    let mut dirty_for_effect = dirty;
    use_effect(use_reactive(
        (&accepted_text, &current_draft, &is_dirty),
        move |(accepted, current_draft, is_dirty)| {
            if current_draft == accepted {
                dirty_for_effect.set(false);
                error_for_effect.set(None);
            } else if !is_dirty {
                draft_for_effect.set(accepted);
                error_for_effect.set(None);
            }
        },
    ));

    let commit: Rc<dyn Fn()> = Rc::new({
        let on_commit = props.on_commit;
        let field_id = props.field_id.clone();
        let rule = props.rule;
        move || {
            let mut error = error;
            let mut dirty = dirty;
            if !dirty() {
                return;
            }
            let value_text = draft();
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
            dirty.set(false);
            on_commit.call(CaseNumberCommit {
                field_id: field_id.clone(),
                value,
            });
        }
    });

    let min = match props.rule {
        NumberRule::Finite => None,
        NumberRule::Nonnegative => Some("0".to_owned()),
        NumberRule::Positive => Some("0.001".to_owned()),
    };
    // A rejected save is as invalid as a rejected draft: both mark the input and describe it.
    let field_error = error().or_else(|| (!dirty()).then_some(field_failure).flatten());
    let id = format!("case-body-number-{}", dioxus::core::current_scope_id().0);
    let message_id = format!("{id}-message");

    rsx! {
        Field { invalid: field_error.is_some(), disabled: !props.editable, class: "gap-1.5",
            FieldLabel { html_for: id.clone(), "{props.label}" }
            InputGroup { class: "max-[760px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                InputGroupInput {
                    id: id.clone(), class: "font-mono",
                    r#type: InputType::Number,
                    step: "0.1",
                    min,
                    aria_label: props.label,
                    value: "{draft}",
                    disabled: !props.editable,
                    aria_invalid: field_error.is_some(),
                    aria_describedby: if field_error.is_some() { Some(message_id.clone()) } else { None },
                    oninput: {
                        let skip_enter_blur = skip_enter_blur.clone();
                        move |event: FormEvent| {
                            let mut draft = draft;
                            let mut dirty = dirty;
                            let mut error = error;
                            skip_enter_blur.set(false);
                            draft.set(event.value());
                            dirty.set(true);
                            error.set(None);
                        }
                    },
                    attributes: vec![dioxus::events::onblur({
                        let commit = commit.clone();
                        let skip_enter_blur = skip_enter_blur.clone();
                        move |_| {
                            if !skip_enter_blur.replace(false) {
                                commit();
                            }
                        }
                    }), dioxus::events::onkeydown({
                        let commit = commit.clone();
                        let mut dirty = dirty;
                        let mut draft = draft;
                        let mut error = error;
                        let accepted = props.value;
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            if key == "Enter" {
                                event.prevent_default();
                                skip_enter_blur.set(true);
                                commit();
                            } else if key == "Escape" {
                                event.prevent_default();
                                skip_enter_blur.set(false);
                                draft.set(accepted.to_string());
                                dirty.set(false);
                                error.set(None);
                            }
                        }
                    })],
                }
                InputGroupText { class: "pr-3", "{props.unit}" }
            }
            div { id: message_id,
                if let Some(error) = field_error.as_deref() { FieldError { "{error}" } }
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

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use crate::test_contexts::use_case_viewer_test_contexts;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_model::model::Mount;
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen_test::wasm_bindgen_test;

    fn scope() -> Scope {
        Scope {
            session_epoch: SessionEpoch(1),
            document_id: "document".into(),
            board_id: "board".into(),
            instance_id: None,
        }
    }

    fn plate_body() -> CaseBody {
        CaseBody {
            features: None,
            openings: None,
            id: "body-plate".into(),
            name: "Plate body".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 1.5,
            clearance: 0.2,
            material_id: None,
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: Some(vec![Mount {
                id: "mount-1".into(),
                at: Vec2 { x: 5.0, y: 6.0 },
                kind: MountKind::Hole,
                hole_diameter: 3.0,
                boss_diameter: None,
                height: None,
            }]),
            pcb_source: None,
            gasket: None,
        }
    }

    fn tray_body() -> CaseBody {
        CaseBody {
            id: "body-tray".into(),
            name: "Tray body".into(),
            kind: CaseKind::Tray,
            mounts: None,
            ..plate_body()
        }
    }

    fn describe(edit: &CaseBodyEdit) -> String {
        match edit {
            CaseBodyEdit::AddBody => "AddBody".into(),
            CaseBodyEdit::SetKind { kind, .. } => format!("SetKind:{kind:?}"),
            CaseBodyEdit::AddMount { .. } => "AddMount".into(),
            other => format!("other:{}", other.action_id().unwrap_or_default()),
        }
    }

    fn host(bodies: Vec<CaseBody>) -> Element {
        use_case_viewer_test_contexts();
        let sequence = use_signal(|| 0u64);
        let mut last_request = use_signal(String::new);
        rsx! {
            CaseBodies {
                board: Some(CaseBoardSummary { id: "board".into(), name: "Board".into() }),
                bodies,
                scope: scope(),
                editor_instance_id: 1,
                request_sequence: sequence,
                snapshot_token: SnapshotToken(1),
                revision: 1,
                generated_stack: false,
                mismatch: None,
                editable: true,
                feedback: vec![],
                on_edit: move |request: CaseBodyRequest| last_request.set(describe(&request.edit)),
                on_show_configured_board: move |_| {},
            }
            p { id: "bodies-last-request", "{last_request}" }
        }
    }

    fn single_host() -> Element {
        host(vec![plate_body()])
    }

    fn pair_host() -> Element {
        host(vec![plate_body(), tray_body()])
    }

    fn mounted(host: fn() -> Element) -> (web_sys::Element, MountedFixture) {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let mounted = MountedFixture::new(root.clone(), VirtualDom::new(host));
        (root, mounted)
    }

    fn last_request(root: &web_sys::Element) -> String {
        root.query_selector("#bodies-last-request")
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap_or_default()
    }

    #[wasm_bindgen_test]
    async fn body_type_select_uses_custom_trigger_and_commits_kind() {
        let (root, _mounted) = mounted(single_host);
        wait_until("case bodies mount", || {
            by_role_name(&root, "button", "+ New case body").is_some()
        })
        .await;
        let trigger = by_role_name(&root, "button", "Body type").expect("body type trigger");
        trigger.click();
        wait_until("body type options open", || {
            by_role_name(&root, "option", "Tray").is_some()
        })
        .await;
        let listbox = by_role_name(&root, "listbox", "Body type").expect("body type list");
        let press = |key: &str| {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(key);
            init.set_bubbles(true);
            listbox
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                        .unwrap(),
                )
                .unwrap();
        };
        let tray_focused = || {
            by_role_name(&root, "option", "Tray")
                .is_some_and(|option| option.get_attribute("tabindex").as_deref() == Some("0"))
        };
        for _ in 0..4 {
            press("ArrowDown");
            gloo_timers::future::TimeoutFuture::new(10).await;
            if tray_focused() {
                break;
            }
        }
        assert!(tray_focused(), "ArrowDown reaches the Tray option");
        press("Enter");
        wait_until("kind change is requested", || {
            last_request(&root) == "SetKind:Tray"
        })
        .await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounting_section_uses_disclosure_trigger() {
        let (root, _mounted) = mounted(single_host);
        wait_until("case bodies mount", || {
            by_role_name(&root, "button", "+ New case body").is_some()
        })
        .await;
        let trigger = by_role_name(&root, "button", "Mounting").expect("mounting trigger");
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("true"),
            "a body with mounts opens its mounting section"
        );
        trigger.click();
        wait_until("mounting section collapses", || {
            by_role_name(&root, "button", "Mounting").is_some_and(|trigger| {
                trigger.get_attribute("aria-expanded").as_deref() == Some("false")
            })
        })
        .await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn new_body_button_emits_add_request() {
        let (root, _mounted) = mounted(single_host);
        wait_until("case bodies mount", || {
            by_role_name(&root, "button", "+ New case body").is_some()
        })
        .await;
        by_role_name(&root, "button", "+ New case body")
            .unwrap()
            .click();
        wait_until("add-body is requested", || last_request(&root) == "AddBody").await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn thickness_rejects_invalid_draft_with_described_error() {
        use wasm_bindgen::JsCast;
        let (root, _mounted) = mounted(single_host);
        wait_until("thickness field mounts", || {
            by_role_name(&root, "spinbutton", "Thickness").is_some()
        })
        .await;
        let input: web_sys::HtmlInputElement = by_role_name(&root, "spinbutton", "Thickness")
            .unwrap()
            .dyn_into()
            .unwrap();
        input.focus().unwrap();
        input.set_value("0");
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
        input.blur().unwrap();
        wait_until("invalid thickness is rejected locally", || {
            input.get_attribute("aria-invalid").as_deref() == Some("true")
        })
        .await;
        assert_eq!(input.value(), "0", "the field owns the rejected draft");
        let described = input
            .get_attribute("aria-describedby")
            .expect("validation must describe the precise input");
        let mut description = String::new();
        for id in described.split_whitespace() {
            let node = root
                .query_selector(&format!("[id='{id}']"))
                .unwrap()
                .expect("description resolves in this production field");
            description.push_str(&node.text_content().unwrap_or_default());
        }
        assert!(
            description.contains("above 0"),
            "missing error: {description}"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn body_tabs_switch_the_active_editor() {
        let (root, _mounted) = mounted(pair_host);
        wait_until("case bodies mount", || {
            by_role_name(&root, "button", "+ New case body").is_some()
        })
        .await;
        assert!(by_role_name(&root, "spinbutton", "Wall height").is_none());
        // Tab content concatenates without spaces, as in production markup.
        let tray_tab = by_role_name(&root, "button", "02Tray bodytray").expect("tray body tab");
        assert_eq!(
            tray_tab.get_attribute("aria-pressed").as_deref(),
            Some("false")
        );
        tray_tab.click();
        wait_until("tray editor shows wall fields", || {
            by_role_name(&root, "spinbutton", "Wall height").is_some()
        })
        .await;
        assert_eq!(
            by_role_name(&root, "button", "02Tray bodytray")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        root.remove();
    }
}
