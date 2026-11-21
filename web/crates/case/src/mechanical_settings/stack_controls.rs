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
use boardstudio_web_ui_shared::choice_field::{ChoiceField, ChoiceFieldLayout};
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
#[component]
pub fn MechanicalSettings(props: MechanicalSettingsProps) -> Element {
    let pending_feedback = props.feedback.clone();
    let action_pending = use_memo(use_reactive((&pending_feedback,), |(feedback,)| {
        feedback
            .iter()
            .filter(|entry| entry.state == MechanicalSettingsFeedbackState::Pending)
            .map(|entry| entry.field_id.clone())
            .collect()
    }));
    use_context_provider(|| MechanicalActionsPending(action_pending));

    let request_sequence = props.request_sequence;
    let identity = &props.identity;
    let configuration_matches = props
        .values
        .as_ref()
        .is_some_and(|values| values.board_id == identity.active_board_id);
    let owner_key = format!(
        "{}:{}:{}:{}:{}:{}:{:?}",
        identity.editor_instance_id,
        identity.scope_generation,
        identity.presentation_generation,
        identity.scope.session_epoch.0,
        identity.scope.document_id,
        identity.scope.board_id,
        identity.scope.instance_id,
    );
    let current_feedback = props.summary_feedback.as_ref().filter(|feedback| {
        feedback.identity.editor_instance_id == identity.editor_instance_id
            && feedback.identity.scope_generation == identity.scope_generation
            && feedback.identity.presentation_generation == identity.presentation_generation
            && feedback.identity.scope == identity.scope
            && feedback.identity.active_board_id == identity.active_board_id
            && feedback.identity.configuration_board_id == identity.configuration_board_id
    });

    if let Some(values) = props.values.as_ref().filter(|values| {
        values.board_id == identity.active_board_id
            && values.board_id == identity.configuration_board_id
    }) && let Some(layer) = props
        .layers
        .iter()
        .find(|layer| layer.id == props.selected_layer)
        && contextual_layer_fields(&layer.id, values).is_some()
    {
        return rsx! {
            ContextualLayerInspector {
                identity: props.identity.clone(),
                request_sequence,
                values: values.clone(),
                layer: layer.clone(),
                editable: props.editable,
                disabled_reason: props.disabled_reason.clone(),
                feedback: props.feedback.clone(),
                findings: props.findings.clone(),
                on_request: props.on_request,
                on_select_layer: props.on_select_layer,
                on_show_finding: props.on_show_finding,
                owner_key: owner_key.clone(),
            }
        };
    }

    if props.values.as_ref().is_some_and(|values| {
        values.board_id == identity.active_board_id
            && values.board_id == identity.configuration_board_id
    }) && let Some(support) = props.gasket_supports.iter().find(|support| {
        props.selected_layer == format!("gasket:{}:lower", support.id)
            || props.selected_layer == format!("gasket:{}:upper", support.id)
    }) {
        let anchors = props
            .gasket_supports
            .iter()
            .filter(|candidate| {
                candidate.id == support.id
                    || (!support.unlinked
                        && support.pair_id.as_deref() == Some(candidate.id.as_str())
                        && !candidate.unlinked)
            })
            .map(MechanicalGasketSupportRow::saved_anchor)
            .collect::<Vec<_>>();
        let index = props
            .gasket_supports
            .iter()
            .position(|candidate| candidate.id == support.id)
            .unwrap_or_default()
            + 1;
        return rsx! {
            section { class: "grid gap-3 [&_h3]:m-0 [&_p]:m-0 [&_label]:grid-cols-1 [&_small]:text-xs [&_small]:leading-normal [&_small]:text-muted-foreground", aria_label: "Mechanical stack settings",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    onclick: move |_| props.on_select_layer.call(String::new()),
                    "Assembly settings"
                }
                h2 { "Gasket {index}" }
                if support.is_previous {
                    div { role: "status", Callout { "Showing previous generated geometry. Gasket measurements cannot be edited until the current assembly resolves." } }
                } else {
                    p { class: "text-muted-foreground text-sm leading-normal",
                        if support.unlinked { "This gasket is unlinked from its pair." } else { "This support is linked to its mirrored pair. Matching upper and lower pads resize together." }
                    }
                    if let Some(error) = support.fit_error.as_deref() {
                        Alert { variant: AlertVariant::Destructive, "{error} Preview and export stay blocked until it fits." }
                    } else {
                        p { class: "text-muted-foreground text-sm leading-normal", "Fits at this position." }
                    }
                    for gasket_owner in [format!("{owner_key}:gasket:{}", support.id)] {
                    fieldset { key: "{gasket_owner}", class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", disabled: !props.editable,
                        DimensionField {
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::GasketSupportLength,
                            label: "Cut length",
                            value: support.length,
                            editable: props.editable,
                            support_target: Some(GasketSupportDimensionTarget {
                                support_id: support.id.clone(), anchors: anchors.clone(),
                            }),
                        }
                        DimensionField {
                            identity: props.identity.clone(),
                            request_sequence,
                            on_request: props.on_request,
                            feedback: props.feedback.clone(),
                            field: MechanicalDimension::GasketSupportWidth,
                            label: "Pad width",
                            value: support.width,
                            editable: props.editable,
                            support_target: Some(GasketSupportDimensionTarget {
                                support_id: support.id.clone(), anchors: anchors.clone(),
                            }),
                        }
                        if !props.editable {
                            if let Some(reason) = props.disabled_reason.as_deref() {
                                div { role: "status", Callout { "{reason}" } }
                            }
                        }
                    }
                    }
                    p { class: "text-muted-foreground text-sm leading-normal", "Prefer 10 mm cuts; use 5 mm increments for a tighter fit. Foam thickness and compression are shared by the floating stack." }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        onclick: move |_| props.on_select_layer.call("gaskets".to_owned()),
                        "All gasket settings"
                    }
                }
                if !support.is_previous {
                    {gasket_fit_issues(props.findings.clone(), props.on_show_finding)}
                }
            }
        };
    }

    rsx! {
        section { class: "grid gap-3 [&_h3]:m-0 [&_p]:m-0 [&_label]:grid-cols-1 [&_small]:text-xs [&_small]:leading-normal [&_small]:text-muted-foreground", aria_label: "Mechanical stack settings",
            h2 { "Case construction" }
            if let Some(values) = props.values.as_ref().filter(|_| configuration_matches).or(props.preview_values.as_ref()) {
                StackupPreview { values: values.clone() }
            }
            Collapsible { keep_mounted: true, default_open: false,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Linked halves" }
                CollapsibleContent { class: "data-[open=true]:overflow-visible data-[open=false]:hidden",
            if let Some(features) = props.pair_features.as_ref().filter(|features| !features.is_empty()) {
                MechanicalPairControls {
                    identity: props.identity.clone(),
                    request_sequence,
                    features: features.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                }
            } else if let Some(options) = props.pair_link_options.as_ref().filter(|options| !options.is_empty()) {
                MechanicalPairLinkControls {
                    identity: props.identity.clone(),
                    request_sequence,
                    options: options.clone(),
                    editable: props.editable,
                    projection_port: props.pair_projection_port.clone(),
                    on_request: props.on_request,
                }
            }
             }
            }
            if props.layers.iter().any(|layer| layer.is_previous) {
                div { role: "status", Callout { "Showing generated layers from a previous accepted revision. Settings below reflect the current accepted configuration." } }
            }
            if let Some(mismatch) = props.mismatch.as_ref() {
                div { role: "status", Callout { variant: CalloutVariant::Warning, "Mechanical settings are unavailable while {mismatch.board_name} is configured. Use the authored Case panel to return to that board." } }
            }
            if let Some(values) = props.values.as_ref().filter(|_| configuration_matches) {
                if let Some(feedback) = current_feedback {
                    if feedback.state == MechanicalSettingsFeedbackState::Failed && !is_dimension_field(&feedback.field_id)
                        && let Some(message) = feedback.message.as_deref()
                    {
                        Alert { variant: AlertVariant::Destructive, "{message}" }
                        if feedback.field_id == "initialize-closures" {
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                disabled: mechanical_action_pending("initialize-closures") || !props.editable,
                                onclick: {
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::InitializeClosures)
                                },
                                "Retry mounting locations"
                            }
                        }
                    }
                }
                if !props.editable {
                    if let Some(reason) = props.disabled_reason.as_deref() {
                        div { role: "status", Callout { "{reason}" } }
                    }
                }
                p { class: "text-muted-foreground text-sm leading-normal", "Authored Case bodies remain saved while the generated stack is active. Disable the stack to return to authored bodies." }
                ConstructionControls {
                    identity: props.identity.clone(),
                    values: values.clone(),
                    editable: props.editable,
                    request_sequence,
                    on_request: props.on_request,
                }
                if props.construction_is_legacy {
                    MechanicalLegacyConstructionControl {
                        identity: props.identity.clone(),
                        request_sequence,
                        editable: props.editable,
                        on_request: props.on_request,
                    }
                }
                Collapsible { keep_mounted: true, default_open: false,
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Switch fit profiles" }
                    CollapsibleContent { class: "data-[open=true]:overflow-visible data-[open=false]:hidden",
                ProfileGuidance {
                    identity: props.identity.clone(),
                    owner_key: owner_key.clone(),
                    feedback: props.feedback.clone(),
                    profiles: props.profiles.clone(),
                    extract_profile: props.extract_profile.clone(),
                    targets: props.profile_targets.clone(),
                    plate_thickness: values.plate_thickness,
                    plate_to_pcb: values.plate_to_pcb,
                    editable: props.editable,
                    request_sequence,
                    on_request: props.on_request,
                }
                 }
                }
                Collapsible { keep_mounted: true, default_open: false,
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Manufacturing overrides" }
                    CollapsibleContent { class: "data-[open=true]:overflow-visible data-[open=false]:hidden",
                ManufacturingControls {
                    identity: props.identity.clone(),
                    targets: props.process_targets.clone(),
                    stabilizers: props.stabilizer_fits.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    request_sequence,
                    on_request: props.on_request,
                }
                 }
                }
                DimensionControls {
                    identity: props.identity.clone(),
                    values: values.clone(),
                    editable: props.editable,
                    disabled_reason: props.disabled_reason.clone(),
                    feedback: props.feedback.clone(),
                    request_sequence,
                    on_request: props.on_request,
                    owner_key: owner_key.clone(),
                }
                Collapsible { keep_mounted: true, default_open: true,
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Resolved stack" }
                    CollapsibleContent { class: "data-[open=false]:hidden",
                ResolvedStack {
                    layers: props.layers.clone(),
                    selected_layer: props.selected_layer.clone(),
                    on_select_layer: props.on_select_layer,
                }
                 }
                }
                Collapsible { keep_mounted: true, default_open: false,
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Mechanical diagnostics" }
                    CollapsibleContent { class: "data-[open=false]:hidden",
                Diagnostics {
                    findings: props.findings.clone(),
                    on_show_finding: props.on_show_finding,
                }
                 }
                }
                Collapsible { keep_mounted: true, default_open: false,
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", "Battery" }
                    CollapsibleContent { class: "data-[open=false]:hidden",
                BatteryControls {
                    identity: props.identity.clone(),
                    request_sequence,
                    values: values.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                    owner_key: owner_key.clone(),
                }
                 }
                }
                for opening_owner in [format!("{owner_key}:case-openings")] {
                    OpeningControls {
                        key: "{opening_owner}",
                        identity: props.identity.clone(),
                        request_sequence,
                        values: values.clone(),
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        on_request: props.on_request,
                        owner_key: owner_key.clone(),
                    }
                }
                // A keyed dynamic owner boundary retires child field drafts on scope changes.
                for mounting_owner in [format!("{owner_key}:mounting")] {
                    MountingControls {
                        key: "{mounting_owner}",
                    identity: props.identity.clone(),
                    request_sequence,
                    values: values.clone(),
                    suggested_mounts: props.suggested_mounts.clone(),
                    editable: props.editable,
                    feedback: props.feedback.clone(),
                    on_request: props.on_request,
                    }
                }
                if values.mount == MechanicalMount::Gasket && values.internal_gasket {
                    section { aria_label: "Gasket supports",
                        h3 { "Gasket supports" }
                        Button {
                            variant: ButtonVariant::Secondary,
                            button_type: "button",
                            class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                            disabled: mechanical_action_pending("reset-gasket-placement") || !props.editable,
                            onclick: {
                                let identity = props.identity.clone();
                                let mut sequence = request_sequence;
                                let on_request = props.on_request;
                                move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::ResetGasketPlacement)
                            },
                            "Reset gasket placement"
                        }
                        p { class: "text-muted-foreground text-sm leading-normal",
                            "Reset releases manually positioned supports. Closure positions stay fixed."
                        }
                    }
                }
                if values.internal_gasket {
                    GasketClosureControls {
                        key: "{owner_key}:gasket-closure-hardware",
                        identity: props.identity.clone(),
                        request_sequence,
                        hardware: values.closure_hardware.clone(),
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        on_request: props.on_request,
                        owner_key: owner_key.clone(),
                    }
                }
                for fit_owner in [format!("{owner_key}:critical-fits")] {
                    CriticalFitControls {
                        key: "{fit_owner}",
                        identity: props.identity.clone(),
                        request_sequence,
                        fits: values.critical_fits.clone(),
                        hardware: values.hardware.clone(),
                        hardware_mounts: props.hardware_mounts.clone(),
                        fit_parts: props.fit_parts.clone(),
                        fit_parts_resolved: props.fit_parts_resolved,
                        values: values.clone(),
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        on_request: props.on_request,
                        owner_key: owner_key.clone(),
                    }
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !props.editable,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = request_sequence;
                        let on_request = props.on_request;
                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::Disable)
                    },
                    if props.pair_counterpart.is_some() { "Unlink and disable this half" } else { "Disable mechanical stack" }
                }
                if let Some(counterpart) = props.pair_counterpart.clone() {
                    p { class: "text-muted-foreground text-sm leading-normal",
                        "This half is linked with {counterpart}. Disabling removes that link and disables only this half's mechanical stack: the other half keeps its geometry and settings, and neither PCB is removed. One Undo restores the link and this stack."
                    }
                    MechanicalPairMemberRemoval {
                        identity: props.identity.clone(),
                        request_sequence,
                        counterpart,
                        editable: props.editable,
                        on_request: props.on_request,
                    }
                }
            } else if props.values.is_some() {
                div { role: "status", Callout { variant: CalloutVariant::Warning, "The configured stack belongs to another board. Switch to that board to edit it." } }
            } else {
                if let Some(feedback) = current_feedback {
                    if feedback.state == MechanicalSettingsFeedbackState::Pending {
                        p { role: "status", "Saving mechanical settings…" }
                    } else if feedback.state == MechanicalSettingsFeedbackState::Failed {
                        if let Some(message) = feedback.message.as_deref() { Alert { variant: AlertVariant::Destructive, "{message}" } }
                    }
                }
                if let Some(values) = props.preview_values.as_ref() {
                    ConstructionControls { identity: props.identity.clone(), values: values.clone(), editable: false, request_sequence, on_request: props.on_request }
                    p { class: "text-muted-foreground text-sm leading-normal", "Suggested stack from this board and its switch profiles. Configure to save these settings; viewing this preview does not change the project." }
                }
                p { class: "text-muted-foreground text-sm leading-normal", "Configure a mechanical stack from the current board and represented switch fit profiles." }
                if !props.editable {
                    if let Some(reason) = props.disabled_reason.as_deref() { div { role: "status", Callout { "{reason}" } } }
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !props.editable,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = request_sequence;
                        let on_request = props.on_request;
                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::Enable)
                    },
                    "Configure mechanical stack"
                }
            }
        }
    }
}

