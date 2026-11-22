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
use boardstudio_web_ui_shared::inspector_fields::CheckboxField;
use boardstudio_web_ui_shared::inspector_fields::SelectField;
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
pub(crate) struct GasketClosureControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) hardware: Option<InternalClosureHardware>,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) owner_key: String,
}

#[component]
pub(crate) fn GasketClosureControls(props: GasketClosureControlsProps) -> Element {
    let Some(hardware) = props.hardware.as_ref() else {
        return rsx! {};
    };
    let selected_preset = [
        ("m2-3", 2.0, 0.4, 3.0, 3.2),
        ("m2-4", 2.0, 0.4, 4.0, 3.2),
        ("m2.5-3", 2.5, 0.45, 3.0, 3.5),
        ("m2.5-4", 2.5, 0.45, 4.0, 3.5),
        ("m2.5-5", 2.5, 0.45, 5.0, 3.5),
        ("m3-3", 3.0, 0.5, 3.0, 4.2),
    ]
    .into_iter()
    .find(|(_, diameter, _, length, insert_diameter)| {
        hardware.thread_diameter == *diameter
            && hardware.insert_length == *length
            && hardware.insert_diameter == *insert_diameter
    })
    .map(|(id, ..)| id);
    let selected_length = hardware
        .fixed_length
        .map_or_else(|| "auto".to_owned(), |length| length.to_string());
    let request_sequence = props.request_sequence;
    let identity = props.identity.clone();
    let on_request = props.on_request;
    rsx! {
        Collapsible { keep_mounted: true, default_open: true,
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Closure hardware · {hardware.thread}" }
            CollapsibleContent { class: "data-[open=true]:overflow-visible data-[open=false]:hidden",
            p { class: "text-muted-foreground text-sm leading-normal", "Custom screw and insert dimensions. Review these against your hardware; the starting M2 dimensions are not a supplier preset." }
            SelectField::<String> {
                label: "Insert size",
                aria_label: "Insert size",
                disabled: !props.editable,
                value: (selected_preset.unwrap_or("custom")).to_string(),
                on_commit: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        move |next_val: String| { let next = Some(next_val);
                            if let Some(id) = next && id != "custom" {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureInsertPreset(id));
                            }
                        }
                    },
                options: vec![
                    ("custom".into(), "Custom dimensions".into()),
                    ("m2-3".into(), "M2 × L3 × OD3.2".into()),
                    ("m2-4".into(), "M2 × L4 × OD3.2".into()),
                    ("m2.5-3".into(), "M2.5 × L3 × OD3.5".into()),
                    ("m2.5-4".into(), "M2.5 × L4 × OD3.5".into()),
                    ("m2.5-5".into(), "M2.5 × L5 × OD3.5".into()),
                    ("m3-3".into(), "M3 × L3 × OD4.2".into()),
                ],
            }
            SelectField::<String> {
                label: "Screw drive",
                disabled: !props.editable,
                value: (if hardware.drive == ScrewDrive::Hex { "hex" } else { "torx" }).to_string(),
                on_commit: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        move |next_val: String| { let next = Some(next_val);
                            let drive = if next.as_deref() == Some("torx") { ScrewDrive::Torx } else { ScrewDrive::Hex };
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureDrive(drive));
                        }
                    },
                options: vec![
                    ("hex".into(), "Hex socket".into()),
                    ("torx".into(), "Torx".into()),
                ],
            }
            SelectField::<String> {
                label: "Insert installation",
                disabled: !props.editable,
                value: (if hardware.installation == InsertInstallation::HeatSet { "heat-set" } else { "tapped" }).to_string(),
                on_commit: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        move |next_val: String| { let next = Some(next_val);
                            let installation = if next.as_deref() == Some("tapped") { InsertInstallation::Tapped } else { InsertInstallation::HeatSet };
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureInstallation(installation));
                        }
                    },
                options: vec![
                    ("heat-set".into(), "Heat-set · printed top".into()),
                    ("tapped".into(), "Tapped · machined top".into()),
                ],
            }
            SelectField::<String> {
                label: "Screw length",
                aria_label: "Screw length",
                disabled: !props.editable,
                value: selected_length.clone(),
                options: std::iter::once(("auto".to_owned(), "Automatic from available lengths".to_owned())).chain(hardware.screw_lengths.iter().map(|length| (length.to_string(), format!("{length} mm · fixed")))).collect::<Vec<_>>(),
                on_commit: {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        let lengths = hardware.screw_lengths.clone();
                        move |next_val: String| { let next = Some(next_val);
                            let fixed = next.as_deref().unwrap_or_default().parse::<f64>().ok();
                            if fixed.is_none() || fixed.is_some_and(|length| lengths.contains(&length)) {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureFixedLength(fixed));
                            }
                        }
                    },
            }
            { rsx! { TextDraftField {
                key: "{props.owner_key}:closure-screw-lengths",
                identity: props.identity.clone(),
                request_sequence,
                on_request,
                feedback: props.feedback.clone(),
                label: "Available screw lengths (mm)",
                value: hardware.screw_lengths.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "),
                intent: MechanicalTextIntent::ClosureScrewLengths,
                editable: props.editable,
            } } }
            Collapsible { keep_mounted: true, default_open: false,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Advanced closure dimensions · Custom · mm" }
                CollapsibleContent { class: "data-[open=true]:overflow-visible data-[open=false]:hidden",
                { rsx! { TextDraftField {
                    key: "{props.owner_key}:closure-thread",
                    identity: props.identity.clone(),
                    request_sequence,
                    on_request,
                    feedback: props.feedback.clone(),
                    label: "Thread designation",
                    value: hardware.thread.clone(),
                    intent: MechanicalTextIntent::ClosureThread,
                    editable: props.editable,
                } } }
                SelectField::<String> {
                    label: "Screw head profile",
                    disabled: !props.editable,
                    value: (if hardware.head_profile == ScrewHeadProfile::Countersunk { "countersunk" } else { "flat" }).to_string(),
                    on_commit: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            move |next_val: String| { let next = Some(next_val);
                                let profile = if next.as_deref() == Some("countersunk") { ScrewHeadProfile::Countersunk } else { ScrewHeadProfile::Flat };
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureHeadProfile(profile));
                            }
                        },
                    options: vec![
                        ("flat".into(), "Flat bearing surface".into()),
                        ("countersunk".into(), "Countersunk".into()),
                    ],
                }
                SelectField::<String> {
                    label: "Screw length measured from",
                    disabled: !props.editable,
                    value: (match hardware.length_datum {
                            ScrewLengthDatum::UnderHead => "under-head",
                            ScrewLengthDatum::Overall => "overall",
                            ScrewLengthDatum::Unresolved => "unresolved",
                        }).to_string(),
                    on_commit: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            move |next_val: String| { let next = Some(next_val);
                                let datum = match next.as_deref().unwrap_or_default() {
                                    "overall" => ScrewLengthDatum::Overall,
                                    "unresolved" => ScrewLengthDatum::Unresolved,
                                    _ => ScrewLengthDatum::UnderHead,
                                };
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetClosureLengthDatum(datum));
                            }
                        },
                    options: vec![
                        ("under-head".into(), "Under the head".into()),
                        ("overall".into(), "Top of head · overall".into()),
                        ("unresolved".into(), "Unknown".into()),
                    ],
                }
                for (field, label, value) in closure_dimension_fields(hardware) {
                    { rsx! { DimensionField {
                        key: "{props.owner_key}:{field.field_id()}",
                        identity: props.identity.clone(),
                        request_sequence,
                        on_request,
                        feedback: props.feedback.clone(),
                        field,
                        label,
                        value,
                        editable: props.editable,
                    } } }
                }
                p { class: "text-muted-foreground text-sm leading-normal", "Thread changes do not resize the insert. Countersunk lengths include the head. Incompatible dimensions appear in Mechanical diagnostics." }
                 }
                }
             }
            }
    }
}

