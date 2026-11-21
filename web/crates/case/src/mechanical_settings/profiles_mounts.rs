#![allow(unused_imports)]
//! Private read-only presentation and field intents for the Case mechanical stack.
//! Root owns accepted-state admission and commits every intent through Runtime.
pub use crate::mechanical_feedback::{
    MechanicalSettingsFeedback, MechanicalSettingsFeedbackState, MechanicalSettingsIdentity,
};
use crate::mechanical_pair::{MechanicalPairFeatureRow, MechanicalPairLinkOption, feature_label};
use crate::mechanical_settings_controller::use_bound_mechanical_field;
pub use boardstudio_core_mechanical::mechanical::change::{
    MechanicalChange as MechanicalSettingsPatch, MechanicalCriticalFitTextField,
    MechanicalDimension, MechanicalHardwareTextField, MechanicalMountCollection,
};
use boardstudio_model::model::{
    CaseOpening, GasketPlacement, HardwareTransport, InsertInstallation, InternalClosureHardware,
    MAX_COORDINATE_MM, MechanicalBattery, MechanicalBottomStyle, MechanicalBuiltinProfile,
    MechanicalCriticalFit, MechanicalExtraction, MechanicalFeatureState, MechanicalGasketAnchor,
    MechanicalGeometry, MechanicalHardwareSpecification, MechanicalMount, MechanicalOpening,
    MechanicalPartProcess, MechanicalPartProfile, MechanicalPurpose, MechanicalPurposeMapping,
    MechanicalStabilizerKind, MechanicalStabilizerOverride, MechanicalSwitchFamily, Mount,
    MountKind, PlateMethod, Pose2, ScrewDrive, ScrewHeadProfile, ScrewLengthDatum, Severity, Vec2,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::{Callout, CalloutVariant},
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput, InputGroupText, InputGroupTextarea},
    item::{Item, ItemActions, ItemContent, ItemDescription, ItemGroup},
    select::{Select, SelectOption},
    toggle_group::{ToggleGroup, ToggleItem},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use std::{future::Future, pin::Pin};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use super::*;
#[derive(Props, Clone, PartialEq)]
pub(crate) struct ManufacturingControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) targets: Rc<[MechanicalProcessTarget]>,
    pub(crate) stabilizers: Rc<[MechanicalStabilizerFit]>,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
