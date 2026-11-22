#![allow(unused_imports)]
//! Private read-only presentation and field intents for the Case mechanical stack.
//! Root owns accepted-state admission and commits every intent through Runtime.
pub use crate::mechanical_feedback::{
    MechanicalSettingsFeedback, MechanicalSettingsFeedbackState, MechanicalSettingsIdentity,
};
use crate::mechanical_pair::{MechanicalPairFeatureRow, MechanicalPairLinkOption, feature_label};
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
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::{Callout, CalloutVariant},
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput, InputGroupText, InputGroupTextarea},
    item::{Item, ItemActions, ItemContent, ItemDescription, ItemGroup},
    toggle_group::{ToggleGroup, ToggleItem},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use std::{future::Future, pin::Pin};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use super::*;
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MechanicalTextIntent {
    ClosureThread,
    ClosureScrewLengths,
    CriticalFit {
        fit_id: String,
        field: MechanicalCriticalFitTextField,
    },
    Hardware {
        hardware_id: String,
        field: MechanicalHardwareTextField,
    },
}

impl MechanicalTextIntent {
    fn patch(&self, value: String) -> Result<MechanicalSettingsPatch, String> {
        Ok(match self {
            Self::ClosureThread => MechanicalSettingsPatch::SetClosureThread(value),
            Self::ClosureScrewLengths => {
                let mut lengths = Vec::new();
                for item in value.split(',') {
                    let length = item
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| "Enter positive lengths separated by commas.".to_owned())?;
                    if !length.is_finite() || length <= 0.0 {
                        return Err("Enter positive lengths separated by commas.".to_owned());
                    }
                    if !lengths.contains(&length) {
                        lengths.push(length);
                    }
                }
                if lengths.is_empty() {
                    return Err("Enter positive lengths separated by commas.".to_owned());
                }
                MechanicalSettingsPatch::SetClosureScrewLengths(lengths)
            }
            Self::CriticalFit { fit_id, field } => MechanicalSettingsPatch::SetCriticalFitText {
                fit_id: fit_id.clone(),
                field: *field,
                value,
            },
            Self::Hardware { hardware_id, field } => MechanicalSettingsPatch::SetHardwareText {
                hardware_id: hardware_id.clone(),
                field: *field,
                value,
            },
        })
    }
}

/// Presentation-only numeric field for caller-owned drafts; parsing stays with the caller.
#[component]
pub(crate) fn SettingsNumberField(
    label: String,
    value: ReadSignal<String>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(into, optional)] step: Option<String>,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let id = format!("case-number-{}", dioxus::core::current_scope_id().0);
    let aria_label = Some(aria_label.unwrap_or_else(|| label.clone()));
    rsx! {
        NumericField { id, label, value: value(), aria_label, step: step.unwrap_or_else(|| "1".to_owned()), oninput }
    }
}

/// Label a mechanical input and keep its ARIA message target mounted.
#[component]
fn MechanicalBoundInputRow(
    label: String,
    html_for: String,
    children: Element,
    #[props(default)] disabled: bool,
    #[props(default)] message_id: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] status: Option<&'static str>,
) -> Element {
    rsx! {
        Field { invalid: error.is_some(), disabled, class: "gap-1.5",
            FieldLabel { html_for, "{label}" }
            {children}
            if let Some(message_id) = message_id {
                div { id: message_id,
                    if let Some(error) = error { FieldError { "{error}" } }
                    if let Some(status) = status { small { role: "status", "{status}" } }
                }
            }
        }
    }
}

/// Canonical mechanical dimension field: all stack/closure/fit/mount numeric
/// rows go through this rust-ui molecule (a thin alias over `DimensionField`).
#[component]
pub(crate) fn MechanicalDimensionField(props: DimensionFieldProps) -> Element {
    DimensionField(props)
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct DimensionControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) values: MechanicalSettingsValues,
    pub(crate) editable: bool,
    pub(crate) disabled_reason: Option<String>,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) owner_key: String,
}

