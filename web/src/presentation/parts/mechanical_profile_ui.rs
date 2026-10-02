use super::PartsPreviewPanel;
use crate::parts_mechanical_profile::{
    ProfileDefinitionSource, ProfileEditCapture, ProfileEditContext, initial_profile,
    prepare_profile_edit,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{MechanicalPartProfile, PartDefinition, Vec2};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProfileOwner {
    scope: Option<Scope>,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    definition_id: String,
    source: ProfileDefinitionSource,
}

#[derive(Clone, Copy)]
enum ContourField {
    Cutouts,
    Clearances,
}

#[component]
pub(crate) fn PartsMechanicalProfileWorkspace(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    mut selection: Signal<Option<(Option<Scope>, String)>>,
    definition: PartDefinition,
    source: ProfileDefinitionSource,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let owner = ProfileOwner {
        scope: scope.clone(),
        session_epoch: snapshot.session_epoch,
        document_id: snapshot.document.id.clone(),
        definition_id: definition.id.clone(),
        source,
    };
    let editing = use_signal(|| false);
    let capture = use_signal(|| None::<ProfileEditCapture>);
    use_effect(use_reactive((&owner,), {
        let mut capture = capture;
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        let definition = definition.clone();
        move |(_owner,)| {
            if editing() {
                capture.set(Some(ProfileEditCapture::new(
                    &snapshot,
                    scope.clone(),
                    source,
                    definition.clone(),
                )));
            }
        }
    }));
    let start_editing = {
        let mut editing = editing;
        let mut capture = capture;
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        let definition = definition.clone();
        move |_| {
            selection.set(Some((scope.clone(), definition.id.clone())));
            capture.set(Some(ProfileEditCapture::new(
                &snapshot,
                scope.clone(),
                source,
                definition.clone(),
            )));
            editing.set(true);
        }
    };
    let close_editor = {
        let mut editing = editing;
        let mut capture = capture;
        move |_| {
            editing.set(false);
            capture.set(None);
        }
    };
    let save_profile = {
        let runtime = runtime.clone();
        let selection = selection;
        let definition = definition.clone();
        move |profile: MechanicalPartProfile| {
            let Some(capture) = capture() else {
                return;
            };
            let model = runtime.model();
            let Some(current) = model.accepted else {
                return;
            };
            let Some(event) = prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    scope: runtime.scope(),
                    selection: selection(),
                    source,
                    definition: &definition,
                },
                &capture,
                profile,
                runtime.operation(),
            ) else {
                return;
            };
            runtime.submit(event);
        }
    };

    if editing() {
        let editor_key = format!("{owner:?}");
        return rsx! {
            ManualProfileEditor {
                key: "{editor_key}",
                definition: definition.clone(),
                initial: definition.mechanical_profile.clone(),
                on_save: save_profile,
                on_close: close_editor,
            }
        };
    }
    let profile_defined = definition.mechanical_profile.is_some();
    rsx! {
        section { class: "m1-parts-mechanical-fit", "aria-label": "Mechanical fit profile",
            div {
                h3 { "Mechanical fit" }
                p {
                    if profile_defined { "Defined with this part and inherited by layouts and cases." }
                    else { "Define this part’s fit once so every layout and case uses the same profile." }
                }
            }
            button {
                class: "m1-secondary",
                r#type: "button",
                onclick: start_editing,
                if profile_defined { "Edit profile" } else { "Define profile" }
            }
        }
        PartsPreviewPanel {
            definition: Some(Rc::new(definition)),
            scope,
            snapshot_token: snapshot.token,
        }
    }
}