pub(crate) fn closure_dimension_fields(
    hardware: &InternalClosureHardware,
) -> [(MechanicalDimension, &'static str, f64); 18] {
    [
        (
            MechanicalDimension::ClosureThreadDiameter,
            "Thread diameter",
            hardware.thread_diameter,
        ),
        (
            MechanicalDimension::ClosurePitch,
            "Thread pitch",
            hardware.pitch,
        ),
        (
            MechanicalDimension::ClosureHeadDiameter,
            "Head diameter",
            hardware.head_diameter,
        ),
        (
            MechanicalDimension::ClosureHeadHeight,
            "Head height",
            hardware.head_height,
        ),
        (
            MechanicalDimension::ClosureHoleDiameter,
            "Screw clearance hole",
            hardware.hole_diameter,
        ),
        (
            MechanicalDimension::ClosureInsertDiameter,
            "Insert outside diameter",
            hardware.insert_diameter,
        ),
        (
            MechanicalDimension::ClosureInsertLength,
            "Insert length",
            hardware.insert_length,
        ),
        (
            MechanicalDimension::ClosureSeatDiameter,
            "Insert seat diameter",
            hardware.seat_diameter,
        ),
        (
            MechanicalDimension::ClosureSeatDepth,
            "Insert seat depth",
            hardware.seat_depth,
        ),
        (
            MechanicalDimension::ClosureEngagement,
            "Thread engagement",
            hardware.engagement,
        ),
        (
            MechanicalDimension::ClosureThreadStart,
            "Thread start offset",
            hardware.thread_start,
        ),
        (
            MechanicalDimension::ClosureTipAllowance,
            "Screw tip allowance",
            hardware.tip_allowance,
        ),
        (
            MechanicalDimension::ClosureBottomingClearance,
            "Bottoming clearance",
            hardware.bottoming_clearance,
        ),
        (
            MechanicalDimension::ClosureRoof,
            "Roof above insert",
            hardware.roof,
        ),
        (
            MechanicalDimension::ClosureSurround,
            "Material around insert",
            hardware.surround,
        ),
        (
            MechanicalDimension::ClosureSeatLeadDepth,
            "Seat lead-in depth",
            hardware.seat_lead_depth,
        ),
        (
            MechanicalDimension::ClosureSeatLeadDiameter,
            "Seat lead-in diameter",
            hardware.seat_lead_diameter,
        ),
        (
            MechanicalDimension::ClosureBearingThickness,
            "Material above screw head",
            hardware.bearing_thickness,
        ),
    ]
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct CriticalFitControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) hardware: Vec<MechanicalHardwareSpecification>,
    pub(crate) hardware_mounts: Rc<[MechanicalHardwareMount]>,
    pub(crate) fits: Vec<MechanicalCriticalFit>,
    pub(crate) fit_parts: Rc<[MechanicalFitPart]>,
    pub(crate) fit_parts_resolved: bool,
    pub(crate) values: MechanicalSettingsValues,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) owner_key: String,
}