pub(crate) fn ManufacturingControls(props: ManufacturingControlsProps) -> Element {
    let version = "2026-09-24";
    rsx! {
        section { class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", aria_label: "Manufacturing overrides",
            h3 { "Manufacturing overrides" }
            section { class: "grid gap-1.5", aria_label: "Per-part process overrides",
                h4 { "Per-part process overrides {props.targets.iter().filter(|target| target.process.is_some()).count()}" }
                for target in props.targets.iter() {
                    div { class: "grid gap-1.5", key: "{target.part_id}",
                        strong { "{target.name}" }
                        if let Some(process) = target.process.as_ref() {
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Remove process override for {target.name}",
                                disabled: mechanical_action_pending(&format!("process:{}:remove", target.part_id)) || !props.editable,
                                onclick: {
                                    let part_id = target.part_id.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveProcessOverride { part_id: part_id.clone() })
                                },
                                "Remove"
                            }
                            if target.part_id.ends_with("foam") {
                                output { "Cut sheet" }
                            } else {
                                SettingsSelect {
                                    label: "Method",
                                    disabled: !props.editable,
                                    value: method_id(&process.method).to_string(),
                                    on_change: {
                                        let part_id = target.part_id.clone();
                                        let identity = props.identity.clone();
                                        let mut sequence = props.request_sequence;
                                        let on_request = props.on_request;
                                        move |next: Option<String>| if let Some(method) = parse_method(&next.unwrap_or_default()) {
                                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetProcessMethod { part_id: part_id.clone(), method });
                                        }
                                    },
                                    SelectOption::<String> { index: 0usize, value: "pcb-fr4".to_owned(), text_value: "PCB FR-4".to_owned(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "PCB FR-4" }
                                        SelectOption::<String> { index: 1usize, value: "printed".to_owned(), text_value: "3D printed".to_owned(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "3D printed" }
                                        SelectOption::<String> { index: 2usize, value: "cnc".to_owned(), text_value: "CNC machined".to_owned(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "CNC machined" }
                                        SelectOption::<String> { index: 3usize, value: "cut-sheet".to_owned(), text_value: "Cut sheet".to_owned(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Cut sheet" }
                                }
                            }
                            SettingsChoiceField {
                                label: "Material",
                                aria_label: "Material",
                                disabled: !props.editable,
                                value: process.material.clone(),
                                options: process_material_options(&target.part_id, &process.method).iter().map(|material| (material.to_string(), material.to_string())).collect::<Vec<_>>(),
                                on_change: {
                                        let part_id = target.part_id.clone();
                                        let identity = props.identity.clone();
                                        let mut sequence = props.request_sequence;
                                        let on_request = props.on_request;
                                        move |next: Option<String>| if let Some(material) = next { send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetProcessMaterial { part_id: part_id.clone(), material }) }
                                    },
                            }
                            DimensionField {
                                identity: props.identity.clone(),
                                request_sequence: props.request_sequence,
                                on_request: props.on_request,
                                feedback: props.feedback.clone(),
                                field: MechanicalDimension::PlateThickness,
                                label: "Finished thickness",
                                value: process_thickness(process, target.default_thickness),
                                editable: props.editable,
                                process_target: Some(target.part_id.clone()),
                            }
                            p { class: "text-muted-foreground text-sm leading-normal", "Constraint set " code { "{process_constraint_version(process)}" } }
                            if !process.constraints_version.is_empty() && process.constraints_version != version {
                                Alert { variant: AlertVariant::Destructive, "Unsupported constraint set. Diagnostics will block export." }
                            }
                        } else {
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Add process override for {target.name}",
                                disabled: mechanical_action_pending(&format!("process:{}:add", target.part_id)) || !props.editable,
                                onclick: {
                                    let part_id = target.part_id.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddProcessOverride { part_id: part_id.clone() })
                                },
                                "Add process override"
                            }
                        }
                    }
                }
                p { class: "text-muted-foreground text-sm leading-normal", "Supported constraint set: {version}." }
            }
            section { class: "grid gap-1.5", aria_label: "Stabilizer fit",
                h4 { "Stabilizer fit {props.stabilizers.len()} wide keys" }
                for stabilizer in props.stabilizers.iter() {
                    div { class: "grid gap-1.5", key: "{stabilizer.part_id}",
                        strong { "{stabilizer.reference} · {stabilizer.key_units}u" }
                        SettingsSelect {
                            label: "Stabilizer",
                            aria_label: "Stabilizer for {stabilizer.reference}",
                            disabled: !props.editable,
                            value: stabilizer_kind_id(&stabilizer.current.kind).to_string(),
                            on_change: {
                                    let mut current = stabilizer.current.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |next: Option<String>| if let Some(kind) = parse_stabilizer_kind(&next.unwrap_or_default()) {
                                        current.kind = kind;
                                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetStabilizerKind(current.clone()));
                                    }
                                },
                            SelectOption::<String> { index: 0usize, value: "pcb-mount".to_owned(), text_value: format!("Cherry PCB mount · {}u", stabilizer.default.units), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Cherry PCB mount · {stabilizer.default.units}u" }
                                SelectOption::<String> { index: 1usize, value: "none".to_owned(), text_value: "None".to_owned(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "None" }
                                if stabilizer.current.profile.is_some() {
                                    SelectOption::<String> { index: 2usize, value: "plate-mount".to_owned(), text_value: "Custom plate mount".to_owned(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Custom plate mount" }
                                }
                        }
                        p { class: "text-muted-foreground text-sm leading-normal", "Size and orientation follow the key layout." }
                    }
                }
                if props.stabilizers.is_empty() {
                    p { class: "text-muted-foreground text-sm leading-normal", "No keys in this layout need stabilizers." }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct ProfileGuidanceProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    #[props(default)]
    pub(crate) owner_key: String,
    #[props(default)]
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) profiles: Rc<[MechanicalProfileChoice]>,
    #[props(default)]
    pub(crate) extract_profile: Option<MechanicalProfileExtractionPort>,
    pub(crate) targets: Rc<[MechanicalProfileTargetChoice]>,
    pub(crate) plate_thickness: f64,
    pub(crate) plate_to_pcb: f64,
    pub(crate) editable: bool,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
pub(crate) fn ProfileGuidance(props: ProfileGuidanceProps) -> Element {
    let mut selected_family = use_signal(String::new);
    let mut selected_stabilizer = use_signal(String::new);
    rsx! {
        section { class: "grid gap-3", aria_label: "Switch fit profiles",
            h3 { "Switch fit profiles" }
            if props.profiles.is_empty() {
                Callout { "No explicit switch fit profile is assigned. The supported plate thickness range is unavailable until a fit profile is assigned in Parts." }
                p { "Plate underside to PCB top: {props.plate_to_pcb:.2} mm · current configured gap." }
            }
            for profile in props.profiles.iter() {
                div { class: "grid gap-3 p-3 rounded-md bg-card", role: "group", aria_label: "Fit profile {profile.name}", key: "{props.owner_key}:{profile.definition_id}",
                    strong { "{profile.name}" }
                    if let Some(source) = profile.source.as_deref() { p { "Profile source: {source}." } }
                    if profile.switch_family_selectable {
                        SettingsSelect {
                            label: "Switch fit family",
                            disabled: !props.editable,
                            value: (profile.family.map(family_id).unwrap_or("")).to_string(),
                            on_change: {
                                    let definition_id = profile.definition_id.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |event: Option<String>| {
                                        if let Some(family) = parse_family(&event.clone().unwrap_or_default()) {
                                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetSwitchFamily { definition_id: definition_id.clone(), family });
                                        }
                                    }
                                },
                            SelectOption::<String> { index: 0usize, value: "".to_owned(), text_value: "Choose switch family…".to_owned(), "Choose switch family…" }
                                SelectOption::<String> { index: 1usize, value: "mx".to_owned(), text_value: "MX".to_owned(), "MX" }
                                SelectOption::<String> { index: 2usize, value: "choc-v1".to_owned(), text_value: "Choc v1".to_owned(), "Choc v1" }
                                SelectOption::<String> { index: 3usize, value: "choc-v2".to_owned(), text_value: "Choc v2".to_owned(), "Choc v2" }
                        }
                    } else if let Some(family) = profile.family {
                        p { "Switch fit: {family_label(family)}" }
                    }
                    if let Some(gap) = profile.plate_to_pcb {
                        p { "Plate underside to PCB top: {gap:.2} mm · derived from switch fit." }
                    }
                    if let Some(range) = profile.supported_thickness {
                        p { "Supported plate thickness: {range.x:.2}–{range.y:.2} mm" }
                    } else {
                        div { role: "status", Callout { variant: CalloutVariant::Warning, "Supplier review needed for the supported thickness range." } }
                    }
                    Button {
                        variant: ButtonVariant::Secondary, button_type: "button",
                        aria_label: "Remove profile {profile.name}",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: mechanical_action_pending(&format!("profile:{}:remove", profile.definition_id)) || !props.editable,
                        onclick: {
                            let identity = props.identity.clone();
                            let definition_id = profile.definition_id.clone();
                            let mut sequence = props.request_sequence;
                            let on_request = props.on_request;
                            move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveProfile { definition_id: definition_id.clone() })
                        },
                        "Remove profile"
                    }
                    ProfileGeometryEditor {
                        identity: props.identity.clone(),
                        feedback: props.feedback.clone(),
                        profile: profile.profile.clone(),
                        editable: props.editable,
                        request_sequence: props.request_sequence,
                        on_request: props.on_request,
                        extract_profile: if profile.kicad_source.is_some() { props.extract_profile.clone() } else { None },
                    }
                }
            }
            if props.profiles.iter().any(|profile| profile.family.is_none()) {
                div { role: "status", Callout { "Choose a supported switch family to resolve the plate gap." } }
            }
            if props.targets.iter().any(|target| target.switch_profile) {
                div {
                    SettingsSelect {
                        label: "Library fit profile",
                        aria_label: "Library fit profile",
                        disabled: !props.editable,
                        value: (selected_family()).to_string(),
                        on_change: move |event: Option<String>| selected_family.set(event.clone().unwrap_or_default()),
                        SelectOption::<String> { index: 0usize, value: "".to_owned(), text_value: "Choose a switch family…".to_owned(), "Choose a switch family…" }
                            SelectOption::<String> { index: 1usize, value: "mx".to_owned(), text_value: "MX switch".to_owned(), "MX switch" }
                            SelectOption::<String> { index: 2usize, value: "choc-v1".to_owned(), text_value: "Choc v1 switch".to_owned(), "Choc v1 switch" }
                            SelectOption::<String> { index: 3usize, value: "choc-v2".to_owned(), text_value: "Choc v2 switch".to_owned(), "Choc v2 switch" }
                    }
                    ProfileTargetField {
                        label: "Assign library fit profile to",
                        aria_label: "Assign library fit profile to",
                        disabled: mechanical_action_pending("assign-switch-profile:") || !props.editable || selected_family().is_empty(),

                        on_change: {
                                let identity = props.identity.clone();
                                let mut sequence = props.request_sequence;
                                let on_request = props.on_request;
                                let selected_family = selected_family;
                                move |event: Option<String>| {
                                    let definition_id = event.clone().unwrap_or_default();
                                    if !definition_id.is_empty()
                                        && let Some(family) = parse_family(&selected_family())
                                    {
                                        send_request(
                                            &mut sequence,
                                            &identity,
                                            on_request,
                                            MechanicalSettingsPatch::AssignSwitchProfile {
                                                definition_id,
                                                family,
                                            },
                                        );
                                    }
                                }
                            },
                        placeholder: "Choose a placed switch type…", options: props.targets.iter().filter(|target| target.switch_profile).cloned().collect(),
                    }
                }
                div {
                    SettingsSelect {
                        label: "Library stabilizer profile",
                        aria_label: "Library stabilizer profile",
                        disabled: !props.editable,
                        value: (selected_stabilizer()).to_string(),
                        on_change: move |event: Option<String>| selected_stabilizer.set(event.clone().unwrap_or_default()),
                        SelectOption::<String> { index: 0usize, value: "".to_owned(), text_value: "Choose a stabilizer size…".to_owned(), "Choose a stabilizer size…" }
                            SelectOption::<String> { index: 1usize, value: "mx-stab2u".to_owned(), text_value: "MX stabilizer · 2u".to_owned(), "MX stabilizer · 2u" }
                            SelectOption::<String> { index: 2usize, value: "mx-stab625u".to_owned(), text_value: "MX stabilizer · 6.25u".to_owned(), "MX stabilizer · 6.25u" }
                    }
                    ProfileTargetField {
                        label: "Assign library stabilizer profile to",
                        aria_label: "Assign library stabilizer profile to",
                        disabled: mechanical_action_pending("assign-stabilizer-profile:") || !props.editable || selected_stabilizer().is_empty(),

                        on_change: {
                                let identity = props.identity.clone();
                                let mut sequence = props.request_sequence;
                                let on_request = props.on_request;
                                let selected_stabilizer = selected_stabilizer;
                                move |event: Option<String>| {
                                    let source = match selected_stabilizer().as_str() {
                                        "mx-stab2u" => Some(MechanicalBuiltinProfile::MxStab2u),
                                        "mx-stab625u" => Some(MechanicalBuiltinProfile::MxStab625u),
                                        _ => None,
                                    };
                                    if !event.clone().unwrap_or_default().is_empty()
                                        && let Some(source) = source
                                    {
                                        send_request(
                                            &mut sequence,
                                            &identity,
                                            on_request,
                                            MechanicalSettingsPatch::AssignStabilizerProfile {
                                                definition_id: event.clone().unwrap_or_default(),
                                                source,
                                            },
                                        );
                                    }
                                }
                            },
                        placeholder: "Choose a placed switch type…", options: props.targets.to_vec(),
                    }
                }
            }
            if props.targets.iter().any(|target| target.imported_geometry) {
                ProfileTargetField {
                    label: "Custom KiCad geometry",
                    aria_label: "Assign custom geometry profile to",
                    disabled: mechanical_action_pending("assign-imported-profile:") || !props.editable,

                    on_change: {
                            let identity = props.identity.clone();
                            let mut sequence = props.request_sequence;
                            let on_request = props.on_request;
                            move |event: Option<String>| {
                                if !event.clone().unwrap_or_default().is_empty() {
                                    send_request(
                                        &mut sequence,
                                        &identity,
                                        on_request,
                                        MechanicalSettingsPatch::AssignImportedGeometryProfile {
                                            definition_id: event.clone().unwrap_or_default(),
                                        },
                                    );
                                }
                            }
                        },
                    placeholder: "Choose imported part type…", options: props.targets.iter().filter(|target| target.imported_geometry).cloned().collect(),
                    }
            }
            p { "Current plate thickness: {props.plate_thickness:.2} mm." }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct ProfileGeometryEditorProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) profile: MechanicalPartProfile,
    pub(crate) editable: bool,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) extract_profile: Option<MechanicalProfileExtractionPort>,
}

#[component]
pub(crate) fn ProfileGeometryEditor(props: ProfileGeometryEditorProps) -> Element {
    let geometry_state = use_signal(|| None::<MechanicalGeometry>);
    let selected = use_signal(Vec::<String>::new);
    let purposes = use_signal(Vec::<MechanicalPurposeMapping>::new);
    let busy = use_signal(|| false);
    let error = use_signal(String::new);
    let extraction_port = props.extract_profile.clone();
    rsx! {
        section { class: "grid gap-3", aria_label: "Profile geometry",
            if let Some(port) = extraction_port.clone() {
                section { aria_label: "Custom KiCad geometry",
                    div { class: "flex flex-wrap items-center justify-between gap-2",
                        strong { "Custom geometry layers" }
                        Button {
                            class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                            variant: ButtonVariant::Secondary, button_type: "button",
                            aria_label: "Read KiCad layers",
                            disabled: !props.editable || busy(),
                            onclick: {
                                let definition_id = props.profile.definition_id.clone();
                                let port = port.clone();
                                let mut geometry_state = geometry_state;
                                let mut selected = selected;
                                let mut purposes = purposes;
                                let profile_source_geometry = props.profile.source_geometry.clone();
                                let mut busy = busy;
                                let mut error = error;
                                move |_| {
                                    busy.set(true);
                                    error.set(String::new());
                                    let definition_id = definition_id.clone();
                                    let port = port.clone();
                                    let existing_source_geometry = profile_source_geometry.clone();
                                    dioxus::prelude::spawn(async move {
                                        match (port.0)(definition_id, Vec::new()).await {
                                            Ok(result) => {
                                                selected.set(existing_source_geometry.as_ref().map_or_else(Vec::new, |source| source.source_ids.clone()));
                                                purposes.set(existing_source_geometry.as_ref().map_or_else(|| result.source_geometry.mappings.clone(), |source| source.mappings.clone()));
                                                geometry_state.set(Some(result.geometry));
                                            }
                                            Err(message) => error.set(message),
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                            if geometry_state().is_some() { "Reload layers" } else { "Read KiCad layers" }
                        }
                    }
                    if let Some(loaded_geometry) = geometry_state() {
                        p { class: "text-muted-foreground text-sm leading-normal", "Select exact primitives for plate cutouts, PCB holes, clearances or drawing guides, then apply the selected geometry." }
                        for primitive in loaded_geometry.primitives.iter() {
                            div { role: "group", aria_label: "Geometry {primitive.id}", key: "{primitive.id}",
                                FieldLabel { class: "min-h-control-touch", html_for: format!("case-geometry-{}-{}", dioxus::core::current_scope_id().0, primitive.id),
                                    Checkbox {
                                        id: format!("case-geometry-{}-{}", dioxus::core::current_scope_id().0, primitive.id),
                                        checked: Some(if selected().contains(&primitive.id) { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                                        aria_label: "Select geometry {primitive.id}",
                                        disabled: !props.editable || busy(),
                                        on_checked_change: {
                                            let id = primitive.id.clone();
                                            let mut selected = selected;
                                            move |event: CheckboxState| {
                                                let mut ids = selected().clone();
                                                if bool::from(event) {
                                                    if !ids.contains(&id) { ids.push(id.clone()); }
                                                } else { ids.retain(|entry| entry != &id); }
                                                selected.set(ids);
                                            }
                                        }
                                    }
                                    span { strong { "{primitive.layer.as_deref().unwrap_or(\"geometry\")}" } small { "{primitive.kind:?} · {primitive.source_group_id}" } }
                                }
                                SettingsSelect {
                                    label: "Purpose",
                                    aria_label: "Purpose for {primitive.layer.as_deref().unwrap_or(\"geometry\")}",
                                    disabled: !props.editable || busy(),
                                    value: (purposes().iter().find(|mapping| mapping.source_id.as_deref() == Some(primitive.id.as_str())).map_or("plate-cutout", |mapping| purpose_id(mapping.purpose))).to_string(),
                                    on_change: {
                                            let id = primitive.id.clone();
                                            let mut purposes = purposes;
                                            move |event: Option<String>| {
                                                let Some(purpose) = parse_purpose(&event.clone().unwrap_or_default()) else { return; };
                                                let mut entries = purposes().clone();
                                                entries.retain(|mapping| mapping.source_id.as_deref() != Some(id.as_str()));
                                                entries.push(MechanicalPurposeMapping { source_id: Some(id.clone()), kind: None, layer: None, purpose });
                                                purposes.set(entries);
                                            }
                                        },
                                    SelectOption::<String> { index: 0usize, value: "plate-cutout".to_owned(), text_value: "Plate cutout".to_owned(), "Plate cutout" }
                                        SelectOption::<String> { index: 1usize, value: "electrical-pcb-mounting-hole".to_owned(), text_value: "PCB mounting hole".to_owned(), "PCB mounting hole" }
                                        SelectOption::<String> { index: 2usize, value: "clearance-envelope".to_owned(), text_value: "Clearance envelope".to_owned(), "Clearance envelope" }
                                        SelectOption::<String> { index: 3usize, value: "drawing-guide".to_owned(), text_value: "Drawing guide".to_owned(), "Drawing guide" }
                                }
                            }
                        }
                        Button {
                            class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                            variant: ButtonVariant::Secondary, button_type: "button",
                            aria_label: "Apply selected geometry",
                            disabled: mechanical_action_pending(&format!("profile:{}:update", props.profile.definition_id)) || !props.editable || busy() || selected().is_empty(),
                            onclick: {
                                let definition_id = props.profile.definition_id.clone();
                                let port = port.clone();
                                let profile = props.profile.clone();
                                let identity = props.identity.clone();
                                let sequence = props.request_sequence;
                                let on_request = props.on_request;
                                let mut busy = busy;
                                let mut error = error;
                                move |_| {
                                    let ids = selected().clone();
                                    let mappings = geometry_state().as_ref().into_iter().flat_map(|geometry| geometry.primitives.iter()).filter(|primitive| ids.contains(&primitive.id)).map(|primitive| {
                                        let purpose = purposes().iter().find(|mapping| mapping.source_id.as_deref() == Some(primitive.id.as_str())).map_or(MechanicalPurpose::PlateCutout, |mapping| mapping.purpose);
                                        MechanicalPurposeMapping { source_id: Some(primitive.id.clone()), kind: None, layer: None, purpose }
                                    }).collect::<Vec<_>>();
                                    busy.set(true);
                                    error.set(String::new());
                                    let definition_id = definition_id.clone();
                                    let port = port.clone();
                                    let mut sequence = sequence;
                                    let profile = profile.clone();
                                    let identity = identity.clone();
                                    dioxus::prelude::spawn(async move {
                                        match (port.0)(definition_id, mappings).await {
                                            Ok(result) => {
                                                let mut next = profile;
                                                next.cutouts = result.plate_cutouts;
                                                next.clearances = Some(result.clearance_envelopes);
                                                next.pcb_holes = Some(result.pcb_holes);
                                                next.source_geometry = Some(result.source_geometry);
                                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next));
                                            }
                                            Err(message) => error.set(message),
                                        }
                                        busy.set(false);
                                    });
                                }
                            },
                            "Apply selected geometry"
                        }
                    }
                    if !error().is_empty() { Alert { variant: AlertVariant::Destructive, "{error()}" } }
                }
            }
            ProfilePolygonListEditor {
                title: "Plate cutout geometry", polygons: props.profile.cutouts.clone(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |cutouts| { let mut next = profile.clone(); next.cutouts = cutouts;
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
            ProfilePolygonListEditor {
                title: "Component clearance zones", polygons: props.profile.clearances.clone().unwrap_or_default(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |clearances| { let mut next = profile.clone(); next.clearances = Some(clearances);
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
            ProfileOpeningListEditor {
                identity: props.identity.clone(), profile: props.profile.clone(),
                request_sequence: props.request_sequence, on_request: props.on_request,
                feedback: props.feedback.clone(), clearance_volume: false,
                title: "Access openings", openings: props.profile.openings.clone().unwrap_or_default(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |openings| { let mut next = profile.clone(); next.openings = Some(openings);
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
            ProfileOpeningListEditor {
                identity: props.identity.clone(), profile: props.profile.clone(),
                request_sequence: props.request_sequence, on_request: props.on_request,
                feedback: props.feedback.clone(), clearance_volume: true,
                title: "Clearance volumes", openings: props.profile.clearance_volumes.clone().unwrap_or_default(), editable: props.editable,
                on_change: {
                    let profile = props.profile.clone(); let identity = props.identity.clone();
                    let mut sequence = props.request_sequence; let on_request = props.on_request;
                    move |clearance_volumes| { let mut next = profile.clone(); next.clearance_volumes = Some(clearance_volumes);
                        send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::UpdateProfile(next)); }
                },
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct ProfilePolygonListEditorProps {
    pub(crate) title: &'static str,
    pub(crate) polygons: Vec<Vec<Vec2>>,
    pub(crate) editable: bool,
    pub(crate) on_change: EventHandler<Vec<Vec<Vec2>>>,
}

#[component]
pub(crate) fn ProfilePolygonListEditor(props: ProfilePolygonListEditorProps) -> Element {
    rsx! {
        section {
            class: "grid gap-3",
            aria_label: "{props.title}",
            header { class: "flex flex-wrap items-center justify-between gap-2",
                strong { "{props.title}" }
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Add polygon to {props.title}",
                    disabled: !props.editable,
                    onclick: {
                        let mut polygons = props.polygons.clone();
                        let on_change = props.on_change;
                        move |_| {
                            polygons
                                .push(
                                    vec![
                                        Vec2 { x: -2.5, y: -2.5 },
                                        Vec2 { x: 2.5, y: -2.5 },
                                        Vec2 { x: 2.5, y: 2.5 },
                                        Vec2 { x: -2.5, y: 2.5 },
                                    ],
                                );
                            on_change.call(polygons.clone());
                        }
                    },
                    "Add polygon"
                }
            }
            if props.polygons.is_empty() {
                Callout { "No contours assigned." }
            }
            for (polygon_index, polygon) in props.polygons.iter().enumerate() {
                fieldset {
                    class: "grid gap-3 m-0 min-w-0 border-0 p-0",
                    aria_label: "{props.title} contour {polygon_index + 1}",
                    key: "{props.title}:{polygon_index}",
                    disabled: !props.editable,
                    div { class: "flex flex-wrap items-center justify-between gap-2",
                        strong { "Contour {polygon_index + 1}" }
                        Button {
                            variant: ButtonVariant::Secondary,
                            button_type: "button",
                            class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                            aria_label: "Remove contour {polygon_index + 1} from {props.title}",
                            disabled: !props.editable,
                            onclick: {
                                let mut polygons = props.polygons.clone();
                                let on_change = props.on_change;
                                move |_| {
                                    polygons.remove(polygon_index);
                                    on_change.call(polygons.clone());
                                }
                            },
                            "Remove"
                        }
                    }
                    for (point_index, point) in polygon.iter().enumerate() {
                        div {
                            class: "flex flex-wrap items-end gap-2",
                            key: "{polygon_index}:{point_index}",
                            span { "V{point_index + 1}" }
                            for (axis, axis_id, coordinate) in [("X", "x", point.x), ("Y", "y", point.y)] {
                                Field {
                                    class: "w-28 gap-1.5",
                                    disabled: !props.editable,
                                    FieldLabel {
                                        html_for: format!("case-profile-{}-{polygon_index}-{point_index}-{axis_id}", dioxus::core::current_scope_id().0),
                                        "{axis}"
                                    }
                                    InputGroup { class: "max-[760px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                                        InputGroupInput {
                                            id: format!("case-profile-{}-{polygon_index}-{point_index}-{axis_id}", dioxus::core::current_scope_id().0),
                                            class: "font-mono",
                                            r#type: InputType::Number,
                                            step: "0.1",
                                            value: "{coordinate}",
                                            disabled: !props.editable,
                                            aria_label: "{props.title} contour {polygon_index + 1} vertex {point_index + 1} {axis}",
                                            oninput: {
                                                let mut polygons = props.polygons.clone();
                                                let on_change = props.on_change;
                                                move |event: FormEvent| {
                                                    if let Ok(value) = event.value().parse::<f64>() {
                                                        if axis_id == "x" { polygons[polygon_index][point_index].x = value; } else { polygons[polygon_index][point_index].y = value; }
                                                        on_change.call(polygons.clone());
                                                    }
                                                }
                                            },
                                        }
                                        InputGroupText { class: "pr-3", "mm" }
                                    }
                                }
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Sm,
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Remove {props.title} contour {polygon_index + 1} vertex {point_index + 1}",
                                disabled: !props.editable || polygon.len() <= 3,
                                onclick: {
                                    let mut polygons = props.polygons.clone();
                                    let on_change = props.on_change;
                                    move |_| {
                                        polygons[polygon_index].remove(point_index);
                                        on_change.call(polygons.clone());
                                    }
                                },
                                "Remove vertex"
                            }
                        }
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: !props.editable,
                        aria_label: "Add vertex to {props.title} contour {polygon_index + 1}",
                        onclick: {
                            let mut polygons = props.polygons.clone();
                            let on_change = props.on_change;
                            move |_| {
                                polygons[polygon_index].push(Vec2 { x: 0.0, y: 0.0 });
                                on_change.call(polygons.clone());
                            }
                        },
                        "Add vertex"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct ProfileOpeningListEditorProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) profile: MechanicalPartProfile,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) clearance_volume: bool,
    pub(crate) title: &'static str,
    pub(crate) openings: Vec<CaseOpening>,
    pub(crate) editable: bool,
    pub(crate) on_change: EventHandler<Vec<CaseOpening>>,
}

#[component]
pub(crate) fn ProfileOpeningListEditor(props: ProfileOpeningListEditorProps) -> Element {
    rsx! {
        section {
            class: "grid gap-3",
            aria_label: "{props.title}",
            header { class: "flex flex-wrap items-center justify-between gap-2",
                strong { "{props.title}" }
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Add volume to {props.title}",
                    disabled: !props.editable,
                    onclick: {
                        let mut openings = props.openings.clone();
                        let on_change = props.on_change;
                        move |_| {
                            openings
                                .push(CaseOpening {
                                    points: vec![
                                        Vec2 { x: -2.5, y: -2.5 },
                                        Vec2 { x: 2.5, y: -2.5 },
                                        Vec2 { x: 2.5, y: 2.5 },
                                        Vec2 { x: -2.5, y: 2.5 },
                                    ],
                                    z: 0.0,
                                    height: 10.0,
                                });
                            on_change.call(openings.clone());
                        }
                    },
                    "Add volume"
                }
            }
            if props.openings.is_empty() {
                Callout { "No volumes configured." }
            }
            for (opening_index, opening) in props.openings.iter().enumerate() {
                fieldset {
                    class: "grid gap-3 m-0 min-w-0 border-0 p-0",
                    aria_label: "{props.title} volume {opening_index + 1}",
                    key: "{props.title}:{opening_index}",
                    disabled: !props.editable,
                    div { class: "flex flex-wrap items-center justify-between gap-2",
                        strong { "Volume {opening_index + 1}" }
                        Button {
                            variant: ButtonVariant::Secondary,
                            button_type: "button",
                            class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                            aria_label: "Remove {props.title} volume {opening_index + 1}",
                            disabled: !props.editable,
                            onclick: {
                                let mut openings = props.openings.clone();
                                let on_change = props.on_change;
                                move |_| {
                                    openings.remove(opening_index);
                                    on_change.call(openings.clone());
                                }
                            },
                            "Remove"
                        }
                    }
                    DimensionField {
                        identity: props.identity.clone(),
                        request_sequence: props.request_sequence,
                        on_request: props.on_request,
                        feedback: props.feedback.clone(),
                        field: MechanicalDimension::OpeningBottomZ,
                        label: "Bottom Z",
                        value: opening.z,
                        editable: props.editable,
                        accessible_label: format!("{} volume {} Bottom Z", props.title, opening_index + 1),
                        profile_volume_target: ProfileVolumeDimensionTarget {
                            profile: props.profile.clone(),
                            opening_index,
                            clearance_volume: props.clearance_volume,
                        },
                    }
                    DimensionField {
                        identity: props.identity.clone(),
                        request_sequence: props.request_sequence,
                        on_request: props.on_request,
                        feedback: props.feedback.clone(),
                        field: MechanicalDimension::OpeningHeight,
                        label: "Height",
                        value: opening.height,
                        editable: props.editable,
                        accessible_label: format!("{} volume {} Height", props.title, opening_index + 1),
                        profile_volume_target: ProfileVolumeDimensionTarget {
                            profile: props.profile.clone(),
                            opening_index,
                            clearance_volume: props.clearance_volume,
                        },
                    }
                    h4 { "XY footprint" }
                    for (point_index, point) in opening.points.iter().enumerate() {
                        div {
                            class: "flex flex-wrap items-end gap-2",
                            key: "{opening_index}:{point_index}",
                            span { "V{point_index + 1}" }
                            for (axis, axis_id, coordinate) in [("X", "x", point.x), ("Y", "y", point.y)] {
                                Field {
                                    class: "w-28 gap-1.5",
                                    disabled: !props.editable,
                                    FieldLabel {
                                        html_for: format!("case-volume-{}-{opening_index}-{point_index}-{axis_id}", dioxus::core::current_scope_id().0),
                                        "{axis}"
                                    }
                                    InputGroup { class: "max-[760px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                                        InputGroupInput {
                                            id: format!("case-volume-{}-{opening_index}-{point_index}-{axis_id}", dioxus::core::current_scope_id().0),
                                            class: "font-mono",
                                            r#type: InputType::Number,
                                            step: "0.1",
                                            value: "{coordinate}",
                                            disabled: !props.editable,
                                            aria_label: "{props.title} volume {opening_index + 1} vertex {point_index + 1} {axis}",
                                            oninput: {
                                                let mut openings = props.openings.clone();
                                                let on_change = props.on_change;
                                                move |event: FormEvent| {
                                                    if let Ok(value) = event.value().parse::<f64>() {
                                                        if axis_id == "x" { openings[opening_index].points[point_index].x = value; } else { openings[opening_index].points[point_index].y = value; }
                                                        on_change.call(openings.clone());
                                                    }
                                                }
                                            },
                                        }
                                        InputGroupText { class: "pr-3", "mm" }
                                    }
                                }
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Sm,
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Remove {props.title} volume {opening_index + 1} vertex {point_index + 1}",
                                disabled: !props.editable || opening.points.len() <= 3,
                                onclick: {
                                    let mut openings = props.openings.clone();
                                    let on_change = props.on_change;
                                    move |_| {
                                        openings[opening_index].points.remove(point_index);
                                        on_change.call(openings.clone());
                                    }
                                },
                                "Remove vertex"
                            }
                        }
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: !props.editable,
                        aria_label: "Add vertex to {props.title} volume {opening_index + 1}",
                        onclick: {
                            let mut openings = props.openings.clone();
                            let on_change = props.on_change;
                            move |_| {
                                openings[opening_index].points.push(Vec2 { x: 0.0, y: 0.0 });
                                on_change.call(openings.clone());
                            }
                        },
                        "Add vertex"
                    }
                }
            }
        }
    }
}

pub(crate) fn parse_purpose(value: &str) -> Option<MechanicalPurpose> {
    match value {
        "plate-cutout" => Some(MechanicalPurpose::PlateCutout),
        "electrical-pcb-mounting-hole" => Some(MechanicalPurpose::ElectricalPcbMountingHole),
        "clearance-envelope" => Some(MechanicalPurpose::ClearanceEnvelope),
        "drawing-guide" => Some(MechanicalPurpose::DrawingGuide),
        _ => None,
    }
}

pub(crate) fn purpose_id(purpose: MechanicalPurpose) -> &'static str {
    match purpose {
        MechanicalPurpose::PlateCutout => "plate-cutout",
        MechanicalPurpose::ElectricalPcbMountingHole => "electrical-pcb-mounting-hole",
        MechanicalPurpose::ClearanceEnvelope => "clearance-envelope",
        MechanicalPurpose::DrawingGuide => "drawing-guide",
    }
}
