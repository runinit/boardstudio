//! Definition-level module profile draft and accepted edit path.
use super::{PartsSelection, PartsSelectionGeneration};
use crate::{presentation::WorkspaceState, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, Event, Scope, SessionEpoch, TerminalOutcome};
use boardstudio_core::model::{
    CaseOpening, EditCommand, EditOperation, EditPhase, EncoderDriver, HardwareOutput,
    ModuleDefinition, ModuleVolume, PartModel, RotaryProfile, Vec2, Vec3,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
struct ModuleProfileOwner {
    session_epoch: SessionEpoch,
    document_id: String,
    scope: Option<Scope>,
    definition_id: String,
    selection_generation: u64,
    project_owned_at_start: bool,
}

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
    x: String,
    y: String,
    width: String,
    depth: String,
    z: String,
    height: String,
    source: String,
    purpose: String,
    qualified: bool,
}

impl Default for ModuleVolumeDraft {
    fn default() -> Self {
        Self {
            x: "0".into(),
            y: "0".into(),
            width: String::new(),
            depth: String::new(),
            z: String::new(),
            height: String::new(),
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
    steps: String,
    triggers_per_rotation: String,
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
            steps: profile
                .and_then(|profile| profile.steps)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            triggers_per_rotation: profile
                .and_then(|profile| profile.triggers_per_rotation)
                .map(|value| value.to_string())
                .unwrap_or_default(),
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
struct PendingModuleProfile {
    owner: ModuleProfileOwner,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

#[derive(Clone)]
struct ModuleProfileError {
    owner: ModuleProfileOwner,
    message: String,
}

fn module_profile_owner(
    snapshot: &AcceptedSnapshot,
    definition: &ModuleDefinition,
    scope: Option<Scope>,
    selection_generation: u64,
    project_owned: bool,
) -> ModuleProfileOwner {
    ModuleProfileOwner {
        session_epoch: snapshot.session_epoch,
        document_id: snapshot.document.id.clone(),
        scope,
        definition_id: definition.id.clone(),
        selection_generation,
        project_owned_at_start: project_owned,
    }
}

fn module_profile_owner_is_current(
    runtime: &Runtime,
    owner: &ModuleProfileOwner,
    selected: &PartsSelection,
    selection_generation: u64,
    workspace: &Signal<&'static str>,
) -> Option<AcceptedSnapshot> {
    if workspace() != "Parts"
        || owner.scope != runtime.scope()
        || owner.selection_generation != selection_generation
        || selected()
            != Some((
                owner.scope.clone(),
                format!("module:{}", owner.definition_id),
            ))
    {
        return None;
    }
    let snapshot = runtime.model().accepted?;
    (snapshot.session_epoch == owner.session_epoch && snapshot.document.id == owner.document_id)
        .then_some(snapshot)
}

fn merge_module_profile_field<T: Clone + PartialEq>(
    label: &str,
    original: &T,
    draft: &T,
    latest: &T,
) -> Result<T, String> {
    if draft == original {
        Ok(latest.clone())
    } else if latest == original || latest == draft {
        Ok(draft.clone())
    } else {
        Err(format!(
            "The module {label} changed while its profile editor was open. Reopen the profile before saving."
        ))
    }
}

fn prepare_module_profile_edit(
    runtime: &Runtime,
    owner: &ModuleProfileOwner,
    original: &ModuleDefinition,
    draft: &ModuleProfileDraft,
    rotary: Option<Option<RotaryProfile>>,
    reviewed: bool,
    selected: &PartsSelection,
    selection_generation: u64,
    workspace: &Signal<&'static str>,
) -> Result<Event, String> {
    let snapshot =
        module_profile_owner_is_current(runtime, owner, selected, selection_generation, workspace)
            .ok_or_else(|| "The selected module changed. Re-select it before saving.".to_owned())?;
    let latest_project = snapshot
        .document
        .module_definitions
        .iter()
        .find(|definition| definition.id == owner.definition_id);
    if owner.project_owned_at_start && latest_project.is_none() {
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
    latest.volumes = merge_module_profile_field(
        "measured volumes",
        &original.volumes,
        &draft.volumes,
        &latest.volumes,
    )?;
    latest.openings = merge_module_profile_field(
        "openings",
        &original.openings,
        &draft.openings,
        &latest.openings,
    )?;
    latest.models = merge_module_profile_field(
        "model bindings",
        &original.models,
        &draft.models,
        &latest.models,
    )?;
    if let Some(rotary) = rotary {
        latest.electrical.rotary_profile = merge_module_profile_field(
            "rotary profile",
            &original.electrical.rotary_profile,
            &rotary,
            &latest.electrical.rotary_profile,
        )?;
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
            !(gate.output == HardwareOutput::Mechanical && gate.code == "assembled-envelope")
        });
    }
    if latest.models.iter().any(|model| {
        [
            model.offset.x,
            model.offset.y,
            model.offset.z,
            model.rotation.x,
            model.rotation.y,
            model.rotation.z,
            model.scale.x,
            model.scale.y,
            model.scale.z,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || [model.scale.x, model.scale.y, model.scale.z]
                .iter()
                .any(|value| *value <= 0.0)
    }) {
        return Err("Model transforms need finite coordinates and positive scale.".into());
    }
    let operation_id = runtime.operation();
    Ok(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: snapshot.document.revision,
            transaction_id: format!("parts-module-profile-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![owner.definition_id.clone()],
            operation: EditOperation::SetModuleDefinition { definition: latest },
        },
    })
}

fn rotary_profile_from_draft(draft: &ModuleRotaryDraft) -> Result<RotaryProfile, String> {
    let a = draft.a.trim();
    let b = draft.b.trim();
    let common = draft.common.trim();
    let steps = draft.steps.trim().parse::<u16>().ok();
    let triggers_per_rotation = draft.triggers_per_rotation.trim().parse::<u16>().ok();
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
    let owner = module_profile_owner(
        &snapshot,
        &definition,
        scope.clone(),
        selection_generation(),
        project_owned,
    );
    let mut draft = use_signal(|| baseline.clone());
    let mut rotary = use_signal(|| baseline_rotary.clone());
    let synced = use_signal(|| (baseline.clone(), baseline_rotary.clone()));
    let mut shape = use_signal(ModuleVolumeDraft::default);
    let mut reviewed = use_signal(|| false);
    let pending = use_signal(|| None::<PendingModuleProfile>);
    let mut error = use_signal(|| None::<ModuleProfileError>);
    use_effect(use_reactive((&baseline, &baseline_rotary), {
        let mut draft = draft;
        let mut rotary = rotary;
        let mut synced = synced;
        let mut reviewed = reviewed;
        move |(next_profile, next_rotary)| {
            let next = (next_profile.clone(), next_rotary.clone());
            if synced() != next {
                draft.set(next_profile);
                rotary.set(next_rotary);
                synced.set(next);
                reviewed.set(false);
            }
        }
    }));
    use_effect(use_reactive((&owner, &version()), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut error = error;
        let selected = selected;
        let selection_generation = selection_generation;
        let workspace = workspace;
        move |(owner, _)| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow().clone() else {
                if module_profile_owner_is_current(
                    &runtime,
                    &waiting.owner,
                    &selected,
                    selection_generation(),
                    &workspace,
                )
                .is_none()
                {
                    pending.set(None);
                    error.set(None);
                }
                return;
            };
            pending.set(None);
            if module_profile_owner_is_current(
                &runtime,
                &waiting.owner,
                &selected,
                selection_generation(),
                &workspace,
            )
            .is_none()
                || waiting.owner.definition_id != owner.definition_id
            {
                error.set(None);
                return;
            }
            match outcome {
                TerminalOutcome::Completed => error.set(None),
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::PersistenceFailed(message)
                | TerminalOutcome::BlockedByRecovery(message)
                | TerminalOutcome::ExecutorFailed(message) => {
                    error.set(Some(ModuleProfileError {
                        owner: waiting.owner,
                        message,
                    }));
                }
                TerminalOutcome::Cancelled
                | TerminalOutcome::Closed
                | TerminalOutcome::Superseded => error.set(None),
            }
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
        .filter(|feedback| {
            feedback.owner.definition_id == owner.definition_id
                && feedback.owner.session_epoch == owner.session_epoch
                && feedback.owner.document_id == owner.document_id
                && feedback.owner.scope == owner.scope
                && feedback.owner.selection_generation == owner.selection_generation
                && current_owner.is_some()
        })
        .map(|feedback| feedback.message);
    let busy = pending().is_some();
    let runtime_for_add = runtime.clone();
    let add_volume = {
        let mut draft = draft;
        let mut shape = shape;
        let mut error = error;
        let owner = owner.clone();
        let selected = selected;
        let workspace = workspace;
        let selection_generation = selection_generation;
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
            let values = [
                shape().x.parse::<f64>(),
                shape().y.parse::<f64>(),
                shape().width.parse::<f64>(),
                shape().depth.parse::<f64>(),
                shape().z.parse::<f64>(),
                shape().height.parse::<f64>(),
            ];
            let [Ok(x), Ok(y), Ok(width), Ok(depth), Ok(z), Ok(height)] = values else {
                error.set(Some(ModuleProfileError {
                    owner: owner.clone(),
                    message: "Enter measured dimensions, a Z position and source evidence.".into(),
                }));
                return;
            };
            let shape_value = shape();
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
        let mut pending = pending;
        let mut error = error;
        let selected = selected;
        let selection_generation = selection_generation;
        let workspace = workspace;
        move |rotary_override: Option<Option<RotaryProfile>>| {
            if busy {
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
            let event = match prepare_module_profile_edit(
                &runtime,
                &owner,
                &original,
                &profile_draft,
                rotary_override,
                reviewed,
                &selected,
                selection_generation(),
                &workspace,
            ) {
                Ok(event) => event,
                Err(message) => {
                    error.set(Some(ModuleProfileError {
                        owner: owner.clone(),
                        message,
                    }));
                    return;
                }
            };
            let Event::Edit { operation_id, .. } = &event else {
                return;
            };
            error.set(None);
            pending.set(Some(PendingModuleProfile {
                owner: owner.clone(),
                outcome: runtime.observe_operation(*operation_id),
            }));
            runtime.submit(event);
        }
    });

    let candidate_models = definition.candidate_models.clone();
    let candidate_models_for_select = candidate_models.clone();
    let current_draft = draft();
    let rotary_profile_exists = definition.electrical.rotary_profile.is_some();
    rsx! {
        details { class: "m1-module-profile-editor m1-generator-settings",
            summary { "Assembly geometry · {current_draft.volumes.len()} volumes · {current_draft.openings.len()} openings" }
            p { class: "m1-parts-empty", "Coordinates use the module PCB midplane. Positive Z faces the module front. Unknown dimensions remain unqualified." }
            if rotary_profile_exists {
                fieldset {
                    legend { "Rotary encoder profile" }
                    p { class: "m1-parts-empty", "The source identifies its terminals and EC11 driver. Enter confirmed pulses and actions per rotation before expecting firmware export to qualify this module." }
                    div { class: "m1-generator-fields",
                        label { class: "m1-generator-field", "A terminal", input { aria_label: "Rotary A terminal", value: "{rotary().a}", oninput: move |event| rotary.with_mut(|draft| draft.a = event.value()) } }
                        label { class: "m1-generator-field", "B terminal", input { aria_label: "Rotary B terminal", value: "{rotary().b}", oninput: move |event| rotary.with_mut(|draft| draft.b = event.value()) } }
                        label { class: "m1-generator-field", "Common terminal", input { aria_label: "Rotary common terminal", value: "{rotary().common}", oninput: move |event| rotary.with_mut(|draft| draft.common = event.value()) } }
                        label { class: "m1-generator-field", "Driver", select { aria_label: "Rotary driver", value: "{rotary().driver}", onchange: move |event| rotary.with_mut(|draft| draft.driver = event.value()), option { value: "", "Choose driver…" } option { value: "ec11", "EC11" } } }
                        label { class: "m1-generator-field", "Pulses per rotation", input { r#type: "number", min: "1", step: "1", aria_label: "Rotary pulses per rotation", value: "{rotary().steps}", oninput: move |event| rotary.with_mut(|draft| draft.steps = event.value()) } }
                        label { class: "m1-generator-field", "Actions per rotation", input { r#type: "number", min: "1", step: "1", aria_label: "Rotary actions per rotation", value: "{rotary().triggers_per_rotation}", oninput: move |event| rotary.with_mut(|draft| draft.triggers_per_rotation = event.value()) } }
                    }
                    button { r#type: "button", disabled: busy, onclick: move |_| submit_profile.call(Some(None)), "Save rotary profile" }
                }
            }
            for (index, volume) in current_draft.volumes.iter().enumerate() {
                div { class: "m1-module-profile-volume", key: "volume-{volume.id}",
                    span { "{volume.purpose} · {volume.geometry.height} mm · " if volume.qualified { "Reviewed" } else { "Unreviewed" } " · {volume.source}" }
                    button { r#type: "button", disabled: busy, aria_label: "Remove measured volume {index + 1}", onclick: move |_| {
                        draft.with_mut(|draft| if index < draft.volumes.len() { draft.volumes.remove(index); });
                        error.set(None);
                    }, "Remove" }
                }
            }
            for (index, opening) in current_draft.openings.iter().enumerate() {
                div { class: "m1-module-profile-volume", key: "opening-{opening.id}",
                    span { "Opening · {opening.geometry.height} mm · " if opening.qualified { "Reviewed" } else { "Unreviewed" } " · {opening.source}" }
                    button { r#type: "button", disabled: busy, aria_label: "Remove functional opening {index + 1}", onclick: move |_| {
                        draft.with_mut(|draft| if index < draft.openings.len() { draft.openings.remove(index); });
                        error.set(None);
                    }, "Remove" }
                }
            }
            label { class: "m1-generator-field", "Volume purpose", select { aria_label: "Module volume purpose", value: "{shape().purpose}", onchange: move |event| shape.with_mut(|shape| shape.purpose = event.value()),
                option { value: "occupied", "Component body" }
                option { value: "support", "Mounting hardware" }
                option { value: "service", "Cable / service space" }
                option { value: "opening", "Functional opening" }
            } }
            div { class: "m1-generator-fields",
                label { class: "m1-generator-field", "X · mm", input { r#type: "number", step: "0.1", aria_label: "Module volume X", value: "{shape().x}", oninput: move |event| shape.with_mut(|shape| shape.x = event.value()) } }
                label { class: "m1-generator-field", "Y · mm", input { r#type: "number", step: "0.1", aria_label: "Module volume Y", value: "{shape().y}", oninput: move |event| shape.with_mut(|shape| shape.y = event.value()) } }
                label { class: "m1-generator-field", "Width · mm", input { r#type: "number", min: "0.1", step: "0.1", aria_label: "Module volume width", value: "{shape().width}", oninput: move |event| shape.with_mut(|shape| shape.width = event.value()) } }
                label { class: "m1-generator-field", "Depth · mm", input { r#type: "number", min: "0.1", step: "0.1", aria_label: "Module volume depth", value: "{shape().depth}", oninput: move |event| shape.with_mut(|shape| shape.depth = event.value()) } }
                label { class: "m1-generator-field", "Z from midplane · mm", input { r#type: "number", step: "0.1", aria_label: "Module volume Z", value: "{shape().z}", oninput: move |event| shape.with_mut(|shape| shape.z = event.value()) } }
                label { class: "m1-generator-field", "Height · mm", input { r#type: "number", min: "0.1", step: "0.1", aria_label: "Module volume height", value: "{shape().height}", oninput: move |event| shape.with_mut(|shape| shape.height = event.value()) } }
            }
            label { class: "m1-generator-field", "Dimension evidence", input { aria_label: "Module volume evidence", value: "{shape().source}", oninput: move |event| shape.with_mut(|shape| shape.source = event.value()) } }
            label { class: "m1-module-profile-check", input { r#type: "checkbox", checked: shape().qualified, aria_label: "Dimensions and datum reviewed", onchange: move |event| shape.with_mut(|shape| shape.qualified = event.checked()) } "Dimensions and datum reviewed" }
            button { r#type: "button", disabled: busy, onclick: add_volume, "Add measured volume" }
            if !candidate_models.is_empty() {
                label { class: "m1-generator-field", "Attach candidate model", select { aria_label: "Module candidate model", value: "", disabled: busy, onchange: move |event| {
                    let asset_id = event.value();
                    if candidate_models_for_select.iter().any(|candidate| candidate.asset_id == asset_id) {
                        draft.with_mut(|draft| if !draft.models.iter().any(|model| model.asset_id == asset_id) {
                            draft.models.push(PartModel {
                                asset_id,
                                offset: Vec3::default(),
                                rotation: Vec3::default(),
                                scale: Vec3 { x: 1.0, y: 1.0, z: 1.0 },
                            });
                        });
                    }
                },
                    option { value: "", "Select source model…" }
                    for candidate in &candidate_models {
                        option { value: "{candidate.asset_id}", disabled: current_draft.models.iter().any(|model| model.asset_id == candidate.asset_id), "{candidate.name}" }
                    }
                } }
            }
            for (model_index, model) in current_draft.models.iter().enumerate() {
                fieldset { class: "m1-module-profile-model", key: "model-{model.asset_id}-{model_index}",
                    legend { "{model.asset_id}" }
                    for (vector_name, vector) in [("offset", model.offset), ("rotation", model.rotation), ("scale", model.scale)] {
                        div { class: "m1-generator-fields",
                            for (axis_name, value) in [("x", vector.x), ("y", vector.y), ("z", vector.z)] {
                                label { class: "m1-generator-field", "{vector_name} {axis_name}", input { r#type: "number", step: "0.1", aria_label: "Module model {model_index + 1} {vector_name} {axis_name}", value: "{value}", oninput: move |event| {
                                    let value = event.value().parse::<f64>().unwrap_or(f64::NAN);
                                    draft.with_mut(|draft| if let Some(model) = draft.models.get_mut(model_index) {
                                        let vector = match vector_name { "offset" => &mut model.offset, "rotation" => &mut model.rotation, _ => &mut model.scale };
                                        match axis_name { "x" => vector.x = value, "y" => vector.y = value, _ => vector.z = value }
                                    });
                                } } }
                            }
                        }
                    }
                    button { r#type: "button", disabled: busy, aria_label: "Remove module model {model_index + 1}", onclick: move |_| {
                        draft.with_mut(|draft| if model_index < draft.models.len() { draft.models.remove(model_index); });
                        error.set(None);
                    }, "Remove model" }
                }
            }
            label { class: "m1-module-profile-check", input { r#type: "checkbox", checked: reviewed(), aria_label: "Complete assembly, mounts, functional openings and cable clearance reviewed", onchange: move |event| reviewed.set(event.checked()) } "Complete assembly, mounts, functional openings and cable clearance reviewed" }
            p { class: "m1-parts-empty", "Saving a candidate model retains its alignment review. Electrical repairs, driver support and other missing evidence keep their own blockers." }
            button { class: "m1-generator-apply", r#type: "button", disabled: busy || current_owner.is_none(), onclick: move |_| submit_profile.call(None), "Save project module profile" }
            if busy { p { class: "m1-parts-loading", role: "status", "Saving module profile…" } }
            if let Some(message) = visible_error { p { class: "m1-parts-load-error", role: "alert", "{message}" } }
        }
    }
}