#[component]
pub(crate) fn CriticalFitControls(props: CriticalFitControlsProps) -> Element {
    let parts = critical_fit_parts(&props.values, &props.fit_parts, props.fit_parts_resolved);
    let mount_choices = hardware_mount_choices(&props.values, &props.hardware_mounts);
    let hardware = &props.hardware;
    let fits = &props.fits;
    let fit_rows = fits
        .iter()
        .cloned()
        .map(|fit| {
            let length = ((fit.to.x - fit.from.x).powi(2) + (fit.to.y - fit.from.y).powi(2)).sqrt();
            (fit, length)
        })
        .collect::<Vec<_>>();
    let request_sequence = props.request_sequence;
    let identity = props.identity.clone();
    let on_request = props.on_request;
    rsx! {
        Collapsible { keep_mounted: true, default_open: !hardware.is_empty() || !fits.is_empty(),
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Hardware & critical fits · {hardware.len()} hardware · {fits.len()} fits" }
            CollapsibleContent { class: "data-[open=true]:overflow-visible data-[open=false]:hidden",
            div { class: "mt-2.5 grid gap-2 border-t border-border pt-2.5 [&>header]:flex [&>header]:items-center [&>header]:justify-between [&>header]:gap-2",
                header {
                    strong { "Hardware" }
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: mechanical_action_pending("hardware:add") || !props.editable || mount_choices.is_empty(),
                        onclick: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            let choice = mount_choices.first().cloned();
                            move |_| if let Some(choice) = choice.as_ref() {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddHardware {
                                    part_id: choice.part_id.clone(), feature_id: choice.feature_id.clone(),
                                });
                            }
                        },
                        "Add hardware"
                    }
                }
                if mount_choices.is_empty() {
                    p { class: "text-muted-foreground text-sm leading-normal", "Add a suspension or closure mount to link a fastener specification to its generated feature." }
                }
                for item in hardware.iter() {
                    article { class: "grid min-w-0 gap-2 rounded-md border border-border bg-card p-2.5 [&>header]:flex [&>header]:items-center [&>header]:justify-between [&>header]:gap-2", key: "{props.owner_key}:hardware:{item.id}",
                        header {
                            strong { if item.designation.is_empty() { "Hardware item" } else { "{item.designation}" } }
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Remove hardware {item.id}",
                                disabled: mechanical_action_pending(&format!("hardware:{}:remove", item.id)) || !props.editable,
                                onclick: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let hardware_id = item.id.clone();
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveHardware { hardware_id: hardware_id.clone() })
                                },
                                "Remove"
                            }
                        }
                        SelectField::<String> {
                            label: "Mount link",
                            aria_label: "Mount link for hardware {item.id}",
                            placeholder: "Choose generated mount…",
                            disabled: !props.editable,
                            value: mount_choices.iter().position(|choice| choice.part_id == item.part_id && choice.feature_id == item.feature_id).map(|index| index.to_string()).unwrap_or_default(),
                            options: mount_choices.iter().enumerate().map(|(index, choice)| (index.to_string(), format!("{} · {} · {:.1}, {:.1} mm", choice.part_name, mount_kind_label(&choice.kind), choice.x, choice.y))).collect::<Vec<_>>(),
                            on_commit: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let hardware_id = item.id.clone();
                                    let mount_choices = mount_choices.clone();
                                    move |next_val: String| { let next = Some(next_val); if let Some(next) = next && let Ok(index) = next.parse::<usize>()
                                        && let Some(choice) = mount_choices.get(index) {
                                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetHardwareMount {
                                                hardware_id: hardware_id.clone(),
                                                part_id: choice.part_id.clone(),
                                                feature_id: choice.feature_id.clone(),
                                            });
                                        }
                                    }
                                },
                        }
                        for (suffix, label, value, field, multiline) in [
                            ("designation", "Designation", item.designation.clone(), MechanicalHardwareTextField::Designation, false),
                            ("thread", "Thread", item.thread.clone(), MechanicalHardwareTextField::Thread, false),
                        ] {
                            TextDraftField {
                                key: "{props.owner_key}:hardware:{item.id}:{suffix}",
                                identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                label, value,
                                intent: MechanicalTextIntent::Hardware { hardware_id: item.id.clone(), field },
                                editable: props.editable, multiline,
                            }
                        }
                        div { class: "grid grid-cols-4 items-end gap-1.5 [@container_mechanical_(max-width:320px)]:grid-cols-2",
                            for (suffix, field, label, value) in [
                                ("length", MechanicalDimension::HardwareLength, "Length", item.length),
                                ("quantity", MechanicalDimension::HardwareQuantity, "Quantity", f64::from(item.quantity)),
                            ] {
                                DimensionField {
                                    key: "{props.owner_key}:hardware:{item.id}:{suffix}",
                                    identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                    field, label, value,
                                    editable: props.editable, hardware_target: Some(HardwareDimensionTarget { hardware_id: item.id.clone() }),
                                }
                            }
                        }
                        for (suffix, label, value, field, multiline) in [
                            ("tolerance", "Tolerance", item.tolerance.clone().unwrap_or_default(), MechanicalHardwareTextField::Tolerance, false),
                            ("notes", "Notes", item.notes.clone().unwrap_or_default(), MechanicalHardwareTextField::Notes, true),
                        ] {
                            TextDraftField {
                                key: "{props.owner_key}:hardware:{item.id}:{suffix}",
                                identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                label, value,
                                intent: MechanicalTextIntent::Hardware { hardware_id: item.id.clone(), field },
                                editable: props.editable, multiline,
                            }
                        }
                    }
                }
            }
            div { class: "mt-2.5 grid gap-2 border-t border-border pt-2.5 [&>header]:flex [&>header]:items-center [&>header]:justify-between [&>header]:gap-2",
                header {
                    strong { "Critical fits" }
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: mechanical_action_pending("critical-fit:add") || !props.editable,
                        onclick: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            let part_id = parts.first().map_or_else(|| "plate".to_owned(), |part| part.id.clone());
                            move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddCriticalFit { part_id: part_id.clone() })
                        },
                        "Add fit dimension"
                    }
                }
                p { class: "text-muted-foreground text-sm leading-normal", "Endpoints are document XY coordinates. Length is calculated from the saved endpoints." }
                for (fit, measured_length) in fit_rows.iter() {
                    article { class: "grid min-w-0 gap-2 rounded-md border border-border bg-card p-2.5 [&>header]:flex [&>header]:items-center [&>header]:justify-between [&>header]:gap-2", key: "{props.owner_key}:fit:{fit.id}",
                        header {
                            strong { if fit.label.is_empty() { "Fit dimension" } else { "{fit.label}" } }
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Remove fit {fit.id}",
                                disabled: mechanical_action_pending(&format!("critical-fit:{}:remove", fit.id)) || !props.editable,
                                onclick: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let fit_id = fit.id.clone();
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveCriticalFit { fit_id: fit_id.clone() })
                                },
                                "Remove"
                            }
                        }
                        SelectField::<String> {
                            label: "Generated part",
                            aria_label: "Part for critical fit {fit.id}",
                            disabled: !props.editable,
                            value: fit.part_id.clone(),
                            options: parts.iter().map(|part| (part.id.clone(), part.name.clone())).collect::<Vec<_>>(),
                            on_commit: {
                                    let mut sequence = request_sequence;
                                    let identity = identity.clone();
                                    let fit_id = fit.id.clone();
                                    move |next_val: String| { let next = Some(next_val); if let Some(part_id) = next { send_request(
                                        &mut sequence,
                                        &identity,
                                        on_request,
                                        MechanicalSettingsPatch::SetCriticalFitPart { fit_id: fit_id.clone(), part_id },
                                    ) }
                                    }
                                },
                        }
                        for (suffix, label, value, field) in [
                            ("label", "Dimension name", fit.label.clone(), MechanicalCriticalFitTextField::Label),
                            ("tolerance", "Tolerance", fit.tolerance.clone(), MechanicalCriticalFitTextField::Tolerance),
                        ] {
                            TextDraftField {
                                key: "{props.owner_key}:fit:{fit.id}:{suffix}",
                                identity: props.identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                label, value,
                                intent: MechanicalTextIntent::CriticalFit { fit_id: fit.id.clone(), field },
                                editable: props.editable,
                            }
                        }
                        div { class: "grid grid-cols-4 items-end gap-1.5 [@container_mechanical_(max-width:320px)]:grid-cols-2",
                            for (suffix, field, label, value, direction) in [
                                ("from-x", MechanicalDimension::CriticalFitFromX, "From X", fit.from.x, "from X"),
                                ("from-y", MechanicalDimension::CriticalFitFromY, "From Y", fit.from.y, "from Y"),
                                ("to-x", MechanicalDimension::CriticalFitToX, "To X", fit.to.x, "to X"),
                                ("to-y", MechanicalDimension::CriticalFitToY, "To Y", fit.to.y, "to Y"),
                            ] {
                                DimensionField {
                                    key: "{props.owner_key}:fit:{fit.id}:{suffix}",
                                    identity: props.identity.clone(),
                                    request_sequence,
                                    on_request,
                                    feedback: props.feedback.clone(),
                                    field,
                                    label,
                                    value,
                                    editable: props.editable,
                                    accessible_label: Some(format!("Critical fit {} {}", fit.id, direction)),
                                    critical_fit_target: Some(CriticalFitDimensionTarget { fit_id: fit.id.clone() }),
                                }
                            }
                        }
                        output { class: "text-muted-foreground text-[11px] font-mono font-normal not-italic leading-[normal]",
                            "Measured {measured_length:.2} mm · {fit.tolerance}"
                        }
                    }
                }
            }
             }
            }
    }
}