#[component]
fn ManualProfileEditor(
    definition: PartDefinition,
    initial: Option<MechanicalPartProfile>,
    on_save: EventHandler<MechanicalPartProfile>,
    on_close: EventHandler<()>,
) -> Element {
    let mut draft = use_signal(|| initial_profile(&definition, initial.as_ref()));
    let profile = draft.read().clone();
    let family_details = profile.switch_family.map(|family| {
        let (datum, thickness) = match family {
            boardstudio_core::model::MechanicalSwitchFamily::Mx => (5.0, 1.5),
            boardstudio_core::model::MechanicalSwitchFamily::ChocV1 => (3.5, 1.3),
            boardstudio_core::model::MechanicalSwitchFamily::ChocV2 => (5.0, 1.5),
        };
        (datum - thickness, thickness)
    });
    rsx! {
        section { class: "m1-parts-fit-editor", "aria-label": "Mechanical fit profile editor",
            header {
                div {
                    h2 { "{definition.name} fit" }
                    p { "Save the fit with this part. Every case using it inherits the profile." }
                }
                button { class: "m1-secondary", r#type: "button", onclick: move |_| on_close.call(()), "Cancel" }
            }
            if let Some((gap, thickness)) = family_details {
                p { "Mounting gap: {gap:.2} mm with a {thickness:.2} mm plate. Case recalculates the gap for its plate thickness." }
            } else {
                label { class: "m1-parts-fit-field",
                    span { "Plate underside to PCB top (mm)" }
                    input {
                        r#type: "number",
                        min: "0",
                        step: "0.01",
                        value: "{profile.plate_to_pcb}",
                        oninput: move |event| {
                            if let Ok(value) = event.value().parse::<f64>()
                                && value.is_finite() && value >= 0.0 {
                                draft.with_mut(|profile| profile.plate_to_pcb = value);
                            }
                        },
                    }
                }
            }
            {contour_editor(draft, ContourField::Cutouts, "Plate cutouts")}
            {contour_editor(draft, ContourField::Clearances, "Component clearances")}
            button {
                class: "m1-primary",
                r#type: "button",
                onclick: move |_| { on_save.call(draft()); on_close.call(()); },
                "Save fit profile"
            }
        }
    }
}

fn contour_editor(
    mut draft: Signal<MechanicalPartProfile>,
    field: ContourField,
    label: &'static str,
) -> Element {
    let profile = draft.read().clone();
    let contours = match field {
        ContourField::Cutouts => profile.cutouts.clone(),
        ContourField::Clearances => profile.clearances.clone().unwrap_or_default(),
    };
    let add_label = match field {
        ContourField::Cutouts => "cutout",
        ContourField::Clearances => "clearance",
    };
    rsx! {
        section { class: "m1-parts-fit-section", "aria-label": "{label}",
            header {
                strong { "{label}" }
                button {
                    class: "m1-secondary",
                    r#type: "button",
                    onclick: move |_| draft.with_mut(|profile| {
                        let mut next = empty_square();
                        match field {
                            ContourField::Cutouts => profile.cutouts.push(std::mem::take(&mut next)),
                            ContourField::Clearances => profile.clearances.get_or_insert_with(Vec::new).push(std::mem::take(&mut next)),
                        }
                    }),
                    "Add {add_label}"
                }
            }
            if contours.is_empty() { p { "No {label.to_lowercase()} defined." } }
            for (shape_index, contour) in contours.iter().enumerate() {
                div { class: "m1-parts-fit-contour",
                    header {
                        strong { "Contour {shape_index + 1}" }
                        button {
                            class: "m1-secondary",
                            r#type: "button",
                            onclick: move |_| draft.with_mut(|profile| match field {
                                ContourField::Cutouts => { if shape_index < profile.cutouts.len() { profile.cutouts.remove(shape_index); } },
                                ContourField::Clearances => if let Some(contours) = &mut profile.clearances
                                    && shape_index < contours.len() { contours.remove(shape_index); },
                            }),
                            "Remove contour {shape_index + 1}"
                        }
                    }
                    for (point_index, point) in contour.iter().enumerate() {
                        div { class: "m1-parts-fit-point",
                            span { "Vertex {point_index + 1}" }
                            for axis in ["X", "Y"] {
                                { let axis_value = if axis == "X" { point.x } else { point.y };
                                  rsx! {
                                    input {
                                        r#type: "number",
                                        step: "0.1",
                                        aria_label: "{label} contour {shape_index + 1} vertex {point_index + 1} {axis}",
                                        value: "{axis_value}",
                                        oninput: move |event| {
                                            let Ok(value) = event.value().parse::<f64>() else { return; };
                                            if !value.is_finite() { return; }
                                            draft.with_mut(|profile| {
                                                let contours = match field {
                                                    ContourField::Cutouts => &mut profile.cutouts,
                                                    ContourField::Clearances => profile.clearances.get_or_insert_with(Vec::new),
                                                };
                                                if let Some(vertex) = contours.get_mut(shape_index).and_then(|contour| contour.get_mut(point_index)) {
                                                    if axis == "X" { vertex.x = value; } else { vertex.y = value; }
                                                }
                                            });
                                        },
                                    }
                                  }
                                }
                            }
                        }
                    }
                    button {
                        class: "m1-secondary",
                        r#type: "button",
                        onclick: move |_| draft.with_mut(|profile| {
                            let contours = match field {
                                ContourField::Cutouts => &mut profile.cutouts,
                                ContourField::Clearances => profile.clearances.get_or_insert_with(Vec::new),
                            };
                            if let Some(last) = contours.get_mut(shape_index).and_then(|contour| contour.last().copied()) {
                                contours[shape_index].push(last);
                            }
                        }),
                        "Add vertex"
                    }
                }
            }
        }
    }
}

fn empty_square() -> Vec<Vec2> {
    vec![
        Vec2 { x: -2.5, y: -2.5 },
        Vec2 { x: 2.5, y: -2.5 },
        Vec2 { x: 2.5, y: 2.5 },
        Vec2 { x: -2.5, y: 2.5 },
    ]
}
