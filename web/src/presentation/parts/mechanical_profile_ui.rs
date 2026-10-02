use super::super::{WorkspaceState, selection::SelectionAdapter};
use super::PartsPreviewPanel;
use super::PartsSelectionGeneration;
use crate::parts_mechanical_profile::{
    PendingProfileEdit, ProfileDefinitionSource, ProfileEditCapture, ProfileEditContext,
    ProfileEditOwner, StandardProfileRequestCapture, displayed_mounting_gap, initial_profile,
    merge_standard_profile, prepare_profile_edit, standard_profile_request_is_current,
    standard_profile_source_and_gap,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{
    MechanicalPartProfile, MechanicalSwitchFamily, PartDefinition, Vec2,
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

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
    let view_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation()
    });
    let owner = ProfileEditOwner::new(view_id, &snapshot, scope.clone(), source, &definition);
    let selection_generation = use_context::<PartsSelectionGeneration>().0;
    let selection_adapter = use_context::<SelectionAdapter>();
    let workspace = use_context::<WorkspaceState>().0;
    let runtime_version = use_context::<Signal<u64>>();
    let _ = runtime_version();
    let editing = use_signal(|| false);
    let capture = use_signal(|| None::<ProfileEditCapture>);
    let pending = use_signal(|| None::<PendingProfileEdit>);
    let capture_owner = owner.clone();
    use_effect(use_reactive((&owner,), {
        let mut capture = capture;
        let definition = definition.clone();
        move |(_owner,)| {
            if editing() {
                capture.set(Some(ProfileEditCapture::new(
                    capture_owner.clone(),
                    definition.clone(),
                )));
            }
        }
    }));
    use_effect(use_reactive((&owner, &runtime_version()), {
        let runtime = runtime.clone();
        let mut pending = pending;
        move |(owner, _)| {
            if pending
                .read()
                .as_ref()
                .is_some_and(|operation| operation.should_retire(&owner, &runtime.scope()))
            {
                pending.set(None);
            }
        }
    }));
    let start_editing = {
        let mut editing = editing;
        let mut capture = capture;
        let owner = owner.clone();
        let scope = scope.clone();
        let definition = definition.clone();
        move |_| {
            selection.set(Some((scope.clone(), definition.id.clone())));
            capture.set(Some(ProfileEditCapture::new(
                owner.clone(),
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
        let mut pending = pending;
        let owner = owner.clone();
        let definition = definition.clone();
        move |profile: MechanicalPartProfile| {
            let Some(capture) = capture() else {
                return;
            };
            let model = runtime.model();
            let Some(current) = model.accepted else {
                return;
            };
            let operation_id = runtime.operation();
            let Some(event) = prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    owner: &owner,
                    runtime_scope: runtime.scope(),
                    selection: selection(),
                    definition: &definition,
                },
                &capture,
                profile,
                operation_id,
            ) else {
                return;
            };
            let outcome = runtime.observe_operation(operation_id);
            pending.set(Some(PendingProfileEdit::new(owner.clone(), outcome)));
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
                source,
                owner: owner.clone(),
                snapshot: snapshot.clone(),
                selection,
                selection_generation,
                scope_generation: selection_adapter.generation,
                workspace,
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
    source: ProfileDefinitionSource,
    owner: ProfileEditOwner,
    snapshot: AcceptedSnapshot,
    selection: Signal<Option<(Option<Scope>, String)>>,
    selection_generation: Signal<u64>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    on_save: EventHandler<MechanicalPartProfile>,
    on_close: EventHandler<()>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let mut draft = use_signal(|| initial_profile(&definition, initial.as_ref()));
    let mut pending_standard = use_signal(|| None::<StandardProfileRequestCapture>);
    let mut standard_error = use_signal(String::new);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let current_view = use_hook(|| {
        Rc::new(RefCell::new((
            owner.clone(),
            definition.clone(),
            snapshot.clone(),
        )))
    });
    *current_view.borrow_mut() = (owner.clone(), definition.clone(), snapshot.clone());
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let profile = draft.read().clone();
    let family_details = profile.switch_family.map(|family| match family {
        boardstudio_core::model::MechanicalSwitchFamily::Mx
        | boardstudio_core::model::MechanicalSwitchFamily::ChocV2 => 1.5,
        boardstudio_core::model::MechanicalSwitchFamily::ChocV1 => 1.3,
    });
    let gap_label = displayed_mounting_gap(&profile).unwrap_or_default();
    let mut load_standard = {
        let runtime = runtime.clone();
        let owner = owner.clone();
        let definition = definition.clone();
        let alive = alive.clone();
        move |family: MechanicalSwitchFamily| {
            if pending_standard.read().is_some() {
                return;
            }
            let (_, plate_to_pcb) = standard_profile_source_and_gap(family);
            let operation_id = runtime.operation();
            let request = StandardProfileRequestCapture::new(
                operation_id,
                owner.clone(),
                definition.clone(),
                family,
                plate_to_pcb,
                scope_generation(),
                selection_generation(),
            );
            pending_standard.set(Some(request.clone()));
            standard_error.set(String::new());
            let runtime = runtime.clone();
            let definition = definition.clone();
            let current_view = current_view.clone();
            let selection = selection;
            let workspace = workspace;
            let scope_generation = scope_generation;
            let selection_generation = selection_generation;
            let alive = alive.clone();
            let mut pending_standard = pending_standard;
            let mut standard_error = standard_error;
            let mut draft = draft;
            spawn_local(async move {
                let result = runtime
                    .standard_switch_profile(
                        operation_id,
                        definition.id.clone(),
                        family,
                        plate_to_pcb,
                    )
                    .await;
                let (current_owner, current_definition, current_snapshot) =
                    current_view.borrow().clone();
                let model = runtime.model();
                let still_current = standard_profile_request_is_current(
                    &request,
                    pending_standard
                        .read()
                        .as_ref()
                        .map(|pending| pending.operation_id),
                    &current_owner,
                    runtime.scope().as_ref(),
                    &current_snapshot,
                    &selection(),
                    &current_definition,
                    scope_generation(),
                    selection_generation(),
                    workspace(),
                    alive.get()
                        && model.accepted.as_ref().is_some_and(|accepted| {
                            accepted.session_epoch == request.owner.session_epoch
                                && accepted.document.id == request.owner.document_id
                        }),
                );
                if !still_current {
                    if pending_standard
                        .read()
                        .as_ref()
                        .is_some_and(|pending| pending.operation_id == request.operation_id)
                    {
                        pending_standard.set(None);
                    }
                    return;
                }
                pending_standard.set(None);
                match result {
                    Ok(loaded) => merge_standard_profile(
                        &mut draft.write(),
                        loaded,
                        request.family,
                        request.plate_to_pcb,
                    ),
                    Err(message) => standard_error.set(message),
                }
            });
        }
    };
    rsx! {
        section { class: "m1-parts-fit-editor", "aria-label": "Mechanical fit profile editor",
            header {
                div {
                    h2 { "{definition.name} fit" }
                    p { "Save the fit with this part. Every case using it inherits the profile." }
                }
                button { class: "m1-secondary", r#type: "button", onclick: move |_| on_close.call(()), "Cancel" }
            }
            if let Some(thickness) = family_details {
                p { "Mounting gap: {gap_label} mm with a {thickness:.2} mm plate. Case recalculates the gap for its plate thickness." }
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
            if source == ProfileDefinitionSource::Ergogen
                && definition.kind == boardstudio_core::model::PartKind::Switch {
                label { class: "m1-parts-fit-field",
                    span { "Switch fit family" }
                    select {
                        value: profile.switch_family.map(family_key).unwrap_or(""),
                        disabled: pending_standard.read().is_some(),
                        onchange: move |event| {
                            if let Some(family) = family_from_key(&event.value()) {
                                draft.with_mut(|profile| profile.switch_family = Some(family));
                            }
                        },
                        option { value: "", "Select a standard family" }
                        option { value: "mx", "MX" }
                        option { value: "choc-v1", "Choc v1" }
                        option { value: "choc-v2", "Choc v2" }
                    }
                }
                button {
                    class: "m1-secondary",
                    r#type: "button",
                    disabled: profile.switch_family.is_none() || pending_standard.read().is_some(),
                    onclick: move |_| {
                        if let Some(family) = draft().switch_family { load_standard(family); }
                    },
                    if pending_standard.read().is_some() { "Loading standard fit…" } else { "Use standard cutout" }
                }
            }
            if let Some(request) = pending_standard.read().as_ref() {
                p { role: "status", "Loading the standard {request.family:?} cutout…" }
            }
            if !standard_error().is_empty() {
                p { role: "alert", "Standard fit could not be loaded: {standard_error()}" }
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

fn family_key(family: MechanicalSwitchFamily) -> &'static str {
    match family {
        MechanicalSwitchFamily::Mx => "mx",
        MechanicalSwitchFamily::ChocV1 => "choc-v1",
        MechanicalSwitchFamily::ChocV2 => "choc-v2",
    }
}

fn family_from_key(value: &str) -> Option<MechanicalSwitchFamily> {
    match value {
        "mx" => Some(MechanicalSwitchFamily::Mx),
        "choc-v1" => Some(MechanicalSwitchFamily::ChocV1),
        "choc-v2" => Some(MechanicalSwitchFamily::ChocV2),
        _ => None,
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
