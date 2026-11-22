//! Authored case body list and editor: declarative fields over the accepted bodies. The
//! controller owns admission and commits every `CaseBodyEdit`.
pub use super::case_body_edit::CaseBodyEdit;
use super::case_body_edit::{BodyField, MountField};
use super::shared_viewer::{BodySelection, CaseSelection};
use boardstudio_application::Scope;
use boardstudio_model::model::{CaseBody, CaseKind, Gasket, MountKind};
use boardstudio_web_ui_shared::inspector_fields::{InspectorSection, NumericField, SelectField};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    callout::Callout,
    toggle_group::{ToggleGroup, ToggleItem},
};
use dioxus::prelude::*;
use std::collections::HashSet;

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

#[derive(Props, Clone, PartialEq)]
pub struct CaseBodiesProps {
    pub board: Option<CaseBoardSummary>,
    pub bodies: Vec<CaseBody>,
    pub scope: Scope,
    pub generated_stack: bool,
    pub mismatch: Option<CaseMismatch>,
    pub editable: bool,
    pub error: Option<String>,
    pub on_edit: EventHandler<CaseBodyEdit>,
    pub on_show_configured_board: EventHandler<String>,
}

const KINDS: [(CaseKind, &str); 3] = [
    (CaseKind::Plate, "Plate"),
    (CaseKind::Tray, "Tray"),
    (CaseKind::Lid, "Lid"),
];

fn kind_label(kind: &CaseKind) -> &'static str {
    KINDS
        .iter()
        .find(|(candidate, _)| candidate == kind)
        .map_or("Plate", |(_, label)| label)
}

