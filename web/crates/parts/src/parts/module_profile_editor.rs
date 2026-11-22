//! Definition-level module profile draft and accepted edit path.
use super::editor_fields::{EditorNumberField, EditorSelect, EditorTextField};
use super::{PartsSelection, PartsSelectionGeneration};
use crate::parts_custom_definition::replacement_commit;
use crate::{presentation::WorkspaceState, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch};
use boardstudio_model::model::{
    CaseOpening, EditOperation, EncoderDriver, HardwareOutput, ModuleDefinition, ModuleVolume,
    PartModel, RotaryProfile, Vec2, Vec3,
};
use boardstudio_web_ui_shared::inspector_fields::CheckboxField;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
    checkbox::CheckboxState,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    combobox::{Combobox, ComboboxOption},
    field::FieldLabel,
    select::{Select, SelectOption},
    spinner::Spinner,
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

fn module_profile_resolver(
    owner: ModuleProfileOwner,
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
                    vec![owner.definition_id.clone()],
                ),
                Err(reason) => Resolution::Retire(reason),
            }
        },
    )
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
    let mut profile_dirty = use_signal(|| false);
    let mut rotary_dirty = use_signal(|| false);
    let mut shape = use_signal(ModuleVolumeDraft::default);
    let mut reviewed = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    let mut error = use_signal(|| None::<ModuleProfileError>);
    let draft_owner = (
        owner.session_epoch,
        owner.document_id.clone(),
        owner.scope.clone(),
        owner.definition_id.clone(),
        owner.selection_generation,
    );
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
        .filter(|feedback| {
            feedback.owner.definition_id == owner.definition_id
                && feedback.owner.session_epoch == owner.session_epoch
                && feedback.owner.document_id == owner.document_id
                && feedback.owner.scope == owner.scope
                && feedback.owner.selection_generation == owner.selection_generation
                && current_owner.is_some()
        })
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
                    .find(|definition| definition.id == owner.definition_id)
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
    let candidate_models_for_select = candidate_models.clone();
    let attach_candidate = use_callback({
        let candidates = candidate_models_for_select.clone();
        move |asset_id: String| {
            if candidates
                .iter()
                .any(|candidate| candidate.asset_id == asset_id)
            {
                profile_dirty.set(true);
                draft.with_mut(|draft| {
                    if !draft.models.iter().any(|model| model.asset_id == asset_id) {
                        draft.models.push(PartModel {
                            asset_id,
                            offset: Vec3::default(),
                            rotation: Vec3::default(),
                            scale: Vec3 {
                                x: 1.0,
                                y: 1.0,
                                z: 1.0,
                            },
                        });
                    }
                });
            }
        }
    });
    let candidate_id = format!(
        "module-candidate-model-{}",
        dioxus::core::current_scope_id().0
    );
    let candidate_value = use_memo(|| Some(String::new()));
    let current_draft = draft();
    let rotary_profile_exists = definition.electrical.rotary_profile.is_some();
    rsx! {
        Collapsible { keep_mounted: true, default_open: false,
            CollapsibleTrigger { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Assembly geometry · {current_draft.volumes.len()} volumes · {current_draft.openings.len()} openings" }
            CollapsibleContent { class: "data-[open=false]:hidden grid gap-3",
            p { "Coordinates use the module PCB midplane. Positive Z faces the module front. Unknown dimensions remain unqualified." }
            if rotary_profile_exists {
                fieldset { class: "grid gap-2",
                    legend { "Rotary encoder profile" }
                    p { "The source identifies its terminals and EC11 driver. Enter confirmed pulses and actions per rotation before expecting firmware export to qualify this module." }
                    div { class: "grid gap-2",
                        EditorTextField { label: "A terminal", aria_label: "Rotary A terminal", value: "{rotary().a}", oninput: move |event: FormEvent| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.a = event.value()); } }
                        EditorTextField { label: "B terminal", aria_label: "Rotary B terminal", value: "{rotary().b}", oninput: move |event: FormEvent| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.b = event.value()); } }
                        EditorTextField { label: "Common terminal", aria_label: "Rotary common terminal", value: "{rotary().common}", oninput: move |event: FormEvent| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.common = event.value()); } }
                        EditorSelect {
                            label: "Driver",
                            aria_label: "Rotary driver",
                            value: "{rotary().driver}",
                            options: vec![("".to_owned(), "Choose driver…".to_owned()), ("ec11".to_owned(), "EC11".to_owned())],
                            on_select: move |value: String| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.driver = value); },
                        }
                        EditorNumberField { label: "Pulses per rotation", aria_label: "Rotary pulses per rotation", min: "1", step: "1", value: "{rotary().steps}", oninput: move |event: FormEvent| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.steps = event.value()); } }
                        EditorNumberField { label: "Actions per rotation", aria_label: "Rotary actions per rotation", min: "1", step: "1", value: "{rotary().triggers_per_rotation}", oninput: move |event: FormEvent| { rotary_dirty.set(true); rotary.with_mut(|draft| draft.triggers_per_rotation = event.value()); } }
                    }
                    Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: move |_| submit_profile.call(Some(None)), "Save rotary profile" }
                }
            }
            for (index, volume) in current_draft.volumes.iter().enumerate() {
                div { class: "flex flex-wrap items-center justify-between gap-2", key: "volume-{volume.id}",
                    span { "{volume.purpose} · {volume.geometry.height} mm · " if volume.qualified { "Reviewed" } else { "Unreviewed" } " · {volume.source}" }
                    Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", aria_label: "Remove measured volume {index + 1}", onclick: move |_| {
                        profile_dirty.set(true);
                        draft.with_mut(|draft| if index < draft.volumes.len() { draft.volumes.remove(index); });
                        error.set(None);
                    }, "Remove" }
                }
            }
            for (index, opening) in current_draft.openings.iter().enumerate() {
                div { class: "flex flex-wrap items-center justify-between gap-2", key: "opening-{opening.id}",
                    span { "Opening · {opening.geometry.height} mm · " if opening.qualified { "Reviewed" } else { "Unreviewed" } " · {opening.source}" }
                    Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", aria_label: "Remove functional opening {index + 1}", onclick: move |_| {
                        profile_dirty.set(true);
                        draft.with_mut(|draft| if index < draft.openings.len() { draft.openings.remove(index); });
                        error.set(None);
                    }, "Remove" }
                }
            }
            EditorSelect {
                label: "Volume purpose",
                aria_label: "Module volume purpose",
                value: "{shape().purpose}",
                options: vec![("occupied".to_owned(), "Component body".to_owned()), ("support".to_owned(), "Mounting hardware".to_owned()), ("service".to_owned(), "Cable / service space".to_owned()), ("opening".to_owned(), "Functional opening".to_owned())],
                on_select: move |value: String| shape.with_mut(|shape| shape.purpose = value),
            }
            div { class: "grid grid-cols-2 gap-2",
                EditorNumberField { label: "X · mm", aria_label: "Module volume X", step: "0.1", value: "{shape().x}", oninput: move |event: FormEvent| shape.with_mut(|shape| shape.x = event.value()) }
                EditorNumberField { label: "Y · mm", aria_label: "Module volume Y", step: "0.1", value: "{shape().y}", oninput: move |event: FormEvent| shape.with_mut(|shape| shape.y = event.value()) }
                EditorNumberField { label: "Width · mm", aria_label: "Module volume width", min: "0.1", step: "0.1", value: "{shape().width}", oninput: move |event: FormEvent| shape.with_mut(|shape| shape.width = event.value()) }
                EditorNumberField { label: "Depth · mm", aria_label: "Module volume depth", min: "0.1", step: "0.1", value: "{shape().depth}", oninput: move |event: FormEvent| shape.with_mut(|shape| shape.depth = event.value()) }
                EditorNumberField { label: "Z from midplane · mm", aria_label: "Module volume Z", step: "0.1", value: "{shape().z}", oninput: move |event: FormEvent| shape.with_mut(|shape| shape.z = event.value()) }
                EditorNumberField { label: "Height · mm", aria_label: "Module volume height", min: "0.1", step: "0.1", value: "{shape().height}", oninput: move |event: FormEvent| shape.with_mut(|shape| shape.height = event.value()) }
            }
            EditorTextField { label: "Dimension evidence", aria_label: "Module volume evidence", value: "{shape().source}", oninput: move |event: FormEvent| shape.with_mut(|shape| shape.source = event.value()) }
            CheckboxField {
                label: "Dimensions and datum reviewed".to_owned(),
                id: format!("module-volume-qualified-{}", dioxus::core::current_scope_id().0),
                aria_label: Some("Dimensions and datum reviewed".to_owned()),
                checked: if shape().qualified { CheckboxState::Checked } else { CheckboxState::Unchecked },
                on_checked_change: move |state: CheckboxState| shape.with_mut(|shape| shape.qualified = bool::from(state)),
            }
            Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: add_volume, "Add measured volume" }
            if !candidate_models.is_empty() {
                FieldLabel { html_for: candidate_id.clone(), "Attach candidate model" }
                if candidate_models.len() > 12 {
                    Combobox::<String> {
                        class: "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch",
                        aria_label: "Module candidate model", list_aria_label: "Module candidate model",
                        value: Some(candidate_value.into()),
                        on_value_change: move |next: Option<String>| {
                            if let Some(asset_id) = next
                                && !asset_id.is_empty() {
                                    attach_candidate.call(asset_id);
                                }
                        },
                        for (index, candidate) in candidate_models.iter().enumerate() {
                            ComboboxOption::<String> { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", index, value: candidate.asset_id.clone(), text_value: candidate.name.clone(), disabled: current_draft.models.iter().any(|model| model.asset_id == candidate.asset_id), "{candidate.name}" }
                        }
                    }
                } else {
                    Select::<String> {
                        class: "max-[960px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch",
                        id: candidate_id.clone(),
                        aria_label: "Module candidate model",
                        value: Some(candidate_value.into()),
                        on_value_change: move |next: Option<String>| {
                            if let Some(asset_id) = next
                                && !asset_id.is_empty() {
                                    attach_candidate.call(asset_id);
                                }
                        },
                        SelectOption::<String> { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", index: 0usize, value: "".to_owned(), text_value: "Select source model…".to_owned(), "Select source model…" }
                        for (offset, candidate) in candidate_models.iter().enumerate() {
                            SelectOption::<String> { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", index: offset + 1, value: candidate.asset_id.clone(), text_value: candidate.name.clone(), disabled: current_draft.models.iter().any(|model| model.asset_id == candidate.asset_id), "{candidate.name}" }
                        }
                    }
                }
            }
            for (model_index, model) in current_draft.models.iter().enumerate() {
                fieldset { class: "grid gap-2", key: "model-{model.asset_id}-{model_index}",
                    legend { "{model.asset_id}" }
                    for (vector_name, vector) in [("offset", model.offset), ("rotation", model.rotation), ("scale", model.scale)] {
                        div { class: "grid grid-cols-3 gap-2",
                            for (axis_name, value) in [("x", vector.x), ("y", vector.y), ("z", vector.z)] {
                                EditorNumberField { label: "{vector_name} {axis_name}", step: "0.1", aria_label: "Module model {model_index + 1} {vector_name} {axis_name}", value: "{value}", oninput: move |event: FormEvent| {
                                    let value = event.value().parse::<f64>().unwrap_or(f64::NAN);
                                    profile_dirty.set(true);
                                    draft.with_mut(|draft| if let Some(model) = draft.models.get_mut(model_index) {
                                        let vector = match vector_name { "offset" => &mut model.offset, "rotation" => &mut model.rotation, _ => &mut model.scale };
                                        match axis_name { "x" => vector.x = value, "y" => vector.y = value, _ => vector.z = value }
                                    });
                                } }
                            }
                        }
                    }
                    Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", aria_label: "Remove module model {model_index + 1}", onclick: move |_| {
                        profile_dirty.set(true);
                        draft.with_mut(|draft| if model_index < draft.models.len() { draft.models.remove(model_index); });
                        error.set(None);
                    }, "Remove model" }
                }
            }
            CheckboxField {
                label: "Complete assembly, mounts, functional openings and cable clearance reviewed".to_owned(),
                id: format!("module-profile-reviewed-{}", dioxus::core::current_scope_id().0),
                aria_label: Some("Complete assembly, mounts, functional openings and cable clearance reviewed".to_owned()),
                checked: if reviewed() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                on_checked_change: move |state: CheckboxState| reviewed.set(bool::from(state)),
            }
            p { "Saving a candidate model retains its alignment review. Electrical repairs, driver support and other missing evidence keep their own blockers." }
            Button { variant: ButtonVariant::Default, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: current_owner.is_none(), onclick: move |_| submit_profile.call(None), "Save project module profile" }
            if let Some(message) = visible_error { Alert { variant: AlertVariant::Destructive, "{message}" } }
            }
        }
    }
}