#[component]
pub(crate) fn DimensionControls(props: DimensionControlsProps) -> Element {
    let fields = [
        (
            MechanicalDimension::PlateThickness,
            "Plate thickness",
            props.values.plate_thickness,
        ),
        (
            MechanicalDimension::PlateFoamThickness,
            "Plate foam",
            props.values.plate_foam_thickness,
        ),
        (
            MechanicalDimension::PcbThickness,
            "PCB thickness",
            props.values.pcb_thickness,
        ),
        (
            MechanicalDimension::BottomFoamThickness,
            "Bottom foam",
            props.values.bottom_foam_thickness,
        ),
        (
            MechanicalDimension::BottomThickness,
            "Bottom thickness",
            props.values.bottom_thickness,
        ),
        (
            MechanicalDimension::WallThickness,
            "Wall thickness",
            props.values.wall_thickness,
        ),
        (
            MechanicalDimension::Clearance,
            "Clearance",
            props.values.clearance,
        ),
        (
            MechanicalDimension::OpeningAllowance,
            "Radial opening allowance",
            props.values.opening_allowance,
        ),
    ];
    let request_sequence = props.request_sequence;
    rsx! {
        fieldset { class: "grid gap-3 m-0 min-w-0 p-3 rounded-md bg-card border-0", disabled: !props.editable,
            legend { "Dimensions and clearances · mm" }
            for (field, label, value) in fields {
                MechanicalDimensionField {
                    key: "{props.owner_key}:{field.field_id()}",
                    identity: props.identity.clone(),
                    request_sequence,
                    on_request: props.on_request,
                    feedback: props.feedback.clone(),
                    field,
                    label,
                    value,
                    editable: props.editable,
                }
            }
            if let Some(reason) = props.disabled_reason.as_deref() {
                p { role: "status", "{reason}" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct DimensionFieldProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) field: MechanicalDimension,
    pub(crate) label: &'static str,
    pub(crate) value: f64,
    pub(crate) editable: bool,
    #[props(default)]
    pub(crate) support_target: Option<GasketSupportDimensionTarget>,
    #[props(default)]
    pub(crate) mount_target: Option<MountDimensionTarget>,
    #[props(default)]
    pub(crate) opening_target: Option<OpeningDimensionTarget>,
    #[props(default)]
    pub(crate) profile_volume_target: Option<ProfileVolumeDimensionTarget>,
    #[props(default)]
    pub(crate) accessible_label: Option<String>,
    #[props(default)]
    pub(crate) critical_fit_target: Option<CriticalFitDimensionTarget>,
    #[props(default)]
    pub(crate) hardware_target: Option<HardwareDimensionTarget>,
    #[props(default)]
    pub(crate) process_target: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GasketSupportDimensionTarget {
    pub(crate) support_id: String,
    pub(crate) anchors: Vec<MechanicalGasketAnchor>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MountDimensionTarget {
    pub(crate) collection: MechanicalMountCollection,
    pub(crate) mount_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OpeningDimensionTarget {
    pub(crate) opening_id: String,
    pub(crate) vertex_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProfileVolumeDimensionTarget {
    pub(crate) profile: MechanicalPartProfile,
    pub(crate) opening_index: usize,
    pub(crate) clearance_volume: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CriticalFitDimensionTarget {
    pub(crate) fit_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HardwareDimensionTarget {
    pub(crate) hardware_id: String,
}

// Profile-volume controls emit the same UpdateProfile domain request, but each
// mounted dimension owns a distinct bounded helper observation and accepted text.
#[component]
pub(crate) fn DimensionField(props: DimensionFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.to_string());
    let mut dirty = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let accepted = props.value.to_string();
    use_effect(use_reactive((&accepted,), move |(accepted,)| {
        if !*dirty.peek() && *draft.peek() != accepted {
            draft.set(accepted);
            error.set(None);
        }
    }));

    let commit: Rc<dyn Fn()> = Rc::new({
        let identity = props.identity.clone();
        let field = props.field;
        let support_target = props.support_target.clone();
        let mount_target = props.mount_target.clone();
        let opening_target = props.opening_target.clone();
        let profile_volume_target = props.profile_volume_target.clone();
        let critical_fit_target = props.critical_fit_target.clone();
        let hardware_target = props.hardware_target.clone();
        let process_target = props.process_target.clone();
        let sequence = props.request_sequence;
        let on_request = props.on_request;
        move || {
            let mut sequence = sequence;
            let mut error = error;
            let mut dirty = dirty;
            let text = draft();
            let process_rule = process_target.as_deref().map(|target| {
                if target.ends_with("foam") {
                    NumberRule::Nonnegative
                } else {
                    NumberRule::AtLeastTenth
                }
            });
            let rule = process_rule.unwrap_or_else(|| field.rule());
            if !dirty() {
                return;
            }
            let Ok(value) = text.trim().parse::<f64>() else {
                error.set(Some(rule.error().to_owned()));
                return;
            };
            if !rule.valid(value) {
                error.set(Some(rule.error().to_owned()));
                return;
            }
            error.set(None);
            let patch = if let Some(target) = support_target.clone() {
                MechanicalSettingsPatch::SetGasketSupportDimension {
                    support_id: target.support_id,
                    anchors: target.anchors,
                    field,
                    value,
                }
            } else if let Some(target) = mount_target.clone() {
                MechanicalSettingsPatch::SetMountDimension {
                    collection: target.collection,
                    mount_id: target.mount_id,
                    field,
                    value,
                }
            } else if let Some(mut target) = profile_volume_target.clone() {
                let openings = if target.clearance_volume {
                    target.profile.clearance_volumes.as_mut()
                } else {
                    target.profile.openings.as_mut()
                };
                let Some(opening) =
                    openings.and_then(|openings| openings.get_mut(target.opening_index))
                else {
                    error.set(Some(
                        "This profile volume is no longer available.".to_owned(),
                    ));
                    return;
                };
                match field {
                    MechanicalDimension::OpeningBottomZ => opening.z = value,
                    MechanicalDimension::OpeningHeight => opening.height = value,
                    _ => return,
                }
                MechanicalSettingsPatch::UpdateProfile(target.profile)
            } else if let Some(target) = opening_target.clone() {
                MechanicalSettingsPatch::SetOpeningDimension {
                    opening_id: target.opening_id,
                    vertex_id: target.vertex_id,
                    field,
                    value,
                }
            } else if let Some(target) = critical_fit_target.clone() {
                MechanicalSettingsPatch::SetCriticalFitPoint {
                    fit_id: target.fit_id,
                    field,
                    value,
                }
            } else if let Some(target) = hardware_target.clone() {
                MechanicalSettingsPatch::SetHardwareDimension {
                    hardware_id: target.hardware_id,
                    field,
                    value,
                }
            } else if let Some(part_id) = process_target.clone() {
                MechanicalSettingsPatch::SetProcessThickness {
                    part_id,
                    thickness: value,
                }
            } else {
                MechanicalSettingsPatch::SetDimension { field, value }
            };
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Mechanical request identity is exhausted; reopen the Case inspector."
                        .to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MechanicalSettingsRequest {
                identity: identity.clone(),
                request_id,
                field_id: patch.field_id(),
                patch,
                reviewed_fingerprint: None,
            };
            dirty.set(false);
            on_request.call(request);
        }
    });
    let error_text = error();
    let id = format!("case-dimension-{}", dioxus::core::current_scope_id().0);
    let message_id = format!("{id}-message");
    let invalid = error_text.is_some();
    let described_by = invalid.then(|| message_id.clone());
    rsx! {
        MechanicalBoundInputRow {
            label: props.label, html_for: id.clone(), disabled: !props.editable,
            message_id: Some(message_id), error: error_text,
            InputGroup { class: "max-[760px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                InputGroupInput {
                    id: id.clone(), class: "font-mono",
                    r#type: InputType::Number,
                    step: props.field.step(),
                    min: props.process_target.as_deref().map(|target| {
                        if target.ends_with("foam") { "0" } else { "0.1" }
                    }).unwrap_or_else(|| props.field.rule().minimum()),
                    max: if props.field == MechanicalDimension::OpeningAllowance { "1" },
                    aria_label: props.accessible_label.as_deref().unwrap_or(props.label),
                    value: "{draft}",
                    disabled: !props.editable,
                    aria_invalid: invalid,
                    aria_describedby: described_by,
                    oninput: move |event: FormEvent| {
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                    },
                    attributes: vec![dioxus::events::onblur({
                        let commit = commit.clone();
                        move |_| commit()
                    }), dioxus::events::onkeydown({
                        let accepted = props.value;
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            if key == "Enter" {
                                event.prevent_default();
                                if let Some(input) = event
                                    .data()
                                    .try_as_web_event()
                                    .and_then(|event| event.target())
                                    .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                {
                                    let _ = input.blur();
                                }
                            } else if key == "Escape" {
                                event.prevent_default();
                                draft.set(accepted.to_string());
                                dirty.set(false);
                                error.set(None);
                            }
                        }
                    })],
                }
                InputGroupText { class: "pr-3", if props.field == MechanicalDimension::HardwareQuantity { "pcs" } else { "mm" } }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct TextDraftFieldProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) label: &'static str,
    pub(crate) value: String,
    pub(crate) intent: MechanicalTextIntent,
    pub(crate) editable: bool,
    #[props(default)]
    pub(crate) multiline: bool,
}

#[component]
pub(crate) fn TextDraftField(props: TextDraftFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.clone());
    let mut dirty = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let accepted = props.value.clone();
    use_effect(use_reactive((&accepted,), move |(accepted,)| {
        if !*dirty.peek() && *draft.peek() != accepted {
            draft.set(accepted);
            error.set(None);
        }
    }));

    let commit: Rc<dyn Fn()> = Rc::new({
        let identity = props.identity.clone();
        let intent = props.intent.clone();
        let sequence = props.request_sequence;
        let on_request = props.on_request;
        move || {
            let mut sequence = sequence;
            let mut error = error;
            let mut dirty = dirty;
            let draft = draft;
            let value = draft();
            if !dirty() {
                return;
            }
            let patch = match intent.patch(value.clone()) {
                Ok(patch) => patch,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Mechanical request identity is exhausted; reopen the Case inspector."
                        .to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MechanicalSettingsRequest {
                identity: identity.clone(),
                request_id,
                field_id: patch.field_id(),
                patch,
                reviewed_fingerprint: None,
            };
            dirty.set(false);
            error.set(None);
            on_request.call(request);
        }
    });
    let error_text = error();
    let id = format!("case-text-{}", dioxus::core::current_scope_id().0);
    let message_id = format!("{id}-message");
    let invalid = error_text.is_some();
    let described_by = invalid.then(|| message_id.clone());
    rsx! {
        MechanicalBoundInputRow {
            label: props.label, html_for: id.clone(), disabled: !props.editable,
            message_id: Some(message_id), error: error_text,
            InputGroup { class: if !props.multiline { "max-[760px]:h-control-touch [@media(pointer:coarse)]:h-control-touch" },
                if props.multiline {
                    InputGroupTextarea {
                        id: id.clone(),
                        value: "{draft}",
                        rows: "2",
                        disabled: !props.editable,
                        aria_label: props.label,
                        aria_invalid: invalid,
                        aria_describedby: described_by.clone(),
                        oninput: move |event: FormEvent| {
                            draft.set(event.value());
                            dirty.set(true);
                            error.set(None);
                        },
                        attributes: vec![dioxus::events::onblur({
                            let commit = commit.clone();
                            move |_| commit()
                        }), dioxus::events::onkeydown({
                            let accepted = props.value.clone();
                            move |event: KeyboardEvent| {
                                let key = event.data().key().to_string();
                                if key == "Escape" {
                                    event.prevent_default();
                                    draft.set(accepted.clone());
                                    dirty.set(false);
                                    error.set(None);
                                }
                            }
                        })],
                    }
                } else {
                    InputGroupInput {
                        id: id.clone(),
                        r#type: InputType::Text,
                        value: "{draft}",
                        disabled: !props.editable,
                        aria_label: props.label,
                        aria_invalid: invalid,
                        aria_describedby: described_by,
                        oninput: move |event: FormEvent| {
                            draft.set(event.value());
                            dirty.set(true);
                            error.set(None);
                        },
                        attributes: vec![dioxus::events::onblur({
                            let commit = commit.clone();
                            move |_| commit()
                        }), dioxus::events::onkeydown({
                            let accepted = props.value.clone();
                            move |event: KeyboardEvent| {
                                let key = event.data().key().to_string();
                                if key == "Enter" {
                                    event.prevent_default();
                                    if let Some(input) = event
                                        .data()
                                        .try_as_web_event()
                                        .and_then(|event| event.target())
                                        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                    {
                                        let _ = input.blur();
                                    }
                                } else if key == "Escape" {
                                    event.prevent_default();
                                    draft.set(accepted.clone());
                                    dirty.set(false);
                                    error.set(None);
                                }
                            }
                        })],
                    }
                }
            }
        }
    }
}