pub(crate) fn contextual_layer_fields(
    id: &str,
    values: &MechanicalSettingsValues,
) -> Option<Vec<(MechanicalDimension, &'static str, f64)>> {
    Some(match id {
        "plate" => vec![(
            MechanicalDimension::PlateThickness,
            "Plate thickness",
            values.plate_thickness,
        )],
        "pcb" => vec![(
            MechanicalDimension::PcbThickness,
            "PCB thickness",
            values.pcb_thickness,
        )],
        "plate-foam" => vec![(
            MechanicalDimension::PlateFoamThickness,
            "Plate foam",
            values.plate_foam_thickness,
        )],
        "bottom-foam" => vec![(
            MechanicalDimension::BottomFoamThickness,
            "Bottom foam",
            values.bottom_foam_thickness,
        )],
        "bottom" => vec![
            (
                MechanicalDimension::BottomThickness,
                "Bottom thickness",
                values.bottom_thickness,
            ),
            (
                MechanicalDimension::WallThickness,
                "Wall thickness",
                values.wall_thickness,
            ),
            (
                MechanicalDimension::Clearance,
                "Clearance",
                values.clearance,
            ),
        ],
        "retainer" => vec![
            (
                MechanicalDimension::WallThickness,
                "Wall thickness",
                values.wall_thickness,
            ),
            (
                MechanicalDimension::Clearance,
                "Clearance",
                values.clearance,
            ),
        ],
        _ => return None,
    })
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct ContextualLayerInspectorProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) values: MechanicalSettingsValues,
    pub(crate) layer: MechanicalLayerRow,
    pub(crate) editable: bool,
    pub(crate) disabled_reason: Option<String>,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) findings: Rc<[MechanicalFindingRow]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) on_select_layer: EventHandler<String>,
    pub(crate) on_show_finding: EventHandler<String>,
    pub(crate) owner_key: String,
}