pub(crate) fn critical_fit_parts(
    values: &MechanicalSettingsValues,
    resolved: &[MechanicalFitPart],
    is_resolved: bool,
) -> Vec<MechanicalFitPart> {
    if is_resolved {
        return resolved.to_vec();
    }
    let ids = [
        Some(("plate", "Plate")),
        (values.plate_foam_thickness > 0.0).then_some(("plate-foam", "Plate foam")),
        (values.bottom_foam_thickness > 0.0).then_some(("bottom-foam", "Bottom foam")),
        Some(("bottom", "Bottom")),
        (values.bottom_style == MechanicalBottomStyle::Sheet && values.middle_frame)
            .then_some(("middle-frame", "Middle frame")),
    ];
    ids.into_iter()
        .flatten()
        .map(|(id, name)| MechanicalFitPart {
            id: id.to_owned(),
            name: name.to_owned(),
        })
        .collect()
}

pub(crate) fn hardware_mount_choices(
    values: &MechanicalSettingsValues,
    resolved: &[MechanicalHardwareMount],
) -> Vec<MechanicalHardwareMount> {
    if !resolved.is_empty() {
        return resolved.to_vec();
    }
    let mut choices = Vec::new();
    if values.mount != MechanicalMount::Gasket {
        let part_id = if values.mount == MechanicalMount::Rigid {
            "plate"
        } else {
            "bottom"
        };
        let part_name = if part_id == "plate" {
            "Plate"
        } else {
            "Bottom"
        };
        choices.extend(
            values
                .suspension_mounts
                .iter()
                .map(|mount| MechanicalHardwareMount {
                    part_id: part_id.to_owned(),
                    part_name: part_name.to_owned(),
                    feature_id: mount.id.clone(),
                    kind: mount.kind.clone(),
                    x: mount.at.x,
                    y: mount.at.y,
                }),
        );
    }
    choices.extend(
        values
            .closure_mounts
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|mount| MechanicalHardwareMount {
                part_id: "bottom".to_owned(),
                part_name: "Bottom".to_owned(),
                feature_id: mount.id.clone(),
                kind: mount.kind.clone(),
                x: mount.at.x,
                y: mount.at.y,
            }),
    );
    choices
}