#[component]
pub fn CaseBodies(props: CaseBodiesProps) -> Element {
    let mut selected_body = use_context::<CaseSelection>().body;
    let edit = props.on_edit;
    let can_edit = props.editable;
    let reason = (!can_edit).then(|| "Read-only right now.".to_owned());
    let bodies: Vec<&CaseBody> = props
        .bodies
        .iter()
        .filter(|body| {
            props
                .board
                .as_ref()
                .is_some_and(|board| body.board_id == board.id)
        })
        .collect();
    let selection = selected_body
        .read()
        .clone()
        .filter(|selected| selected.scope == props.scope);
    let active = selection
        .and_then(|selection| {
            bodies
                .iter()
                .copied()
                .find(|body| body.id == selection.body_id)
        })
        .or_else(|| bodies.first().copied());

    if props.generated_stack {
        return rsx! {
            section { aria_label: "Authored case bodies",
                Callout { "Generated assembly preview · {bodies.len()} authored case bodies remain saved. Disable the mechanical stack to preview and edit them." }
            }
        };
    }
    rsx! {
        section { class: "flex min-h-0 min-w-0 flex-1 flex-col content-start gap-3.5 overflow-auto bg-card px-3 py-4 text-foreground", aria_label: "Authored case bodies",
            if let Some(mismatch) = props.mismatch.as_ref() {
                section { aria_label: "Mechanical stack board mismatch",
                    h3 { "Mechanical stack belongs to {mismatch.board_name}" }
                    Callout { "This board shows its authored case bodies. Select the configured board to inspect that mechanical stack." }
                    Button { variant: ButtonVariant::Secondary, disabled: mismatch.board_id.is_empty(),
                        onclick: { let board_id = mismatch.board_id.clone(); move |_| props.on_show_configured_board.call(board_id.clone()) },
                        "Show configured board" }
                }
            }
            header { class: "flex items-center justify-between gap-2 border-b border-border pb-2.5",
                h2 { "Case stack" }
                Button { variant: ButtonVariant::Secondary, disabled: !can_edit || props.board.is_none(), onclick: move |_| edit.call(CaseBodyEdit::AddBody), "+ New case body" }
            }
            if !bodies.is_empty() {
                ToggleGroup {
                    aria_label: "Case bodies", disabled: !can_edit, style: "width: 100%",
                    class: "grid grid-cols-[repeat(auto-fit,minmax(112px,1fr))] gap-1.5",
                    pressed: Some(active.and_then(|active| bodies.iter().position(|body| body.id == active.id)).map(|index| HashSet::from([index])).unwrap_or_default()),
                    on_pressed_change: {
                        let ids: Vec<String> = bodies.iter().map(|body| body.id.clone()).collect();
                        let scope = props.scope.clone();
                        move |next: HashSet<usize>| {
                            if let Some(id) = next.iter().next().and_then(|index| ids.get(*index)) {
                                selected_body.set(Some(BodySelection { scope: scope.clone(), body_id: id.clone() }));
                            }
                        }
                    },
                    for (index, body) in bodies.iter().enumerate() {
                        ToggleItem { key: "{body.id}", index, disabled: !can_edit,
                            span { "{index + 1:02}" }
                            strong { "{body.name}" }
                            small { "{kind_label(&body.kind)}" }
                        }
                    }
                }
            }
            if let Some(body) = active {
                div { key: "{body.id}", class: "grid min-w-0 gap-2.5 border-t border-border py-3",
                    SelectField::<CaseKind> { label: "Body type", value: body.kind.clone(), disabled_reason: reason.clone(),
                        options: KINDS.iter().map(|(kind, label)| (kind.clone(), (*label).to_owned())).collect::<Vec<_>>(),
                        on_commit: { let body_id = body.id.clone(); move |kind| edit.call(CaseBodyEdit::SetKind { body_id: body_id.clone(), kind }) } }
                    div { class: "grid grid-cols-2 gap-2.5 max-[760px]:grid-cols-1",
                        {body_field(body, BodyField::Thickness, body.thickness, &reason, edit)}
                        {body_field(body, BodyField::Clearance, body.clearance, &reason, edit)}
                        {body_field(body, BodyField::Z, body.z.unwrap_or(0.0), &reason, edit)}
                        if !matches!(body.kind, CaseKind::Plate) {
                            {body_field(body, BodyField::WallHeight, body.wall_height.unwrap_or(14.0), &reason, edit)}
                            {body_field(body, BodyField::WallThickness, body.wall_thickness.unwrap_or(2.0), &reason, edit)}
                        }
                    }
                    InspectorSection { title: "Mounting", default_open: body.mounts.as_ref().is_some_and(|mounts| !mounts.is_empty()),
                        badge: format!("{} mounts", body.mounts.as_ref().map_or(0, Vec::len)),
                        Button { variant: ButtonVariant::Secondary, disabled: !can_edit,
                            onclick: { let body_id = body.id.clone(); move |_| edit.call(CaseBodyEdit::AddMount { body_id: body_id.clone() }) }, "+ Add mount" }
                        for (index, mount) in body.mounts.iter().flatten().enumerate() {
                            fieldset { key: "{mount.id}", class: "grid min-w-0 gap-2.5 rounded-[5px] border border-border p-2.5",
                                legend { "Mount {index + 1}" }
                                Button { variant: ButtonVariant::Secondary, disabled: !can_edit, aria_label: "Remove Mount {index + 1}",
                                    onclick: { let (body_id, mount_id) = (body.id.clone(), mount.id.clone()); move |_| edit.call(CaseBodyEdit::RemoveMount { body_id: body_id.clone(), mount_id: mount_id.clone() }) }, "Remove" }
                                SelectField::<MountKind> { label: "Type", value: mount.kind.clone(), disabled_reason: reason.clone(),
                                    options: vec![(MountKind::Hole, "Hole".to_owned()), (MountKind::Boss, "Boss".to_owned())],
                                    on_commit: { let (body_id, mount_id) = (body.id.clone(), mount.id.clone()); move |kind| edit.call(CaseBodyEdit::SetMountKind { body_id: body_id.clone(), mount_id: mount_id.clone(), kind }) } }
                                div { class: "grid grid-cols-2 gap-2.5 max-[760px]:grid-cols-1",
                                    for (field, value) in [(MountField::X, mount.at.x), (MountField::Y, mount.at.y), (MountField::HoleDiameter, mount.hole_diameter)]
                                        .into_iter()
                                        .chain((mount.kind == MountKind::Boss).then_some([(MountField::BossDiameter, mount.boss_diameter.unwrap_or(5.0)), (MountField::Height, mount.height.unwrap_or(5.0))]).into_iter().flatten())
                                    {
                                        {
                                            let (label, rule) = field.meta();
                                            let (body_id, mount_id) = (body.id.clone(), mount.id.clone());
                                            rsx! { NumericField { key: "{label}", label, unit: "mm", value, min: rule.min(), disabled_reason: reason.clone(),
                                                on_commit: move |value| edit.call(CaseBodyEdit::SetMountField { body_id: body_id.clone(), mount_id: mount_id.clone(), field, value }) } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    InspectorSection { title: "Gasket channel", default_open: body.gasket.is_some(), badge: if body.gasket.is_some() { "Configured" } else { "Optional" },
                        Button { variant: ButtonVariant::Secondary, disabled: !can_edit,
                            onclick: { let body_id = body.id.clone(); let gasket = body.gasket.is_none().then_some(Gasket { inset: 2.0, width: 2.0, depth: 1.5 }); move |_| edit.call(CaseBodyEdit::SetGasket { body_id: body_id.clone(), gasket: gasket.clone() }) },
                            if body.gasket.is_some() { "Remove" } else { "+ Add gasket" } }
                        if let Some(gasket) = body.gasket.as_ref() {
                            div { class: "grid grid-cols-2 gap-2.5 max-[760px]:grid-cols-1",
                                {body_field(body, BodyField::GasketInset, gasket.inset, &reason, edit)}
                                {body_field(body, BodyField::GasketWidth, gasket.width, &reason, edit)}
                                {body_field(body, BodyField::GasketDepth, gasket.depth, &reason, edit)}
                            }
                        }
                    }
                }
            } else if props.board.is_some() {
                Callout { "Add a plate, tray, or lid to begin the case stack." }
            } else {
                Callout { "Add a board before creating a case body." }
            }
            if let Some(error) = props.error.clone() {
                Alert { variant: AlertVariant::Destructive, AlertDescription { "{error}" } }
            }
        }
    }
}

fn body_field(
    body: &CaseBody,
    field: BodyField,
    value: f64,
    reason: &Option<String>,
    edit: EventHandler<CaseBodyEdit>,
) -> Element {
    let (label, rule) = field.meta();
    let body_id = body.id.clone();
    rsx! {
        NumericField { key: "{label}", label, unit: "mm", value, min: rule.min(), disabled_reason: reason.clone(),
            on_commit: move |value| edit.call(CaseBodyEdit::SetField { body_id: body_id.clone(), field, value }) }
    }
}