#[component]
pub(crate) fn ContextualLayerInspector(props: ContextualLayerInspectorProps) -> Element {
    let title = match props.layer.id.as_str() {
        "bottom" => "Bottom case",
        "retainer" => "Top case",
        _ => props.layer.label.as_str(),
    };
    let fields = contextual_layer_fields(&props.layer.id, &props.values).unwrap_or_default();
    let errors: Rc<[MechanicalFindingRow]> = Rc::from(
        props
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .cloned()
            .collect::<Vec<_>>(),
    );
    rsx! {
        section { class: "grid gap-3 [&_h3]:m-0 [&_p]:m-0 [&_label]:grid-cols-1 [&_small]:text-xs [&_small]:leading-normal [&_small]:text-muted-foreground", aria_label: "Mechanical stack settings",
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                button_type: "button",
                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                onclick: move |_| props.on_select_layer.call(String::new()),
                "Assembly settings"
            }
            h2 { "{title}" }
            if props.layer.is_previous {
                div { role: "status", Callout { "Showing previous generated geometry. Measurements describe the prior accepted revision; settings below reflect the current configuration." } }
            }
            fieldset { class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", disabled: !props.editable,
                for (field, label, value) in fields {
                    DimensionField {
                        key: "{props.owner_key}:context:{props.layer.id}:{field.field_id()}",
                        identity: props.identity.clone(),
                        request_sequence: props.request_sequence,
                        on_request: props.on_request,
                        feedback: props.feedback.clone(),
                        field,
                        label,
                        value,
                        editable: props.editable,
                    }
                }
                if let Some(reason) = props.disabled_reason.as_deref() {
                    div { role: "status", Callout { "{reason}" } }
                }
            }
            if let Some(thickness) = props.layer.resolved_body_thickness {
                p { class: "text-muted-foreground text-sm leading-normal", "Resolved thickness {thickness:.2} mm." }
            }
            if props.values.internal_gasket && matches!(props.layer.id.as_str(), "retainer" | "bottom") {
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    onclick: move |_| props.on_select_layer.call(String::new()),
                    "Edit shared closure hardware"
                }
            }
            if !errors.is_empty() {
                section { class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", aria_label: "Fit issues",
                    h3 { "Fit issues" }
                    ItemGroup {
                        for finding in errors.iter() {
                            { let id = finding.id.clone();
                            let message = finding.message.clone();
                            let on_show_finding = props.on_show_finding;
                            rsx! { Item { key: "{id}",
                                Badge { variant: severity_badge(&finding.severity), "{severity_label(&finding.severity)}" }
                                ItemContent { ItemDescription { "{message}" } }
                                ItemActions {
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        size: ButtonSize::Sm,
                                        button_type: "button",
                                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                        aria_label: "Show finding {id}",
                                        onclick: move |_| on_show_finding.call(id.clone()),
                                        "Show"
                                    }
                                }
                            } }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn gasket_fit_issues(
    findings: Rc<[MechanicalFindingRow]>,
    on_show_finding: EventHandler<String>,
) -> Element {
    let errors: Vec<_> = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .cloned()
        .collect();
    if errors.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", aria_label: "Fit issues",
            h3 { "Fit issues" }
            ItemGroup {
                for finding in errors {
                    { let id = finding.id.clone();
                      let message = finding.message.clone();
                      let severity = finding.severity.clone();
                      rsx! { Item { key: "{id}",
                          Badge { variant: severity_badge(&severity), "{severity_label(&severity)}" }
                          ItemContent { ItemDescription { "{message}" } }
                          ItemActions {
                              Button {
                                  variant: ButtonVariant::Ghost,
                                  size: ButtonSize::Sm,
                                  button_type: "button",
                                  class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                  aria_label: "Show finding {id}",
                                  onclick: move |_| on_show_finding.call(id.clone()),
                                  "Show"
                              }
                          }
                      } }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct ConstructionControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) values: MechanicalSettingsValues,
    pub(crate) editable: bool,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

/// Compact case choices for the guided setup flow; the full editor stays in Case settings.
#[component]
pub fn MechanicalSetupBasics(settings: MechanicalSettingsProps) -> Element {
    let Some(values) = settings
        .values
        .as_ref()
        .or(settings.preview_values.as_ref())
    else {
        return rsx! { p { role: "status", "Case settings are loading." } };
    };
    rsx! {
        section { class: "grid gap-4 border-0 bg-transparent p-0", aria_label: "Basic case setup",
            ChoiceField {
                label: "Mounting style",
                aria_label: "Case mounting style",
                value: mount_id(&values.mount).to_string(),
                disabled: !settings.editable,
                on_value_change: {
                        let identity = settings.identity.clone();
                        let mut sequence = settings.request_sequence;
                        let on_request = settings.on_request;
                        move |next: Option<String>| if let Some(mount) = parse_mount(&next.unwrap_or_default()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMount(mount));
                        }
                    },
                layout: ChoiceFieldLayout::StackedNarrowTouchOptions,
                options: vec![
                    ("tray".into(), "Tray mount".into()),
                    ("rigid".into(), "Rigid mount".into()),
                    ("gasket".into(), "Gasket mount".into()),
                ],
            }
            FieldLabel { class: "min-h-control-touch", html_for: format!("case-setup-{}-battery", dioxus::core::current_scope_id().0),
                Checkbox {
                    id: format!("case-setup-{}-battery", dioxus::core::current_scope_id().0),
                    checked: Some(if values.battery.is_some() { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                    disabled: !settings.editable || values.transport != HardwareTransport::Wireless,
                    aria_label: "Include wireless battery",
                    on_checked_change: {
                        let identity = settings.identity.clone();
                        let mut sequence = settings.request_sequence;
                        let on_request = settings.on_request;
                        move |state: CheckboxState| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetBatteryEnabled(bool::from(state)))
                    },
                }
                span { "Include a battery" }
                small { class: "text-muted-foreground", if values.transport == HardwareTransport::Wireless { "For the wireless split." } else { "Choose wireless transport to add a battery." } }
            }
            ChoiceField {
                label: "Case process",
                aria_label: "Case fabrication process",
                value: method_id(&values.method).to_string(),
                disabled: !settings.editable,
                on_value_change: {
                        let identity = settings.identity.clone();
                        let mut sequence = settings.request_sequence;
                        let on_request = settings.on_request;
                        move |next: Option<String>| if let Some(method) = parse_method(&next.unwrap_or_default()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMethod(method));
                        }
                    },
                layout: ChoiceFieldLayout::StackedNarrowTouchOptions,
                options: vec![
                    ("printed".into(), "3D printed".into()),
                    ("cnc".into(), "CNC machined".into()),
                    ("cut-sheet".into(), "Cut sheet".into()),
                    ("pcb-fr4".into(), "PCB FR-4".into()),
                ],
            }
        }
    }
}

pub(crate) fn process_material_options(
    part_id: &str,
    method: &PlateMethod,
) -> &'static [&'static str] {
    if part_id.ends_with("foam") {
        &["EVA"]
    } else {
        match method {
            PlateMethod::Printed => &["PLA", "ABS"],
            PlateMethod::Cnc => &["Aluminium"],
            PlateMethod::CutSheet => &["Acrylic"],
            PlateMethod::PcbFr4 => &["FR-4"],
        }
    }
}

pub(crate) fn process_thickness(process: &MechanicalPartProcess, default_thickness: f64) -> f64 {
    if process.thickness >= 0.0 {
        process.thickness
    } else {
        default_thickness
    }
}

pub(crate) fn process_constraint_version(process: &MechanicalPartProcess) -> &str {
    if process.constraints_version.is_empty() {
        "2026-09-24"
    } else {
        &process.constraints_version
    }
}

pub(crate) fn stabilizer_kind_id(kind: &MechanicalStabilizerKind) -> &'static str {
    match kind {
        MechanicalStabilizerKind::PcbMount => "pcb-mount",
        MechanicalStabilizerKind::None => "none",
        MechanicalStabilizerKind::PlateMount => "plate-mount",
    }
}

pub(crate) fn parse_stabilizer_kind(value: &str) -> Option<MechanicalStabilizerKind> {
    Some(match value {
        "pcb-mount" => MechanicalStabilizerKind::PcbMount,
        "none" => MechanicalStabilizerKind::None,
        "plate-mount" => MechanicalStabilizerKind::PlateMount,
        _ => return None,
    })
}

#[component]
pub(crate) fn ConstructionControls(props: ConstructionControlsProps) -> Element {
    let values = &props.values;
    rsx! {
        fieldset { class: "grid gap-3 m-0 min-w-0 p-3 rounded-md bg-card border-0", disabled: !props.editable,
            legend { "Construction" }
            ChoiceField {
                label: "Plate method", disabled: !props.editable,
                value: "{method_id(&values.method)}",
                on_value_change: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: Option<String>| if let Some(method) = parse_method(&event.clone().unwrap_or_default()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMethod(method));
                        }
                    },
                layout: ChoiceFieldLayout::StackedNarrow,
                options: vec![
                    ("pcb-fr4".into(), "PCB FR-4".into()),
                    ("printed".into(), "3D printed".into()),
                    ("cnc".into(), "CNC machined".into()),
                    ("cut-sheet".into(), "Cut sheet".into()),
                ],
            }
            ChoiceField {
                label: "Mount style", disabled: !props.editable,
                value: "{mount_id(&values.mount)}",
                on_value_change: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: Option<String>| if let Some(mount) = parse_mount(&event.clone().unwrap_or_default()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMount(mount));
                        }
                    },
                layout: ChoiceFieldLayout::StackedNarrow,
                options: vec![
                    ("tray".into(), "Tray".into()),
                    ("rigid".into(), "Rigid mount".into()),
                    ("gasket".into(), "Gasket mount".into()),
                ],
            }
            ChoiceField {
                label: "Bottom construction",
                disabled: !props.editable || values.mount == MechanicalMount::Gasket,
                value: "{bottom_style_id(&values.bottom_style)}",
                on_value_change: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: Option<String>| if let Some(style) = parse_bottom_style(&event.clone().unwrap_or_default()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetBottomStyle(style));
                        }
                    },
                layout: ChoiceFieldLayout::StackedNarrow,
                options: vec![
                    ("shell".into(), "Tray shell".into()),
                    ("sheet".into(), "Flat sheet".into()),
                ],
            }
            if values.bottom_style == MechanicalBottomStyle::Sheet {
                FieldLabel { class: "min-h-control-touch", html_for: format!("case-construction-{}-middle", dioxus::core::current_scope_id().0),
                    Checkbox {
                        id: format!("case-construction-{}-middle", dioxus::core::current_scope_id().0),
                        aria_label: "Add middle frame", disabled: !props.editable,
                        checked: Some(if values.middle_frame { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                        on_checked_change: {
                            let identity = props.identity.clone();
                            let mut sequence = props.request_sequence;
                            let on_request = props.on_request;
                            move |event: CheckboxState| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMiddleFrame(bool::from(event)))
                        },
                    }
                    span { "Add middle frame" }
                }
            }
            FieldLabel { class: "min-h-control-touch", html_for: format!("case-construction-{}-integrated", dioxus::core::current_scope_id().0),
                Checkbox {
                    id: format!("case-construction-{}-integrated", dioxus::core::current_scope_id().0),
                    aria_label: "Integrate plate frame into case",
                    disabled: !props.editable || values.mount == MechanicalMount::Gasket,
                    checked: Some(if values.integrated_plate_frame { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                    on_checked_change: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: CheckboxState| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetIntegratedPlateFrame(bool::from(event)))
                    },
                }
                span { "Integrate plate frame into case" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct ResolvedStackProps {
    pub(crate) layers: Rc<[MechanicalLayerRow]>,
    pub(crate) selected_layer: String,
    pub(crate) on_select_layer: EventHandler<String>,
}

#[component]
pub(crate) fn ResolvedStack(props: ResolvedStackProps) -> Element {
    let is_previous = props.layers.iter().any(|layer| layer.is_previous);
    let selected = props
        .layers
        .iter()
        .position(|layer| layer.id == props.selected_layer);
    rsx! {
        section { class: "grid gap-3 m-0 min-w-0 p-3 rounded-md bg-card border-0", aria_label: "Resolved mechanical stack",
            h3 { if is_previous { "Previous resolved stack · {props.layers.len()} layers" } else { "Resolved stack · {props.layers.len()} layers" } }
            if props.layers.is_empty() {
                div { role: "status", Callout { "The stack appears after the current revision resolves." } }
            } else {
                ToggleGroup {
                    aria_label: "Resolved mechanical layers", pressed: Some(selected.into_iter().collect()),
                    horizontal: false, class: "w-full flex-col",
                    on_pressed_change: {
                        let layers = props.layers.clone(); let callback = props.on_select_layer;
                        move |indices: std::collections::HashSet<usize>| callback.call(indices.iter().next().and_then(|index| layers.get(*index)).map(|layer| layer.id.clone()).unwrap_or_default())
                    },
                    for (index, row) in props.layers.iter().enumerate() {
                        ToggleItem { key: "{row.id}", index, class: "w-full h-auto flex-wrap justify-start py-2 max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                            aria_label: format!("{}, {:.2} mm, Z {:.2}{}", row.label, row.thickness, row.z, if row.is_previous { ", previous revision" } else { "" }),
                            strong { "{row.label}" }
                            span { "{row.thickness:.2} mm" }
                            small { "Z {row.z:.2}" }
                            if row.is_previous { small { "Previous revision" } }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct DiagnosticsProps {
    pub(crate) findings: Rc<[MechanicalFindingRow]>,
    pub(crate) on_show_finding: EventHandler<String>,
}

#[component]
pub(crate) fn Diagnostics(props: DiagnosticsProps) -> Element {
    rsx! {
        section { class: "m-0 grid gap-2.5 border-0 border-t border-border py-2.5 [&_legend]:p-0 [&_legend]:font-semibold", aria_label: "Mechanical diagnostics",
            h3 { "Mechanical diagnostics · {props.findings.len()}" }
            if props.findings.is_empty() {
                Callout { "No current mechanical findings." }
            }
            ItemGroup {
                for row in props.findings.iter() {
                    { let id = row.id.clone();
                    let callback = props.on_show_finding;
                    rsx! { Item { key: "{id}",
                        Badge { variant: severity_badge(&row.severity), "{severity_label(&row.severity)}" }
                        ItemContent { ItemDescription { "{row.message}" } }
                        ItemActions {
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Sm,
                                button_type: "button",
                                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Show finding {id}",
                                onclick: move |_| callback.call(id.clone()),
                                "Show"
                            }
                        }
                    } }
                    }
                }
            }
        }
    }
}

#[component]
pub(crate) fn StackupPreview(values: MechanicalSettingsValues) -> Element {
    let rows = [
        ("Plate", values.plate_thickness),
        ("Plate foam", values.plate_foam_thickness),
        (
            "Switch clearance",
            (values.plate_to_pcb - values.plate_foam_thickness).max(0.0),
        ),
        ("PCB", values.pcb_thickness),
        ("Bottom foam", values.bottom_foam_thickness),
        ("Bottom", values.bottom_thickness),
    ];
    rsx! { figure { class: "m-0 mb-3 [&_svg]:block [&_svg]:max-h-[180px] [&_svg]:w-full [&_rect]:fill-surface [&_rect]:stroke-primary [&_rect]:[stroke-width:1] [&_text]:fill-foreground [&_text]:font-sans [&_text]:text-[11px] [&_figcaption]:text-xs [&_figcaption]:text-muted-foreground",
        svg { view_box: "0 0 240 156", role: "img", "aria-label": "Mechanical stackup preview",
            for (index, (name, thickness)) in rows.iter().enumerate().filter(|(_, (_, thickness))| *thickness > 0.0) {
                g { "data-layer": *name,
                    rect { x: "8", y: "{8 + index * 24}", width: "64", height: "{(thickness * 4.0).clamp(3.0, 18.0)}", rx: "2" }
                    text { x: "84", y: "{20 + index * 24}", "{name} · {thickness:.2} mm" }
                }
            }
        }
        figcaption { "Stackup preview · layer thicknesses labelled" }
    } }
}