pub(crate) fn mount_kind_label(kind: &MountKind) -> &'static str {
    match kind {
        MountKind::Hole => "hole",
        MountKind::Boss => "boss",
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MountingControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) values: MechanicalSettingsValues,
    pub(crate) suggested_mounts: Rc<[Mount]>,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
pub(crate) fn MountingControls(props: MountingControlsProps) -> Element {
    let values = &props.values;
    let suspension_mounts = &values.suspension_mounts;
    let closure_mounts = values.closure_mounts.as_deref().unwrap_or_default();
    let reserved_mount_ids = suspension_mounts
        .iter()
        .chain(closure_mounts.iter())
        .map(|mount| mount.id.clone())
        .collect::<Vec<_>>();
    let suggestions = &props.suggested_mounts;
    rsx! {
        section { aria_label: "Mounting and hardware",
            h3 { "Mounting & hardware" }
            if values.mount != MechanicalMount::Gasket {
                MountCollectionControls {
                    identity: props.identity.clone(),
                    request_sequence: props.request_sequence,
                    on_request: props.on_request,
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    collection: MechanicalMountCollection::Suspension,
                    label: "Suspension mounts",
                    mounts: suspension_mounts.clone(),
                    reserved_mount_ids: reserved_mount_ids.clone(),
                    allow_add: true,
                }
            }
            MountCollectionControls {
                identity: props.identity.clone(),
                request_sequence: props.request_sequence,
                on_request: props.on_request,
                editable: props.editable,
                feedback: props.feedback.clone(),
                collection: MechanicalMountCollection::Closure,
                label: "Closure screws",
                mounts: closure_mounts.to_vec(),
                reserved_mount_ids: reserved_mount_ids.clone(),
                allow_add: !values.internal_gasket,
            }
            Collapsible { keep_mounted: true, default_open: false,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Suggested mount locations · {suggestions.len()} candidates" }
                CollapsibleContent { class: "data-[open=false]:hidden",
                if suggestions.is_empty() {
                    p { class: "text-muted-foreground text-sm leading-normal", "Resolve the current geometry to see clearance-tested mounting locations." }
                } else if values.internal_gasket {
                    p { class: "text-muted-foreground text-sm leading-normal", "Candidates clear the current openings and battery envelope. Adopting them is explicit; later edits keep the chosen coordinates fixed." }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: mechanical_action_pending("adopt-closure-positions") || !props.editable,
                        onclick: {
                            let mut sequence = props.request_sequence;
                            let identity = props.identity.clone();
                            let callback = props.on_request;
                            let mounts = suggestions.to_vec();
                            move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::AdoptClosurePositions(mounts.clone()))
                        },
                        "Adopt closure positions"
                    }
                } else {
                    p { class: "text-muted-foreground text-sm leading-normal", "Clearance-tested positions on this board. Adopt them as suspension mounts or closure screws." }
                    div {
                        for (index, mount) in suggestions.iter().enumerate() {
                            span { key: "{mount.id}", "{index + 1}: X {mount.at.x:.1}, Y {mount.at.y:.1} mm" }
                        }
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: mechanical_action_pending("suspension-mounts") || !props.editable,
                        onclick: {
                            let mut sequence = props.request_sequence;
                            let identity = props.identity.clone();
                            let callback = props.on_request;
                            let mounts = suggestions.to_vec();
                            move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::SetMountCollection { collection: MechanicalMountCollection::Suspension, mounts: mounts.clone() })
                        },
                        "Adopt suggested mounts"
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: mechanical_action_pending("closure-mounts") || !props.editable,
                        onclick: {
                            let mut sequence = props.request_sequence;
                            let identity = props.identity.clone();
                            let callback = props.on_request;
                            let suggestions = suggestions.to_vec();
                            move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::AddSuggestedClosureMounts(suggestions.clone()))
                        },
                        "Add suggested closure screws"
                    }
                }
                 }
                }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MountCollectionControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) collection: MechanicalMountCollection,
    pub(crate) label: &'static str,
    pub(crate) mounts: Vec<Mount>,
    pub(crate) reserved_mount_ids: Vec<String>,
    pub(crate) allow_add: bool,
}

