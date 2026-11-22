//! Closure hardware, fastener specifications and critical fit dimensions.
use crate::mechanical_projection::{
    MechanicalFitPart, MechanicalHardwareMount, critical_fit_parts, hardware_mount_choices,
};
use crate::mechanical_rules::{
    CLOSURE_INSERT_PRESETS, closure_dimension_fields, closure_preset_id, parse_screw_lengths,
};
use crate::mechanical_settings::{
    Fields, MechanicalDimension, MechanicalSettingsPatch, Sender, dimension,
};
use boardstudio_core_mechanical::mechanical::change::{
    MechanicalCriticalFitTextField, MechanicalHardwareTextField,
};
use boardstudio_model::model::{
    InsertInstallation, InternalClosureHardware, MechanicalConfiguration, MountKind, ScrewDrive,
    ScrewHeadProfile, ScrewLengthDatum,
};
use boardstudio_web_ui_shared::inspector_fields::{InspectorSection, SelectField, TextCommitField};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonSize, ButtonVariant},
};
use dioxus::prelude::*;
use std::rc::Rc;

fn mount_label(choice: &MechanicalHardwareMount) -> String {
    let kind = if choice.kind == MountKind::Boss {
        "boss"
    } else {
        "hole"
    };
    format!(
        "{} · {kind} · {:.1}, {:.1} mm",
        choice.part_name, choice.x, choice.y
    )
}

#[component]
pub(crate) fn ClosureHardwareSection(
    hardware: InternalClosureHardware,
    editable: bool,
    send: Sender,
) -> Element {
    let mut lengths_error = use_signal(|| None::<String>);
    let preset = closure_preset_id(&hardware).unwrap_or("custom");
    let mut presets = vec![("custom".to_owned(), "Custom dimensions".to_owned())];
    presets.extend(
        CLOSURE_INSERT_PRESETS
            .iter()
            .map(|(id, label, ..)| ((*id).to_owned(), (*label).to_owned())),
    );
    let fixed = hardware
        .fixed_length
        .map_or_else(|| "auto".to_owned(), |length| length.to_string());
    let mut length_options = vec![(
        "auto".to_owned(),
        "Automatic from available lengths".to_owned(),
    )];
    length_options.extend(
        hardware
            .screw_lengths
            .iter()
            .map(|length| (length.to_string(), format!("{length} mm"))),
    );
    let known = hardware.screw_lengths.clone();
    rsx! {
        InspectorSection { title: format!("Closure hardware · {}", hardware.thread),
            Fields { editable,
                p { class: "text-muted-foreground text-sm", "Custom screw and insert dimensions. Review these against your hardware; the starting M2 dimensions are not a supplier specification." }
                SelectField::<String> { label: "Insert size", value: preset.to_owned(), options: presets,
                    on_commit: move |id: String| if id != "custom" { send.send(MechanicalSettingsPatch::SetClosureInsertPreset(id)) } }
                SelectField::<ScrewDrive> { label: "Screw drive", value: hardware.drive.clone(),
                    options: vec![(ScrewDrive::Hex, "Hex socket".into()), (ScrewDrive::Torx, "Torx".into())],
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetClosureDrive(v)) }
                SelectField::<InsertInstallation> { label: "Insert installation", value: hardware.installation.clone(),
                    options: vec![(InsertInstallation::HeatSet, "Heat-set · printed top".into()), (InsertInstallation::Tapped, "Tapped · machined top".into())],
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetClosureInstallation(v)) }
                SelectField::<String> { label: "Screw length", value: fixed, options: length_options,
                    on_commit: move |next: String| {
                        let fixed = next.parse::<f64>().ok();
                        if fixed.is_none() || fixed.is_some_and(|length| known.contains(&length)) { send.send(MechanicalSettingsPatch::SetClosureFixedLength(fixed)) }
                    } }
                TextCommitField { label: "Available screw lengths (mm)", value: hardware.screw_lengths.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "),
                    on_commit: move |text: String| match parse_screw_lengths(&text) {
                        Ok(lengths) => { lengths_error.set(None); send.send(MechanicalSettingsPatch::SetClosureScrewLengths(lengths)) }
                        Err(message) => lengths_error.set(Some(message)),
                    } }
                if let Some(message) = lengths_error() { Alert { variant: AlertVariant::Destructive, "{message}" } }
                InspectorSection { title: "Advanced closure dimensions · mm", default_open: false,
                    TextCommitField { label: "Thread designation", value: hardware.thread.clone(), on_commit: move |v| send.send(MechanicalSettingsPatch::SetClosureThread(v)) }
                    SelectField::<ScrewHeadProfile> { label: "Screw head profile", value: hardware.head_profile.clone(),
                        options: vec![(ScrewHeadProfile::Flat, "Flat bearing surface".into()), (ScrewHeadProfile::Countersunk, "Countersunk".into())],
                        on_commit: move |v| send.send(MechanicalSettingsPatch::SetClosureHeadProfile(v)) }
                    SelectField::<ScrewLengthDatum> { label: "Screw length measured from", value: hardware.length_datum.clone(),
                        options: vec![(ScrewLengthDatum::UnderHead, "Under the head".into()), (ScrewLengthDatum::Overall, "Top of head · overall".into()), (ScrewLengthDatum::Unresolved, "Unknown".into())],
                        on_commit: move |v| send.send(MechanicalSettingsPatch::SetClosureLengthDatum(v)) }
                    for (field, label, value) in closure_dimension_fields(&hardware) {
                        {dimension(label, field, value, move |value| send.send(MechanicalSettingsPatch::SetDimension { field, value }))}
                    }
                    p { class: "text-muted-foreground text-sm", "Thread changes do not resize the insert. Countersunk lengths include the head." }
                }
            }
        }
    }
}

