use super::PartsStandardProfileLifetime;
use crate::parts_mechanical_profile::{
    ProfileEditOwner, StandardProfileRequestCapture, dispatch_standard_profile_family,
    displayed_mounting_gap, initial_profile, merge_standard_profile, standard_profile_controls,
    standard_profile_request_is_current, standard_profile_source_and_gap,
};
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{
    MechanicalPartProfile, MechanicalSwitchFamily, PartDefinition, Vec2,
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy)]
enum ContourField {
    Cutouts,
    Clearances,
}

pub(super) type StandardProfileFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<MechanicalPartProfile, String>>>>;
pub(super) type StandardProfileRequester = Rc<
    dyn Fn(
        String,
        MechanicalSwitchFamily,
        f64,
    ) -> (boardstudio_application::OperationId, StandardProfileFuture),
>;
pub(super) type DetachedProfileSpawner =
    Rc<dyn Fn(std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>)>;
pub(super) type CurrentProfileScope = Rc<dyn Fn() -> Option<Scope>>;
pub(super) type AcceptedProfileOwner = Rc<dyn Fn(&ProfileEditOwner) -> bool>;

#[derive(Clone)]
pub(super) struct ManualProfileEditorPorts {
    pub(super) request_standard_profile: StandardProfileRequester,
    pub(super) spawn_detached: DetachedProfileSpawner,
    pub(super) current_scope: CurrentProfileScope,
    pub(super) accepted_owner_is_current: AcceptedProfileOwner,
}

impl PartialEq for ManualProfileEditorPorts {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(
            &self.request_standard_profile,
            &other.request_standard_profile,
        ) && Rc::ptr_eq(&self.spawn_detached, &other.spawn_detached)
            && Rc::ptr_eq(&self.current_scope, &other.current_scope)
            && Rc::ptr_eq(
                &self.accepted_owner_is_current,
                &other.accepted_owner_is_current,
            )
    }
}

impl Eq for ManualProfileEditorPorts {}

#[component]
pub(super) fn ManualProfileEditor(
    definition: PartDefinition,
    initial: Option<MechanicalPartProfile>,
    owner: ProfileEditOwner,
    snapshot: AcceptedSnapshot,
    selection: Signal<Option<(Option<Scope>, String)>>,
    selection_generation: Signal<u64>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    on_save: EventHandler<MechanicalPartProfile>,
    on_close: EventHandler<()>,
    ports: ManualProfileEditorPorts,
) -> Element {
    let mut draft = use_signal(|| initial_profile(&definition, initial.as_ref()));
    let mut pending_standard = use_signal(|| None::<StandardProfileRequestCapture>);
    let mut standard_error = use_signal(String::new);
    let lifetime = use_hook(PartsStandardProfileLifetime::new);
    let current_view = use_hook(|| {
        Rc::new(RefCell::new((
            owner.clone(),
            definition.clone(),
            snapshot.clone(),
        )))
    });
    *current_view.borrow_mut() = (owner.clone(), definition.clone(), snapshot.clone());
    use_drop({
        let lifetime = lifetime.clone();
        move || lifetime.retire()
    });
    let profile = draft.read().clone();
    let family_details = profile.switch_family.map(|family| match family {
        boardstudio_core::model::MechanicalSwitchFamily::Mx
        | boardstudio_core::model::MechanicalSwitchFamily::ChocV2 => 1.5,
        boardstudio_core::model::MechanicalSwitchFamily::ChocV1 => 1.3,
    });
    let gap_label = displayed_mounting_gap(&profile).unwrap_or_default();
    let (show_family_selector, show_standard_action) =
        standard_profile_controls(&definition, &profile);
    let family_selector_label = String::from("Switch fit family");
    let standard_action_label = String::from("Use standard cutout");
    let save_profile_label = String::from("Save fit profile");
    let load_standard = Rc::new(std::cell::RefCell::new({
        let request_standard_profile = ports.request_standard_profile.clone();
        let owner = owner.clone();
        let definition = definition.clone();
        let lifetime = lifetime.clone();
        move |family: MechanicalSwitchFamily| {
            if pending_standard.read().is_some() {
                return;
            }
            let (_, plate_to_pcb) = standard_profile_source_and_gap(family);
            let (operation_id, request_future) =
                request_standard_profile(definition.id.clone(), family, plate_to_pcb);
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
            let request_future = request_future;
            let current_view = current_view.clone();
            let selection = selection;
            let workspace = workspace;
            let scope_generation = scope_generation;
            let selection_generation = selection_generation;
            let current_scope = ports.current_scope.clone();
            let accepted_owner_is_current = ports.accepted_owner_is_current.clone();
            let lifetime = lifetime.clone();
            let mut pending_standard = pending_standard;
            let mut standard_error = standard_error;
            let mut draft = draft;
            (ports.spawn_detached)(Box::pin(async move {
                let result = request_future.await;
                let Some(()) = lifetime.run_if_mounted(|| {
                    let (current_owner, current_definition, current_snapshot) =
                        current_view.borrow().clone();
                    let still_current = standard_profile_request_is_current(
                        &request,
                        pending_standard
                            .read()
                            .as_ref()
                            .map(|pending| pending.operation_id),
                        &current_owner,
                        current_scope().as_ref(),
                        &current_snapshot,
                        &selection(),
                        &current_definition,
                        scope_generation(),
                        selection_generation(),
                        workspace(),
                        accepted_owner_is_current(&request.owner),
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
                }) else {
                    return;
                };
            }));
        }
    }));
    let selector_load_standard = load_standard.clone();
    let action_load_standard = load_standard.clone();
    rsx! {
        section { class: "m1-parts-fit-editor", "aria-label": "Mechanical fit profile editor",
            header {
                div {
                    h2 { "{definition.name} fit" }
                    p { "Save the fit with this part. Every case using it inherits the profile." }
                }
                button { class: "m1-secondary", r#type: "button", onclick: move |_| on_close.call(()), "Cancel" }
            }
            if show_family_selector {
                label { class: "m1-parts-fit-field",
                    span { "Switch fit family" }
                    select {
                        "aria-label": "{family_selector_label}",
                        value: profile.switch_family.map(family_key).unwrap_or(""),
                        disabled: pending_standard.read().is_some(),
                        onchange: move |event| {
                            let load_standard = selector_load_standard.clone();
                            dispatch_standard_profile_family(&event.value(), move |family| {
                                (load_standard.borrow_mut())(family);
                            });
                        },
                        option { value: "", "Select a standard family" }
                        option { value: "mx", "MX" }
                        option { value: "choc-v1", "Choc v1" }
                        option { value: "choc-v2", "Choc v2" }
                    }
                }
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
            if show_standard_action {
                button {
                    class: "m1-secondary",
                    r#type: "button",
                    "aria-label": "{standard_action_label}",
                    disabled: pending_standard.read().is_some(),
                    onclick: move |_| {
                        if let Some(family) = draft().switch_family {
                            (action_load_standard.borrow_mut())(family);
                        }
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
                "aria-label": "{save_profile_label}",
                disabled: pending_standard.read().is_some(),
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
                    "aria-label": "Add {add_label}",
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