#[component]
pub(crate) fn MountCollectionControls(props: MountCollectionControlsProps) -> Element {
    let mounts = props.mounts.clone();
    rsx! {
        Collapsible { keep_mounted: true, default_open: false,
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "{props.label} · {mounts.len()}" }
            CollapsibleContent { class: "data-[open=true]:overflow-visible data-[open=false]:hidden",
            for (index, mount) in mounts.iter().enumerate() {
                { rsx! { MountRow {
                    key: "{mount.id}",
                    identity: props.identity.clone(),
                    request_sequence: props.request_sequence,
                    on_request: props.on_request,
                    feedback: props.feedback.clone(),
                    editable: props.editable,
                    collection: props.collection,
                    index: index + 1,
                    mount: mount.clone(),
                } } }
            }
            if mounts.is_empty() { Callout { "No {props.label.to_lowercase()} configured." } }
            if props.allow_add {
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: mechanical_action_pending(&format!("{}-mounts", mount_collection_id(props.collection))) || !props.editable,
                    onclick: {
                        let mut sequence = props.request_sequence;
                        let identity = props.identity.clone();
                        let callback = props.on_request;
                        let collection = props.collection;
                        move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::AddMount { collection })
                    },
                    "Add {props.label.trim_end_matches('s').to_lowercase()}"
                }
            }
             }
            }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MountRowProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) editable: bool,
    pub(crate) collection: MechanicalMountCollection,
    pub(crate) index: usize,
    pub(crate) mount: Mount,
}

