//! Definition-level module profile draft and accepted edit path.
use super::model_transform::{
    ModelTransformFieldset, ModelVector, model_transform_is_valid, model_vector, new_model,
    set_model_axis,
};
use super::{PartsSelection, PartsSelectionGeneration};
use crate::parts_custom_definition::{PartsOwnerGuard, replacement_commit};
use crate::{presentation::WorkspaceState, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_model::model::{
    CaseOpening, EditOperation, EncoderDriver, HardwareOutput, ModuleDefinition, ModuleVolume,
    PartModel, RotaryProfile, Vec2,
};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, TOUCH_CLS, TextCommitField, ToggleField,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
struct ModuleProfileDraft {
    volumes: Vec<ModuleVolume>,
    openings: Vec<ModuleVolume>,
    models: Vec<PartModel>,
}

impl ModuleProfileDraft {
    fn from_definition(definition: &ModuleDefinition) -> Self {
        Self {
            volumes: definition.volumes.clone(),
            openings: definition.openings.clone(),
            models: definition.models.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ModuleVolumeDraft {
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    depth: Option<f64>,
    z: Option<f64>,
    height: Option<f64>,
    source: String,
    purpose: String,
    qualified: bool,
}

impl Default for ModuleVolumeDraft {
    fn default() -> Self {
        Self {
            x: Some(0.0),
            y: Some(0.0),
            width: None,
            depth: None,
            z: None,
            height: None,
            source: String::new(),
            purpose: "occupied".into(),
            qualified: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ModuleRotaryDraft {
    a: String,
    b: String,
    common: String,
    steps: Option<u16>,
    triggers_per_rotation: Option<u16>,
    driver: String,
}

impl ModuleRotaryDraft {
    fn from_profile(profile: Option<&RotaryProfile>) -> Self {
        Self {
            a: profile.map(|profile| profile.a.clone()).unwrap_or_default(),
            b: profile.map(|profile| profile.b.clone()).unwrap_or_default(),
            common: profile
                .map(|profile| profile.common.clone())
                .unwrap_or_default(),
            steps: profile.and_then(|profile| profile.steps),
            triggers_per_rotation: profile.and_then(|profile| profile.triggers_per_rotation),
            driver: profile
                .and_then(|profile| profile.driver.as_ref())
                .map(|driver| match driver {
                    EncoderDriver::Ec11 => "ec11".into(),
                })
                .unwrap_or_default(),
        }
    }
}

#[derive(Clone)]
struct ModuleProfileError {
    owner: PartsOwnerGuard,
    message: String,
}

fn module_profile_owner_is_current(
    runtime: &Runtime,
    owner: &PartsOwnerGuard,
    selected: &PartsSelection,
    selection_generation: u64,
    workspace: &Signal<Workspace>,
) -> Option<AcceptedSnapshot> {
    let snapshot = runtime.model().accepted?;
    owner
        .is_current(
            &snapshot,
            runtime.scope(),
            selected(),
            Some(selection_generation),
            None,
            Some(workspace()),
        )
        .then_some(snapshot)
}

fn module_profile_resolver(
    owner: PartsOwnerGuard,
    project_owned_at_start: bool,
    original: ModuleDefinition,
    draft: ModuleProfileDraft,
    rotary: Option<Option<RotaryProfile>>,
    reviewed: bool,
) -> EditResolver {
    EditResolver::new(
        "parts-module-profile",
        move |snapshot: &AcceptedSnapshot| {
            if snapshot.session_epoch != owner.session_epoch
                || snapshot.document.id != owner.document_id
            {
                return Resolution::Retire("The selected module's project changed.".into());
            }
            let resolve = || -> Result<ModuleDefinition, String> {
                let latest_project = snapshot
                    .document
                    .module_definitions
                    .iter()
                    .find(|definition| definition.id == original.id);
                if project_owned_at_start && latest_project.is_none() {
                    return Err(
            "This project-owned module definition was removed. Re-select it before saving.".into(),
        );
                }
                let mut latest = latest_project.cloned().unwrap_or_else(|| original.clone());
                if latest.source != original.source
                    || latest.circuit != original.circuit
                    || latest.interfaces != original.interfaces
                {
                    return Err("The module source, circuit, or connector mapping changed. Re-select it before editing its profile.".into());
                }
                if let Some(rotary) = &rotary {
                    latest.electrical.rotary_profile = rotary.clone();
                } else {
                    latest.volumes = draft.volumes.clone();
                    latest.openings = draft.openings.clone();
                    latest.models = draft.models.clone();
                }
                if reviewed
                    && !latest.volumes.is_empty()
                    && latest
                        .volumes
                        .iter()
                        .chain(&latest.openings)
                        .all(|volume| volume.qualified)
                {
                    latest.gates.retain(|gate| {
                        !(gate.output == HardwareOutput::Mechanical
                            && gate.code == "assembled-envelope")
                    });
                }
                if latest
                    .models
                    .iter()
                    .any(|model| !model_transform_is_valid(model))
                {
                    return Err(
                        "Model transforms need finite coordinates and positive scale.".into(),
                    );
                }
                Ok(latest)
            };
            match resolve() {
                Ok(latest)
                    if snapshot
                        .document
                        .module_definitions
                        .iter()
                        .any(|definition| definition == &latest) =>
                {
                    Resolution::Unchanged
                }
                Ok(latest) => replacement_commit(
                    EditOperation::SetModuleDefinition { definition: latest },
                    vec![original.id.clone()],
                ),
                Err(reason) => Resolution::Retire(reason),
            }
        },
    )
}

/// A committed numeric field only counts as a pulse count when it is a whole u16.
fn whole_u16(number: f64) -> Option<u16> {
    (number.fract() == 0.0 && (0.0..=f64::from(u16::MAX)).contains(&number))
        .then_some(number as u16)
}

fn rotary_profile_from_draft(draft: &ModuleRotaryDraft) -> Result<RotaryProfile, String> {
    let a = draft.a.trim();
    let b = draft.b.trim();
    let common = draft.common.trim();
    let steps = draft.steps;
    let triggers_per_rotation = draft.triggers_per_rotation;
    if a.is_empty()
        || b.is_empty()
        || common.is_empty()
        || a == b
        || a == common
        || b == common
        || draft.driver != "ec11"
        || steps.is_none_or(|value| value == 0)
        || triggers_per_rotation.is_none_or(|value| value == 0)
    {
        return Err("Enter distinct A, B and common terminals and positive whole-number pulses and actions per rotation.".into());
    }
    Ok(RotaryProfile {
        a: a.into(),
        b: b.into(),
        common: common.into(),
        steps,
        triggers_per_rotation,
        driver: Some(EncoderDriver::Ec11),
    })
}

#[component]
pub fn ModuleProfileEditor(
    snapshot: AcceptedSnapshot,
    definition: ModuleDefinition,
    project_owned: bool,
    scope: Option<Scope>,
    selected: PartsSelection,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let selection_generation = use_context::<PartsSelectionGeneration>().0;
    let workspace = use_context::<WorkspaceState>().0;
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let baseline = ModuleProfileDraft::from_definition(&definition);
    let baseline_rotary =
        ModuleRotaryDraft::from_profile(definition.electrical.rotary_profile.as_ref());
    let owner = PartsOwnerGuard::capture(
        &snapshot,
        scope.clone(),
        &format!("module:{}", definition.id),
    )
    .generations(Some(selection_generation()), None);
    let mut draft = use_signal(|| baseline.clone());
    let mut rotary = use_signal(|| baseline_rotary.clone());
    let mut profile_dirty = use_signal(|| false);
    let mut rotary_dirty = use_signal(|| false);
    let mut shape = use_signal(ModuleVolumeDraft::default);
    let mut reviewed = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    let mut error = use_signal(|| None::<ModuleProfileError>);
    let draft_owner = owner.clone();
    let mut processed_owner = use_signal(|| None);
    use_effect(use_reactive((&draft_owner,), {
        let baseline = baseline.clone();
        let baseline_rotary = baseline_rotary.clone();
        move |(next_owner,)| {
            if processed_owner.peek().as_ref() == Some(&next_owner) {
                return;
            }
            processed_owner.set(Some(next_owner));
            draft.set(baseline.clone());
            rotary.set(baseline_rotary.clone());
            profile_dirty.set(false);
            rotary_dirty.set(false);
            error.set(None);
            failure.set(None);
        }
    }));
    use_effect(use_reactive((&baseline, &baseline_rotary), {
        let mut draft = draft;
        let mut rotary = rotary;
        let mut reviewed = reviewed;
        move |(next_profile, next_rotary)| {
            // New typing can return to the previous accepted value while a save waits.
            // Only a clean draft follows accepted revisions, including Undo and Redo.
            if !profile_dirty() {
                draft.set(next_profile);
            }
            if !rotary_dirty() {
                rotary.set(next_rotary);
            }
            reviewed.set(false);
        }
    }));
    let current_owner = module_profile_owner_is_current(
        &runtime,
        &owner,
        &selected,
        selection_generation(),
        &workspace,
    );
    let visible_error = error()
        .filter(|feedback| feedback.owner == owner && current_owner.is_some())
        .map(|feedback| feedback.message)
        .or_else(|| current_owner.as_ref().and_then(|_| failure()));
    let runtime_for_add = runtime.clone();
    let add_volume = {
        let mut draft = draft;
        let mut shape = shape;
        let mut error = error;
        let owner = owner.clone();
        let selected = selected;
        move |_| {
            if module_profile_owner_is_current(
                &runtime_for_add,
                &owner,
                &selected,
                selection_generation(),
                &workspace,
            )
            .is_none()
            {
                return;
            }
            let shape_value = shape();
            let (Some(x), Some(y), Some(width), Some(depth), Some(z), Some(height)) = (
                shape_value.x,
                shape_value.y,
                shape_value.width,
                shape_value.depth,
                shape_value.z,
                shape_value.height,
            ) else {
                error.set(Some(ModuleProfileError {
                    owner: owner.clone(),
                    message: "Enter measured dimensions, a Z position and source evidence.".into(),
                }));
                return;
            };
            if [x, y, width, depth, z, height]
                .iter()
                .any(|value| !value.is_finite())
                || width <= 0.0
                || depth <= 0.0
                || height <= 0.0
                || shape_value.source.trim().is_empty()
            {
                error.set(Some(ModuleProfileError {
                    owner: owner.clone(),
                    message:
                        "Enter positive finite dimensions, a Z position and dimension evidence."
                            .into(),
                }));
                return;
            }
            let half_width = width / 2.0;
            let half_depth = depth / 2.0;
            let id = loop {
                let candidate = format!("volume/parts-{}", runtime_for_add.operation().0);
                let current = draft();
                if current
                    .volumes
                    .iter()
                    .chain(&current.openings)
                    .all(|volume| volume.id != candidate)
                {
                    break candidate;
                }
            };
            let volume = ModuleVolume {
                id,
                geometry: CaseOpening {
                    points: vec![
                        Vec2 {
                            x: x - half_width,
                            y: y - half_depth,
                        },
                        Vec2 {
                            x: x + half_width,
                            y: y - half_depth,
                        },
                        Vec2 {
                            x: x + half_width,
                            y: y + half_depth,
                        },
                        Vec2 {
                            x: x - half_width,
                            y: y + half_depth,
                        },
                    ],
                    z,
                    height,
                },
                purpose: shape_value.purpose,
                source: shape_value.source.trim().into(),
                qualified: shape_value.qualified,
            };
            profile_dirty.set(true);
            draft.with_mut(|draft| {
                if volume.purpose == "opening" {
                    draft.openings.push(volume);
                } else {
                    draft.volumes.push(volume);
                }
            });
            shape.set(ModuleVolumeDraft::default());
            error.set(None);
        }
    };

    let submit_profile = use_callback({
        let runtime = runtime.clone();
        let owner = owner.clone();
        let original = definition.clone();
        let baseline = baseline.clone();
        let draft = draft;
        let rotary = rotary;
        let mut rotary = rotary;
        let mut draft = draft;
        let mut failure = failure;
        let mut error = error;
        let selected = selected;
        move |rotary_override: Option<Option<RotaryProfile>>| {
            if module_profile_owner_is_current(
                &runtime,
                &owner,
                &selected,
                selection_generation(),
                &workspace,
            )
            .is_none()
            {
                return;
            }
            let rotary_override = match rotary_override {
                Some(_) => match rotary_profile_from_draft(&rotary()) {
                    Ok(profile) => Some(Some(profile)),
                    Err(message) => {
                        error.set(Some(ModuleProfileError {
                            owner: owner.clone(),
                            message,
                        }));
                        return;
                    }
                },
                None => None,
            };
            let (profile_draft, reviewed) = if rotary_override.is_some() {
                (baseline.clone(), false)
            } else {
                (draft(), reviewed())
            };
            let rotary_save = rotary_override.is_some();
            let resolver = module_profile_resolver(
                owner.clone(),
                project_owned,
                original.clone(),
                profile_draft,
                rotary_override,
                reviewed,
            );
            let result = runtime.commit("parts-module-profile", resolver);
            let accepted = runtime.model().accepted.and_then(|snapshot| {
                snapshot
                    .document
                    .module_definitions
                    .iter()
                    .find(|definition| definition.id == original.id)
                    .cloned()
            });
            if rotary_save {
                rotary_dirty.set(false);
                if let Some(definition) = accepted {
                    rotary.set(ModuleRotaryDraft::from_profile(
                        definition.electrical.rotary_profile.as_ref(),
                    ));
                }
            } else {
                profile_dirty.set(false);
                if let Some(definition) = accepted {
                    draft.set(ModuleProfileDraft::from_definition(&definition));
                }
            }
            error.set(None);
            failure.set(result.err());
        }
    });

    let candidate_models = definition.candidate_models.clone();
    let attach_candidate = use_callback({
        let candidates = candidate_models.clone();
        move |asset_id: String| {
            if candidates
                .iter()
                .any(|candidate| candidate.asset_id == asset_id)
            {
                profile_dirty.set(true);
                draft.with_mut(|draft| {
                    if !draft.models.iter().any(|model| model.asset_id == asset_id) {
                        draft.models.push(new_model(asset_id));
                    }
                });
            }
        }
    });
    let current_draft = draft();
    let rotary_profile_exists = definition.electrical.rotary_profile.is_some();
    rsx! {
        InspectorSection {
            title: "Assembly geometry",
            badge: format!("{} volumes · {} openings", current_draft.volumes.len(), current_draft.openings.len()),
            default_open: false,
            p { "Coordinates use the module PCB midplane. Positive Z faces the module front. Unknown dimensions remain unqualified." }
            if rotary_profile_exists {
                fieldset { class: "grid gap-2",
                    legend { "Rotary encoder profile" }
                    p { "The source identifies its terminals and EC11 driver. Enter confirmed pulses and actions per rotation before expecting firmware export to qualify this module." }
                    div { class: "grid gap-2",
                        TextCommitField { label: "A terminal", aria_label: "Rotary A terminal", value: "{rotary().a}", on_commit: move |value| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.a = value); } }
                        TextCommitField { label: "B terminal", aria_label: "Rotary B terminal", value: "{rotary().b}", on_commit: move |value| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.b = value); } }
                        TextCommitField { label: "Common terminal", aria_label: "Rotary common terminal", value: "{rotary().common}", on_commit: move |value| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.common = value); } }
                        SelectField::<String> {
                            label: "Driver",
                            aria_label: "Rotary driver",
                            value: "{rotary().driver}",
                            options: vec![("".to_owned(), "Choose driver…".to_owned()), ("ec11".to_owned(), "EC11".to_owned())],
                            on_commit: move |value: String| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.driver = value); },
                        }
                        NumericField { label: "Pulses per rotation", aria_label: "Rotary pulses per rotation", min: Some(1.0), step: Some(1.0), value: rotary().steps.map(f64::from).unwrap_or_default(), mixed: rotary().steps.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.steps = whole_u16(number)); } }
                        NumericField { label: "Actions per rotation", aria_label: "Rotary actions per rotation", min: Some(1.0), step: Some(1.0), value: rotary().triggers_per_rotation.map(f64::from).unwrap_or_default(), mixed: rotary().triggers_per_rotation.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.triggers_per_rotation = whole_u16(number)); } }
                    }
                    Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: move |_| submit_profile.call(Some(None)), "Save rotary profile" }
                }
            }
            for (index, volume) in current_draft.volumes.iter().enumerate() {
                div { class: "flex flex-wrap items-center justify-between gap-2", key: "volume-{volume.id}",
                    span { "{volume.purpose} · {volume.geometry.height} mm · " if volume.qualified { "Reviewed" } else { "Unreviewed" } " · {volume.source}" }
                    Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, aria_label: "Remove measured volume {index + 1}", onclick: move |_| {
                        profile_dirty.set(true);
                        draft.with_mut(|draft| if index < draft.volumes.len() { draft.volumes.remove(index); });
                        error.set(None);
                    }, "Remove" }
                }
            }
            for (index, opening) in current_draft.openings.iter().enumerate() {
                div { class: "flex flex-wrap items-center justify-between gap-2", key: "opening-{opening.id}",
                    span { "Opening · {opening.geometry.height} mm · " if opening.qualified { "Reviewed" } else { "Unreviewed" } " · {opening.source}" }
                    Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, aria_label: "Remove functional opening {index + 1}", onclick: move |_| {
                        profile_dirty.set(true);
                        draft.with_mut(|draft| if index < draft.openings.len() { draft.openings.remove(index); });
                        error.set(None);
                    }, "Remove" }
                }
            }
            SelectField::<String> {
                label: "Volume purpose",
                aria_label: "Module volume purpose",
                value: "{shape().purpose}",
                options: vec![("occupied".to_owned(), "Component body".to_owned()), ("support".to_owned(), "Mounting hardware".to_owned()), ("service".to_owned(), "Cable / service space".to_owned()), ("opening".to_owned(), "Functional opening".to_owned())],
                on_commit: move |value: String| shape.with_mut(|shape| shape.purpose = value),
            }
            div { class: "grid grid-cols-2 gap-2",
                NumericField { label: "X · mm", aria_label: "Module volume X", step: Some(0.1), value: shape().x.unwrap_or_default(), mixed: shape().x.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| shape.with_mut(|shape| shape.x = Some(number)) }
                NumericField { label: "Y · mm", aria_label: "Module volume Y", step: Some(0.1), value: shape().y.unwrap_or_default(), mixed: shape().y.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| shape.with_mut(|shape| shape.y = Some(number)) }
                NumericField { label: "Width · mm", aria_label: "Module volume width", min: Some(0.1), step: Some(0.1), value: shape().width.unwrap_or_default(), mixed: shape().width.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| shape.with_mut(|shape| shape.width = Some(number)) }
                NumericField { label: "Depth · mm", aria_label: "Module volume depth", min: Some(0.1), step: Some(0.1), value: shape().depth.unwrap_or_default(), mixed: shape().depth.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| shape.with_mut(|shape| shape.depth = Some(number)) }
                NumericField { label: "Z from midplane · mm", aria_label: "Module volume Z", step: Some(0.1), value: shape().z.unwrap_or_default(), mixed: shape().z.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| shape.with_mut(|shape| shape.z = Some(number)) }
                NumericField { label: "Height · mm", aria_label: "Module volume height", min: Some(0.1), step: Some(0.1), value: shape().height.unwrap_or_default(), mixed: shape().height.is_none(), placeholder: "Enter a number", on_commit: move |number: f64| shape.with_mut(|shape| shape.height = Some(number)) }
            }
            TextCommitField { label: "Dimension evidence", aria_label: "Module volume evidence", value: "{shape().source}", on_commit: move |value| shape.with_mut(|shape| shape.source = value) }
            ToggleField {
                label: "Dimensions and datum reviewed", checked: shape().qualified,
                on_commit: move |checked| shape.with_mut(|shape| shape.qualified = checked),
            }
            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: add_volume, "Add measured volume" }
            SelectField::<String> {
                label: "Attach candidate model", aria_label: "Module candidate model", value: String::new(),
                placeholder: "Select source model…",
                disabled: candidate_models.iter().all(|candidate| current_draft.models.iter().any(|model| model.asset_id == candidate.asset_id)),
                options: candidate_models.iter().filter(|candidate| !current_draft.models.iter().any(|model| model.asset_id == candidate.asset_id))
                    .map(|candidate| (candidate.asset_id.clone(), candidate.name.clone())).collect(),
                on_commit: attach_candidate,
            }
            for (model_index, model) in current_draft.models.iter().enumerate() {
                fieldset { class: "grid gap-2", key: "model-{model.asset_id}-{model_index}",
                    legend { "{model.asset_id}" }
                    for (vector_name, field) in [("offset", ModelVector::Offset), ("rotation", ModelVector::Rotation), ("scale", ModelVector::Scale)] {
                        ModelTransformFieldset { title: vector_name.to_owned(), value: model_vector(model, field), step: Some(0.1),
                            labels: [format!("{vector_name} x"), format!("{vector_name} y"), format!("{vector_name} z")],
                            aria_labels: Some([
                                format!("Module model {} {vector_name} x", model_index + 1),
                                format!("Module model {} {vector_name} y", model_index + 1),
                                format!("Module model {} {vector_name} z", model_index + 1),
                            ]),
                            on_commit: move |(axis, value)| {
                                profile_dirty.set(true);
                                draft.with_mut(|draft| if let Some(model) = draft.models.get_mut(model_index) {
                                    set_model_axis(model, field, axis, value);
                                });
                            },
                        }
                    }
                    Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, aria_label: "Remove module model {model_index + 1}", onclick: move |_| {
                        profile_dirty.set(true);
                        draft.with_mut(|draft| if model_index < draft.models.len() { draft.models.remove(model_index); });
                        error.set(None);
                    }, "Remove model" }
                }
            }
            ToggleField {
                label: "Complete assembly, mounts, functional openings and cable clearance reviewed", checked: reviewed(),
                on_commit: move |checked| reviewed.set(checked),
            }
            p { "Saving a candidate model retains its alignment review. Electrical repairs, driver support and other missing evidence keep their own blockers." }
            Button { variant: ButtonVariant::Default, class: TOUCH_CLS, disabled: current_owner.is_none(), onclick: move |_| submit_profile.call(None), "Save project module profile" }
            if let Some(message) = visible_error { Alert { variant: AlertVariant::Destructive, "{message}" } }
        }
    }
}
