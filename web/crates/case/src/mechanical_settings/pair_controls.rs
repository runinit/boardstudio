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
pub(crate) struct MechanicalLegacyConstructionControlProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) editable: bool,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

/// Converting one legacy half's construction to its own local values.
///
/// The confirmation states the whole change before anything is submitted: this half's
/// current effective construction (including the overlaid hardware, critical fits and
/// profiles) becomes its own local values, its ownership becomes local, the other legacy
/// halves keep the baseline they had, and one Undo restores the ownership and the overlay.
/// Exactly one instance is named, and Core validates it.
#[component]
pub(crate) fn MechanicalLegacyConstructionControl(
    props: MechanicalLegacyConstructionControlProps,
) -> Element {
    let confirming = use_signal(|| false);
    let instance_id = props.identity.scope.instance_id.clone().unwrap_or_default();
    let patch = MechanicalSettingsPatch::MaterializeLegacyConstruction {
        instance_ids: vec![instance_id.clone()],
    };
    let pending = mechanical_action_pending(&patch.field_id()) || !props.editable;
    let mut confirming_state = confirming;
    rsx! {
        fieldset { class: "grid gap-3 m-0 min-w-0 border-0 p-3 rounded-md bg-card", disabled: !props.editable,
            legend { "Shared construction" }
            if confirming() {
                p { class: "text-muted-foreground text-sm leading-normal",
                    "Makes {instance_id}'s construction its own: the values it shows today (including the overlaid hardware, critical fits and profiles) become this half's local values and its construction becomes local. Other legacy halves keep their previous baseline. One Undo restores the ownership and the overlay."
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Confirm making construction local for {instance_id}",
                    disabled: pending,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        let patch = patch.clone();
                        move |_| {
                            confirming_state.set(false);
                            send_request(&mut sequence, &identity, on_request, patch.clone());
                        }
                    },
                    "Make construction local"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Keep shared construction for {instance_id}",
                    onclick: move |_| confirming_state.set(false),
                    "Keep it shared"
                }
            } else {
                p { class: "text-muted-foreground text-sm leading-normal",
                    "This half still uses the shared construction baseline."
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Make construction local for {instance_id}",
                    disabled: pending,
                    onclick: move |_| confirming_state.set(true),
                    "Make construction local…"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MechanicalPairMemberRemovalProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) counterpart: String,
    pub(crate) editable: bool,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

/// Removing one half of a mechanical pair, behind an explicit confirmation.
///
/// The typed removal is submitted only after the confirmation states the whole result:
/// this assembly leaves the project, the link is cleared, the other half keeps the
/// geometry and settings it shows today as its own local assembly, and the boards, PCBs
/// and parts stay. Core owns the materialization; the panel never writes geometry.
#[component]
pub(crate) fn MechanicalPairMemberRemoval(props: MechanicalPairMemberRemovalProps) -> Element {
    let confirming = use_signal(|| false);
    let instance_id = props.identity.scope.instance_id.clone().unwrap_or_default();
    let pending = mechanical_action_pending("remove-physical-instance") || !props.editable;
    let counterpart = props.counterpart.clone();
    let mut confirming_state = confirming;
    rsx! {
        section { aria_label: "Remove this physical assembly",
            if confirming() {
                p { class: "text-muted-foreground text-sm leading-normal",
                    "Removes {instance_id} from the project and clears the link. {counterpart} keeps the geometry and settings it shows today as its own local assembly. Your boards, PCBs and parts stay. One Undo restores the assembly and the link."
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Confirm removing assembly {instance_id}",
                    disabled: pending,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |_| {
                            confirming_state.set(false);
                            send_request(
                                &mut sequence,
                                &identity,
                                on_request,
                                MechanicalSettingsPatch::RemovePhysicalInstance,
                            );
                        }
                    },
                    "Remove assembly"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Keep assembly {instance_id}",
                    onclick: move |_| confirming_state.set(false),
                    "Keep it"
                }
            } else {
                p { class: "text-muted-foreground text-sm leading-normal",
                    "Removing this assembly clears the link and removes only it; the other half stays as its own local assembly."
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Remove assembly {instance_id}",
                    disabled: pending,
                    onclick: move |_| confirming_state.set(true),
                    "Remove this assembly…"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MechanicalPairLinkControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) options: Rc<[MechanicalPairLinkOption]>,
    pub(crate) editable: bool,
    /// The mount's read-only Core projection of the reviewed choices.
    pub(crate) projection_port: Option<crate::mechanical_pair::MechanicalPairProjectionPort>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

/// The reviewed link controls for a half that belongs to no pair yet.
///
/// The review shows the map the link will use, the construction source and the
/// durable-id feature matches the link would consolidate, so nothing is submitted
/// before the user has seen the complete proposed result. A translated or rotated
/// board frame has no derived map, so its map is entered explicitly.
#[component]
pub(crate) fn MechanicalPairLinkControls(props: MechanicalPairLinkControlsProps) -> Element {
    let selected = use_signal(|| 0_usize);
    let construction_from_other = use_signal(|| false);
    // Per-feature source choices, keyed by the reviewed half's durable feature id so a
    // re-render cannot hand a choice to a different feature.
    let sources = use_signal(std::collections::BTreeMap::<String, String>::new);
    let mapping_x = use_signal(String::new);
    let mapping_y = use_signal(String::new);
    let mapping_rotation = use_signal(String::new);
    let options = props.options.clone();
    let index = selected().min(options.len().saturating_sub(1));
    let option = options[index].clone();
    let number = |text: String| {
        text.trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
    };
    let explicit_mapping = number(mapping_x())
        .zip(number(mapping_y()))
        .map(|(x, y)| Pose2 {
            at: Vec2 { x, y },
            rotation: number(mapping_rotation()).unwrap_or(0.0),
        });
    let mapping = option.derived_mapping.or(explicit_mapping);
    let mapping_note = match (option.derived_mapping, mapping) {
        (Some(derived), _) => format!(
            "Derived from the halves' declared transforms: offset ({:?}, {:?}) · rotation {:?}°",
            derived.at.x, derived.at.y, derived.rotation
        ),
        (None, Some(reviewed)) => format!(
            "Reviewed map: offset ({:?}, {:?}) · rotation {:?}°",
            reviewed.at.x, reviewed.at.y, reviewed.rotation
        ),
        (None, None) => {
            "A translated or rotated board frame needs its map entered before linking.".to_owned()
        }
    };
    let construction_from_other_value = construction_from_other();
    // The complete proposed result is projected for the choices actually reviewed, from
    // Core through the mount's port. The cache key is the accepted snapshot plus every
    // reviewed choice, so a change reprojects and an unrelated render does not.
    let sources_value = sources();
    let projection_key = format!(
        "{}|{index}|{}|{}|{}|{construction_from_other_value}|{}",
        props.identity.snapshot_token.0,
        mapping_x(),
        mapping_y(),
        mapping_rotation(),
        sources_value
            .iter()
            .map(|(feature, source)| format!("{feature}={source}"))
            .collect::<Vec<_>>()
            .join(","),
    );
    let projection_cache = use_hook(|| {
        Rc::new(std::cell::RefCell::new(
            None::<(
                String,
                Option<Result<crate::mechanical_pair::MechanicalLinkPreview, String>>,
            )>,
        ))
    });
    let projection: Option<Result<crate::mechanical_pair::MechanicalLinkPreview, String>> = {
        let cached = projection_cache.borrow();
        match cached.as_ref() {
            Some((key, value)) if *key == projection_key => value.clone(),
            _ => {
                drop(cached);
                let value = props.projection_port.as_ref().map(|port| {
                    match crate::mechanical_pair::reviewed_link_change(
                        &option,
                        mapping,
                        construction_from_other_value,
                        &sources_value,
                    ) {
                        Some(change) => port.0(&change),
                        None => Err(
                            "A translated or rotated board frame needs its map entered before the result is projected."
                                .to_owned(),
                        ),
                    }
                });
                *projection_cache.borrow_mut() = Some((projection_key, value.clone()));
                value
            }
        }
    };
    // The offer's own projection is the fallback for callers without a port.
    let (preview, preview_note) = match projection {
        Some(Ok(preview)) => (Some(preview), None),
        Some(Err(reason)) => (None, Some(reason)),
        None => (option.preview.clone(), option.preview_note.clone()),
    };
    let blocked = mechanical_action_pending(&format!("mechanical-pair:{}:link", option.pair_id))
        || !props.editable
        || mapping.is_none()
        || preview.is_none()
        || option.reviewed_fingerprint.is_none();
    let selected_value = use_memo(move || selected().to_string());
    let construction_value = use_memo(move || {
        (if construction_from_other() {
            "other"
        } else {
            "this"
        })
        .to_string()
    });
    rsx! {
        section { aria_label: "Link a mechanical mirror pair",
            h3 { "Mirror another half" }
            p { class: "text-muted-foreground text-sm leading-normal",
                "Linking keeps shared mounts, closure mounts, openings and gasket supports reflected and linked. Features without a match stay local to their half."
            }
            SettingsChoiceField {
                label: "Link with",
                aria_label: "Link with",
                disabled: !props.editable,
                value: selected_value,
                options: options.iter().enumerate().map(|(candidate, item)| (candidate.to_string(), item.other_name.clone())).collect::<Vec<_>>(),
                on_change: {
                        let mut selected = selected;
                        move |next: Option<String>| {
                            if let Some(next) = next && let Ok(value) = next.parse::<usize>() {
                                selected.set(value);
                            }
                        }
                    },
            }
            SettingsSelect {
                label: "Construction from",
                aria_label: "Pair construction source",
                disabled: !props.editable,
                value: construction_value,
                on_change: {
                        let mut construction_from_other = construction_from_other;
                        move |next: Option<String>| {
                            construction_from_other.set(next.as_deref() == Some("other"));
                        }
                    },
                SelectOption::<String> { index: 0usize, value: "this".to_owned(), text_value: option.instance_id.clone(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{option.instance_id}" }
                    SelectOption::<String> { index: 1usize, value: "other".to_owned(), text_value: option.other_instance_id.clone(), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{option.other_instance_id}" }
            }
            if option.derived_mapping.is_none() {
                div {
                    SettingsNumberField {
                        label: "Map offset X",
                        aria_label: "Pair mapping X",
                        value: mapping_x,
                        oninput: { let mut mapping_x = mapping_x; move |event: FormEvent| mapping_x.set(event.value()) },
                        step: "1",
                    }
                    SettingsNumberField {
                        label: "Map offset Y",
                        aria_label: "Pair mapping Y",
                        value: mapping_y,
                        oninput: { let mut mapping_y = mapping_y; move |event: FormEvent| mapping_y.set(event.value()) },
                        step: "1",
                    }
                    SettingsNumberField {
                        label: "Map rotation",
                        aria_label: "Pair mapping rotation",
                        value: mapping_rotation,
                        oninput: { let mut mapping_rotation = mapping_rotation; move |event: FormEvent| mapping_rotation.set(event.value()) },
                        step: "1",
                    }
                }
            }
            p { class: "text-muted-foreground text-sm leading-normal", "{mapping_note}" }
            if option.matched.is_empty() {
                p { class: "text-muted-foreground text-sm leading-normal", "Matched features: {option.matched_features}" }
            } else {
                p { class: "text-muted-foreground text-sm leading-normal",
                    "Matched features stay linked. Features without a match stay local to their half."
                }
            }
            for matched in option.matched.iter() {
                {
                    let matched = matched.clone();
                    let key = matched.first_feature_id.clone();
                    let chosen = sources()
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| matched.source_instance_id.clone());
                    let first_label = matched
                        .first_geometry
                        .clone()
                        .unwrap_or_else(|| "no geometry".to_owned());
                    let second_label = matched
                        .second_geometry
                        .clone()
                        .unwrap_or_else(|| "no geometry".to_owned());
                    let row_id = format!("pair-source-{key}");
                    rsx! {
                        fieldset { key: "{row_id}",
                            "aria-label": "Source for {key}",
                            legend { "{feature_label(matched.kind)} {key} · {matched.second_feature_id}" }
                            SettingsSelect {
                                label: "Shared geometry from",
                                aria_label: "Source for {key}",
                                disabled: !props.editable,
                                value: chosen.clone(),
                                on_change: {
                                        let mut sources = sources;
                                        let key = key.clone();
                                        move |next: Option<String>| {
                                            if let Some(next) = next {
                                                sources.write().insert(key.clone(), next);
                                            }
                                        }
                                    },
                                SelectOption::<String> { index: 0usize, value: matched.source_instance_id.clone(), text_value: format!("{} · {first_label}", matched.source_instance_id), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                        "{matched.source_instance_id} · {first_label}"
                                    }
                                    SelectOption::<String> { index: 1usize, value: option.other_instance_id.clone(), text_value: format!("{} · {second_label}", option.other_instance_id), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                        "{option.other_instance_id} · {second_label}"
                                    }
                            }
                        }
                    }
                }
            }
            // The complete proposed result on both halves, read from Core for exactly
            // the choices reviewed above.
            if let Some(preview) = preview.clone() {
                div { aria_label: "Proposed result on both halves",
                    for member in preview.members.iter() {
                        div { "data-instance": "{member.instance_id}",
                            h4 { "{member.instance_id}" }
                            if member.features.is_empty() {
                                p { class: "text-muted-foreground text-sm leading-normal", "No mechanical features after linking." }
                            } else {
                                ul {
                                    for feature in member.features.iter() {
                                        li { "{feature}" }
                                    }
                                }
                            }
                            for finding in member.findings.iter() {
                                Alert { variant: AlertVariant::Destructive, "{finding}" }
                            }
                        }
                    }
                }
            } else if let Some(note) = preview_note.clone() {
                p { class: "text-muted-foreground text-sm leading-normal",
                    "The complete proposed result is unavailable until you confirm: {note}"
                }
            }
            Button {
                variant: ButtonVariant::Secondary,
                button_type: "button",
                class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                aria_label: "Link {option.instance_id} with {option.other_instance_id}",
                disabled: blocked,
                onclick: {
                    let mut sequence = props.request_sequence;
                    let identity = props.identity.clone();
                    let callback = props.on_request;
                    let option = option.clone();
                    let reviewed = option.reviewed_fingerprint.clone();
                    move |_| {
                        // Exactly the choices the review projected and showed.
                        let Some(change) = crate::mechanical_pair::reviewed_link_change(
                            &option,
                            mapping,
                            construction_from_other_value,
                            &sources.peek(),
                        ) else {
                            return;
                        };
                        send_request_with_review(
                            &mut sequence,
                            &identity,
                            callback,
                            change,
                            reviewed.clone(),
                        );
                    }
                },
                "Link halves"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MechanicalPairControlsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_sequence: Signal<u64>,
    pub(crate) features: Rc<[MechanicalPairFeatureRow]>,
    pub(crate) editable: bool,
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
}

/// Per-feature controls for the selected half's mechanical mirror pair.
///
/// The state of every row comes from the accepted pair store, so there is no second
/// relationship store to drift. A linked feature can be made local or removed on this
/// half; a local or omitted feature can be rejoined from the other half, with the
/// geometry the rejoin takes shown before the commit.
#[component]
pub(crate) fn MechanicalPairControls(props: MechanicalPairControlsProps) -> Element {
    let features = props.features.clone();
    // Per-feature rejoin source choices, keyed by this half's durable feature id.
    let rejoin_sources = use_signal(std::collections::BTreeMap::<String, String>::new);
    rsx! {
        section { aria_label: "Mirrored pair features",
            h3 { "Mirrored pair" }
            p { class: "text-muted-foreground text-sm leading-normal",
                "Shared mechanical features stay linked to the other half. Make one local, remove it on this half, or rejoin it from the other half."
            }
            for row in features.iter() {
                {
                    let row = row.clone();
                    let key = format!("{}:{}", row.instance_id, row.feature_id);
                    let linked = row.state == MechanicalFeatureState::Linked;
                    let local_patch = MechanicalSettingsPatch::MakeFeatureLocal {
                        feature_id: row.feature_id.clone(),
                        instance_id: row.instance_id.clone(),
                    };
                    let omit_patch = MechanicalSettingsPatch::OmitFeatureMember {
                        feature_id: row.feature_id.clone(),
                        instance_id: row.instance_id.clone(),
                    };
                    // A local half may deliberately rejoin from its own current geometry,
                    // keeping this half's placement and re-deriving the other half; an
                    // omitted half has no geometry of its own to take.
                    let offers_own_source = row.state == MechanicalFeatureState::Local
                        && row.own_geometry.is_some();
                    let chosen_source = rejoin_sources
                        .read()
                        .get(&row.feature_id)
                        .cloned()
                        .unwrap_or_else(|| row.counterpart_instance_id.clone());
                    let source_is_own = chosen_source == row.instance_id;
                    let source_geometry = if source_is_own {
                        row.own_geometry.clone()
                    } else {
                        row.preview.clone()
                    };
                    let rejoin_patch = MechanicalSettingsPatch::RejoinFeatureMember {
                        feature_id: row.feature_id.clone(),
                        instance_id: row.instance_id.clone(),
                        source_instance_id: chosen_source.clone(),
                    };
                    let row_fields = [
                        local_patch.field_id(),
                        omit_patch.field_id(),
                        rejoin_patch.field_id(),
                    ];
                    let failure = props
                        .feedback
                        .iter()
                        .find(|entry| {
                            entry.state == MechanicalSettingsFeedbackState::Failed
                                && row_fields.contains(&entry.field_id)
                        })
                        .and_then(|entry| entry.message.clone());
                    let blocked = |patch: &MechanicalSettingsPatch| {
                        mechanical_action_pending(&patch.field_id()) || !props.editable
                    };
                    let local_blocked = blocked(&local_patch);
                    let omit_blocked = blocked(&omit_patch);
                    let rejoin_blocked = blocked(&rejoin_patch)
                        || source_geometry.is_none()
                        || row.rejoin_fingerprint.is_none();
                    let source_label = if source_is_own {
                        "this half"
                    } else {
                        "the other half"
                    };
                    let rejoin_label = if source_is_own {
                        "Rejoin from this half"
                    } else {
                        "Rejoin from the other half"
                    };
                    let state = match row.state {
                        MechanicalFeatureState::Linked => "linked to the other half",
                        MechanicalFeatureState::Local => "local to this half",
                        MechanicalFeatureState::Omitted => "removed on this half",
                    };
                    rsx! {
                        fieldset { key: "{key}", class: "m-0 border-0 border-b border-border py-2", disabled: !props.editable,
                            "aria-label": "Mirrored {feature_label(row.kind).to_lowercase()}",
                            legend { "{feature_label(row.kind)} · {state}" }
                            if let Some(message) = failure {
                                Alert { variant: AlertVariant::Destructive, "{message}" }
                            }
                            if linked {
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    button_type: "button",
                                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                    aria_label: "Make local {row.feature_id}",
                                    disabled: local_blocked,
                                    onclick: {
                                        let mut sequence = props.request_sequence;
                                        let identity = props.identity.clone();
                                        let callback = props.on_request;
                                        let patch = local_patch.clone();
                                        move |_| send_request(&mut sequence, &identity, callback, patch.clone())
                                    },
                                    "Make local"
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    button_type: "button",
                                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                    aria_label: "Remove on this half {row.feature_id}",
                                    disabled: omit_blocked,
                                    onclick: {
                                        let mut sequence = props.request_sequence;
                                        let identity = props.identity.clone();
                                        let callback = props.on_request;
                                        let patch = omit_patch.clone();
                                        move |_| send_request(&mut sequence, &identity, callback, patch.clone())
                                    },
                                    "Remove on this half"
                                }
                            } else {
                                if offers_own_source {
                                    SettingsSelect {
                                        label: "Rejoin from",
                                        aria_label: "Rejoin source for {row.feature_id}",
                                        disabled: !props.editable,
                                        value: chosen_source.clone(),
                                        on_change: {
                                                let mut rejoin_sources = rejoin_sources;
                                                let feature_id = row.feature_id.clone();
                                                move |next: Option<String>| {
                                                    if let Some(next) = next {
                                                        rejoin_sources
                                                            .write()
                                                            .insert(feature_id.clone(), next);
                                                    }
                                                }
                                            },
                                        SelectOption::<String> { index: 0usize, value: row.instance_id.clone(), text_value: format!("this half · {}", row.own_geometry.clone().unwrap_or_default()), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                                "this half · {row.own_geometry.clone().unwrap_or_default()}"
                                            }
                                            SelectOption::<String> { index: 1usize, value: row.counterpart_instance_id.clone(), text_value: format!("the other half · {}", row.preview.clone().unwrap_or_else(|| "no geometry".to_owned())), class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                                "the other half · {row.preview.clone().unwrap_or_else(|| \"no geometry\".to_owned())}"
                                            }
                                    }
                                }
                                if let Some(geometry) = source_geometry.clone() {
                                    p { class: "text-muted-foreground text-sm leading-normal",
                                        "Rejoin takes {geometry} from {chosen_source}."
                                    }
                                } else {
                                    p { class: "text-muted-foreground text-sm leading-normal",
                                        "The reviewed source half has no geometry for this feature, so it cannot be rejoined yet."
                                    }
                                }
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    button_type: "button",
                                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                    aria_label: "Rejoin {row.feature_id} from {source_label}",
                                    disabled: rejoin_blocked,
                                    onclick: {
                                        let mut sequence = props.request_sequence;
                                        let identity = props.identity.clone();
                                        let callback = props.on_request;
                                        let patch = rejoin_patch.clone();
                                        // The reviewed fingerprint the row was produced with
                                        // travels with the submission, so a relevant change
                                        // between this review and its execution retires it.
                                        let reviewed = row.rejoin_fingerprint.clone();
                                        move |_| {
                                            send_request_with_review(
                                                &mut sequence,
                                                &identity,
                                                callback,
                                                patch.clone(),
                                                reviewed.clone(),
                                            )
                                        }
                                    },
                                    "{rejoin_label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