#[component]
pub(crate) fn MountRow(props: MountRowProps) -> Element {
    let mount = &props.mount;
    let label = if mount.kind == MountKind::Boss {
        "Boss"
    } else {
        "Hole"
    };
    let field_prefix = format!("{:?}:{}", props.collection, mount.id);
    let make_target = |id: &str| MountDimensionTarget {
        collection: props.collection,
        mount_id: id.to_owned(),
    };
    rsx! {
        fieldset { disabled: !props.editable,
            legend { "{label} {props.index}" }
            SelectField::<String> {
                label: "Mount type",
                value: (if mount.kind == MountKind::Boss { "boss" } else { "hole" }).to_string(),
                on_commit: {
                        let mut sequence = props.request_sequence;
                        let identity = props.identity.clone();
                        let callback = props.on_request;
                        let collection = props.collection;
                        let mount_id = mount.id.clone();
                        move |next_val: String| { let next = Some(next_val); if let Some(kind) = parse_mount_kind(&next.unwrap_or_default()) {
                            send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::SetMountKind { collection, mount_id: mount_id.clone(), kind });
                        }
                        }
                    },
                options: vec![
                    ("hole".into(), "Hole".into()),
                    ("boss".into(), "Boss".into()),
                ],
            }
            DimensionField { key: "{field_prefix}:x",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountPositionX,
                label: "Position X", value: mount.at.x, editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            DimensionField { key: "{field_prefix}:y",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountPositionY,
                label: "Position Y", value: mount.at.y, editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            DimensionField { key: "{field_prefix}:hole",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountHoleDiameter,
                label: "Hole diameter", value: mount.hole_diameter, editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            DimensionField { key: "{field_prefix}:boss",
                identity: props.identity.clone(), request_sequence: props.request_sequence,
                on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountBossDiameter,
                label: "Boss diameter", value: mount.boss_diameter.unwrap_or(5.0), editable: props.editable, mount_target: Some(make_target(&mount.id)),
            }
            if mount.kind == MountKind::Boss {
                DimensionField { key: "{field_prefix}:height",
                    identity: props.identity.clone(), request_sequence: props.request_sequence,
                    on_request: props.on_request, feedback: props.feedback.clone(), field: MechanicalDimension::MountBossHeight,
                    label: "Boss height", value: mount.height.unwrap_or(5.0), editable: props.editable, mount_target: Some(make_target(&mount.id)),
                }
            }
            Button {
                variant: ButtonVariant::Secondary,
                button_type: "button",
                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                aria_label: "Remove {label} {props.index}",
                disabled: mechanical_action_pending(&format!("mount:{}:{}:remove", mount_collection_id(props.collection), mount.id)) || !props.editable,
                onclick: {
                    let mut sequence = props.request_sequence;
                    let identity = props.identity.clone();
                    let callback = props.on_request;
                    let collection = props.collection;
                    let mount_id = mount.id.clone();
                    move |_| send_request(&mut sequence, &identity, callback, MechanicalSettingsPatch::RemoveMount { collection, mount_id: mount_id.clone() })
                },
                "Remove"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct BatteryControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) values: MechanicalSettingsValues,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) owner_key: String,
}

#[component]
pub(crate) fn BatteryControls(props: BatteryControlsProps) -> Element {
    let battery = props.values.battery.as_ref();
    let request_sequence = props.request_sequence;
    rsx! {
        section { aria_label: "Openings and battery",
            h3 { "Openings & battery" }
            section { class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", aria_label: "Battery",
                h4 { "Battery" }
                if props.values.transport == HardwareTransport::Wireless {
                    p { class: "text-muted-foreground text-sm leading-normal", "Included for wireless. Set your cell dimensions; the battery shares the space below the PCB with bottom foam." }
                } else {
                    CheckboxField {
                        label: "Include a battery envelope".to_owned(),
                        id: format!("case-battery-{}-include", dioxus::core::current_scope_id().0),
                        aria_label: Some("Include a battery envelope".to_owned()),
                        disabled: !props.editable,
                        checked: if battery.is_some() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                        on_checked_change: {
                            let identity = props.identity.clone();
                            let on_request = props.on_request;
                            let mut sequence = request_sequence;
                            move |state: CheckboxState| send_request(
                                &mut sequence,
                                &identity,
                                on_request,
                                MechanicalSettingsPatch::SetBatteryEnabled(bool::from(state)),
                            )
                        },
                    }
                }
                if let Some(battery) = battery {
                    fieldset { class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", disabled: !props.editable,
                        legend { "Battery dimensions and cable · mm" }
                        for (suffix, field, label, value) in [
                            ("battery-width", MechanicalDimension::BatteryWidth, "Width", battery.size.x),
                            ("battery-depth", MechanicalDimension::BatteryDepth, "Depth", battery.size.y),
                            ("battery-height", MechanicalDimension::BatteryHeight, "Height", battery.size.z),
                            ("battery-cable-width", MechanicalDimension::BatteryCableWidth, "Cable width", battery.cable_width.unwrap_or(2.0)),
                            ("battery-position-x", MechanicalDimension::BatteryPositionX, "Position X", battery.at.x),
                            ("battery-position-y", MechanicalDimension::BatteryPositionY, "Position Y", battery.at.y),
                            ("battery-cable-exit-x", MechanicalDimension::BatteryCableExitX, "Cable exit X", battery.cable_exit.x),
                            ("battery-cable-exit-y", MechanicalDimension::BatteryCableExitY, "Cable exit Y", battery.cable_exit.y),
                        ] {
                            DimensionField {
                                key: "{props.owner_key}:{suffix}",
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
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct OpeningControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) values: MechanicalSettingsValues,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) owner_key: String,
}

#[component]
pub(crate) fn OpeningControls(props: OpeningControlsProps) -> Element {
    let openings = &props.values.openings;
    let identity = props.identity.clone();
    let request_sequence = props.request_sequence;
    let on_request = props.on_request;
    let owner_key = props.owner_key.clone();
    rsx! {
        Collapsible { keep_mounted: true, default_open: false,
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Case openings · {openings.len()}" }
            CollapsibleContent { class: "data-[open=false]:hidden",
            div { class: "grid min-w-0 gap-2.5",
                div { class: "flex flex-wrap items-center justify-between gap-2",
                    strong { "Document-coordinate access openings" }
                    Button {
                        variant: ButtonVariant::Secondary,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: mechanical_action_pending("case-openings") || !props.editable,
                        onclick: {
                            let mut sequence = request_sequence;
                            let identity = identity.clone();
                            move |_| {
                                send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddOpening);
                            }
                        },
                        "Add volume"
                    }
                }
                if openings.is_empty() {
                    p { class: "text-muted-foreground text-sm leading-normal", "No volumes configured." }
                }
                for (opening_index, opening) in openings.iter().enumerate() {
                    {
                        let mut sequence = request_sequence;
                        let identity = identity.clone();
                        let remove_identity = identity.clone();
                        let owner = owner_key.clone();
                        let opening_id = opening.id.clone();
                        rsx! {
                            fieldset { class: "m-0 grid min-w-0 gap-2.5 rounded-sm border border-border p-2.5", key: "{owner}:opening:{opening_id}", disabled: !props.editable,
                                div { class: "flex flex-wrap items-center justify-between gap-2",
                                    strong { "Volume {opening_index + 1}" }
                                    Button {
                                        variant: ButtonVariant::Secondary,
                                        button_type: "button",
                                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                        aria_label: "Remove volume {opening_index + 1}",
                                        disabled: mechanical_action_pending("case-openings") || !props.editable,
                                        onclick: {
                                            let opening_id = opening.id.clone();
                                            move |_| send_request(&mut sequence, &remove_identity, on_request, MechanicalSettingsPatch::RemoveOpening { opening_id: opening_id.clone() })
                                        },
                                        "Remove"
                                    }
                                }
                                div { class: "grid grid-cols-[repeat(auto-fit,minmax(150px,1fr))] gap-2.5",
                                    { rsx! { DimensionField {
                                        key: "{owner}:opening:{opening_id}:bottom-z",
                                        identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                        field: MechanicalDimension::OpeningBottomZ, label: "Bottom Z", value: opening.z, editable: props.editable,
                                        accessible_label: Some(format!("Document-coordinate access opening {} bottom Z", opening_index + 1)),
                                        opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: None }),
                                    } } }
                                    { rsx! { DimensionField {
                                        key: "{owner}:opening:{opening_id}:height",
                                        identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                        field: MechanicalDimension::OpeningHeight, label: "Height", value: opening.height, editable: props.editable,
                                        accessible_label: Some(format!("Document-coordinate access opening {} height", opening_index + 1)),
                                        opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: None }),
                                    } } }
                                }
                                h4 { "XY footprint" }
                                for (point_index, vertex) in opening.vertices.iter().enumerate() {
                                    div { class: "grid grid-cols-[72px_repeat(2,minmax(0,1fr))] items-end gap-2.5 [&>span]:pb-2 [&>span]:text-xs [&>span]:text-muted-foreground",
                                        span { "Vertex {point_index + 1}" }
                                        { rsx! { DimensionField {
                                            key: "{owner}:opening:{opening_id}:point:{point_index}:x",
                                            identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                            field: MechanicalDimension::OpeningPointX, label: "X", value: vertex.point.x, editable: props.editable,
                                            accessible_label: Some(format!("Document-coordinate access opening {} vertex {} X", opening_index + 1, point_index + 1)),
                                            opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: Some(vertex.id.clone()) }),
                                        } } }
                                        { rsx! { DimensionField {
                                            key: "{owner}:opening:{opening_id}:point:{point_index}:y",
                                            identity: identity.clone(), request_sequence, on_request, feedback: props.feedback.clone(),
                                            field: MechanicalDimension::OpeningPointY, label: "Y", value: vertex.point.y, editable: props.editable,
                                            accessible_label: Some(format!("Document-coordinate access opening {} vertex {} Y", opening_index + 1, point_index + 1)),
                                            opening_target: Some(OpeningDimensionTarget { opening_id: opening_id.clone(), vertex_id: Some(vertex.id.clone()) }),
                                        } } }
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            size: ButtonSize::Sm,
                                            button_type: "button",
                                            class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                            aria_label: "Remove access opening {opening_index + 1} vertex {point_index + 1}",
                                            disabled: mechanical_action_pending("case-openings") || !props.editable || opening.vertices.len() <= 3,
                                            onclick: {
                                                let mut sequence = request_sequence;
                                                let identity = identity.clone();
                                                let opening_id = opening_id.clone();
                                                let vertex_id = vertex.id.clone();
                                                move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::RemoveOpeningVertex { opening_id: opening_id.clone(), vertex_id: vertex_id.clone() })
                                            },
                                            "×"
                                        }
                                    }
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    button_type: "button",
                                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                    aria_label: "Add vertex to volume {opening_index + 1}",
                                    disabled: mechanical_action_pending("case-openings") || !props.editable,
                                    onclick: {
                                        let mut sequence = request_sequence;
                                        let identity = identity.clone();
                                        let opening_id = opening_id.clone();
                                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::AddOpeningVertex { opening_id: opening_id.clone() })
                                    },
                                    "Add vertex"
                                }
                                if opening.vertices.len() < 3 {
                                    Alert { variant: AlertVariant::Destructive, "An opening footprint needs at least three vertices." }
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