#[component]
pub(crate) fn HardwareAndFitsSection(
    configuration: Rc<MechanicalConfiguration>,
    fit_parts: Rc<[MechanicalFitPart]>,
    fit_parts_resolved: bool,
    hardware_mounts: Rc<[MechanicalHardwareMount]>,
    editable: bool,
    send: Sender,
) -> Element {
    let parts = critical_fit_parts(&configuration, &fit_parts, fit_parts_resolved);
    let choices = hardware_mount_choices(&configuration, &hardware_mounts);
    let hardware = configuration.hardware.clone().unwrap_or_default();
    let fits = configuration.critical_fits.clone().unwrap_or_default();
    let (fit_count, hardware_count) = (fits.len(), hardware.len());
    rsx! {
        InspectorSection { title: "Hardware & critical fits", default_open: hardware_count + fit_count > 0, badge: format!("{hardware_count} hardware · {fit_count} fits"),
            Fields { editable,
                h3 { "Hardware" }
                if choices.is_empty() { p { class: "text-muted-foreground text-sm", "Add a suspension or closure mount to link a fastener specification to its generated feature." } }
                Button { variant: ButtonVariant::Secondary, disabled: choices.is_empty(),
                    onclick: { let first = choices.first().cloned(); move |_| if let Some(choice) = first.as_ref() { send.send(MechanicalSettingsPatch::AddHardware { part_id: choice.part_id.clone(), feature_id: choice.feature_id.clone() }) } }, "Add hardware" }
                for item in hardware {
                    article { key: "{item.id}", class: "grid min-w-0 gap-2 rounded-md border border-border bg-card p-2.5",
                        strong { if item.designation.is_empty() { "Hardware item" } else { "{item.designation}" } }
                        SelectField::<usize> { label: "Mount link", aria_label: "Mount link for hardware {item.id}", placeholder: "Choose generated mount…",
                            value: choices.iter().position(|choice| choice.part_id == item.part_id && choice.feature_id == item.feature_id).unwrap_or(usize::MAX),
                            options: choices.iter().enumerate().map(|(index, choice)| (index, mount_label(choice))).collect::<Vec<_>>(),
                            on_commit: { let (id, choices) = (item.id.clone(), choices.clone()); move |index: usize| if let Some(choice) = choices.get(index) {
                                send.send(MechanicalSettingsPatch::SetHardwareMount { hardware_id: id.clone(), part_id: choice.part_id.clone(), feature_id: choice.feature_id.clone() })
                            } } }
                        for (label, field, value) in [
                            ("Designation", MechanicalHardwareTextField::Designation, item.designation.clone()),
                            ("Thread", MechanicalHardwareTextField::Thread, item.thread.clone()),
                            ("Tolerance", MechanicalHardwareTextField::Tolerance, item.tolerance.clone().unwrap_or_default()),
                            ("Notes", MechanicalHardwareTextField::Notes, item.notes.clone().unwrap_or_default()),
                        ] {
                            TextCommitField { key: "{label}", label, value, on_commit: { let id = item.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetHardwareText { hardware_id: id.clone(), field, value }) } }
                        }
                        for (label, field, value) in [("Length", MechanicalDimension::HardwareLength, item.length), ("Quantity", MechanicalDimension::HardwareQuantity, f64::from(item.quantity))] {
                            {dimension(label, field, value, { let id = item.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetHardwareDimension { hardware_id: id.clone(), field, value }) })}
                        }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove hardware {item.id}",
                            onclick: { let id = item.id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveHardware { hardware_id: id.clone() }) }, "Remove" }
                    }
                }
                h3 { "Critical fits" }
                p { class: "text-muted-foreground text-sm", "Endpoints are document XY coordinates. Length is calculated from the saved endpoints." }
                Button { variant: ButtonVariant::Secondary,
                    onclick: { let part_id = parts.first().map_or_else(|| "plate".to_owned(), |part| part.id.clone()); move |_| send.send(MechanicalSettingsPatch::AddCriticalFit { part_id: part_id.clone() }) }, "Add fit dimension" }
                for fit in fits {
                    article { key: "{fit.id}", class: "grid min-w-0 gap-2 rounded-md border border-border bg-card p-2.5",
                        strong { if fit.label.is_empty() { "Fit dimension" } else { "{fit.label}" } }
                        SelectField::<String> { label: "Generated part", aria_label: "Part for critical fit {fit.id}", value: fit.part_id.clone(),
                            options: parts.iter().map(|part| (part.id.clone(), part.name.clone())).collect::<Vec<_>>(),
                            on_commit: { let id = fit.id.clone(); move |part_id| send.send(MechanicalSettingsPatch::SetCriticalFitPart { fit_id: id.clone(), part_id }) } }
                        for (label, field, value) in [("Dimension name", MechanicalCriticalFitTextField::Label, fit.label.clone()), ("Tolerance", MechanicalCriticalFitTextField::Tolerance, fit.tolerance.clone())] {
                            TextCommitField { key: "{label}", label, value, on_commit: { let id = fit.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetCriticalFitText { fit_id: id.clone(), field, value }) } }
                        }
                        for (label, field, value) in [
                            ("From X", MechanicalDimension::CriticalFitFromX, fit.from.x), ("From Y", MechanicalDimension::CriticalFitFromY, fit.from.y),
                            ("To X", MechanicalDimension::CriticalFitToX, fit.to.x), ("To Y", MechanicalDimension::CriticalFitToY, fit.to.y),
                        ] {
                            {dimension(label, field, value, { let id = fit.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetCriticalFitPoint { fit_id: id.clone(), field, value }) })}
                        }
                        output { class: "text-muted-foreground text-[11px] font-mono", "Measured {(fit.to.x - fit.from.x).hypot(fit.to.y - fit.from.y):.2} mm · {fit.tolerance}" }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove fit {fit.id}",
                            onclick: { let id = fit.id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveCriticalFit { fit_id: id.clone() }) }, "Remove" }
                    }
                }
            }
        }
    }
}
