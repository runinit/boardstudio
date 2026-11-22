//! Mirrored-pair actions: reviewed link, per-feature make-local / remove / rejoin, legacy
//! construction conversion and member removal. Reviews show the complete result first.
use crate::mechanical_pair::{
    MechanicalPairFeatureRow, MechanicalPairLinkOption, MechanicalPairProjectionPort,
    feature_label, reviewed_link_change,
};
use crate::mechanical_settings::{Fields, MechanicalSettingsPatch, Sender};
use boardstudio_model::model::{MechanicalFeatureState, Pose2, Vec2};
use boardstudio_web_ui_shared::inspector_fields::{NumericField, SelectField};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonSize, ButtonVariant},
};
use dioxus::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

/// A destructive action behind an explicit confirmation that states the whole result.
#[component]
fn Confirmed(
    summary: String,
    prompt: String,
    confirm: String,
    keep: String,
    editable: bool,
    on_confirm: EventHandler<()>,
) -> Element {
    let mut confirming = use_signal(|| false);
    rsx! {
        if confirming() {
            p { class: "text-muted-foreground text-sm", "{summary}" }
            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "{confirm}", disabled: !editable,
                onclick: move |_| { confirming.set(false); on_confirm.call(()) }, "{confirm}" }
            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "{keep}", onclick: move |_| confirming.set(false), "{keep}" }
        } else {
            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "{prompt}", disabled: !editable, onclick: move |_| confirming.set(true), "{prompt}…" }
        }
    }
}

#[component]
pub(crate) fn LegacyConstructionControl(editable: bool, send: Sender) -> Element {
    let id = send.instance_id();
    rsx! {
        section { aria_label: "Shared construction", class: "grid gap-2",
            p { class: "text-muted-foreground text-sm", "This half still uses the shared construction baseline." }
            Confirmed { editable, prompt: format!("Make construction local for {id}"), confirm: format!("Confirm making construction local for {id}"), keep: format!("Keep shared construction for {id}"),
                summary: format!("Makes {id}'s construction its own: the values it shows today (including overlaid hardware, critical fits and profiles) become this half's local values. One Undo restores the ownership and the overlay."),
                on_confirm: { let id = id.clone(); move |_| send.send(MechanicalSettingsPatch::MaterializeLegacyConstruction { instance_ids: vec![id.clone()] }) } }
        }
    }
}

#[component]
pub(crate) fn PairMemberRemoval(counterpart: String, editable: bool, send: Sender) -> Element {
    let id = send.instance_id();
    rsx! {
        section { aria_label: "Remove this physical assembly",
            Confirmed { editable, prompt: format!("Remove assembly {id}"), confirm: format!("Confirm removing assembly {id}"), keep: format!("Keep assembly {id}"),
                summary: format!("Removes {id} from the project and clears the link. {counterpart} keeps the geometry and settings it shows today as its own local assembly. Your boards, PCBs and parts stay. One Undo restores the assembly and the link."),
                on_confirm: move |_| send.send(MechanicalSettingsPatch::RemovePhysicalInstance) }
        }
    }
}

/// Per-feature controls for the selected half's pair. States come from the accepted pair store.
#[component]
pub(crate) fn PairRowControls(
    features: Rc<[MechanicalPairFeatureRow]>,
    editable: bool,
    send: Sender,
) -> Element {
    let mut sources = use_signal(BTreeMap::<String, String>::new);
    rsx! {
        section { aria_label: "Mirrored pair features", class: "grid gap-2",
            p { class: "text-muted-foreground text-sm", "Shared mechanical features stay linked to the other half. Make one local, remove it on this half, or rejoin it from the other half." }
            for row in features.iter().cloned() {
                {
                    let state = match row.state {
                        MechanicalFeatureState::Linked => "linked to the other half",
                        MechanicalFeatureState::Local => "local to this half",
                        MechanicalFeatureState::Omitted => "removed on this half",
                    };
                    // A local half may rejoin from its own geometry, keeping its placement; an omitted half has none.
                    let own_source = row.state == MechanicalFeatureState::Local && row.own_geometry.is_some();
                    let chosen = sources.read().get(&row.feature_id).cloned().unwrap_or_else(|| row.counterpart_instance_id.clone());
                    let from_own = chosen == row.instance_id;
                    let source_label = if from_own { "this half" } else { "the other half" };
                    let geometry = if from_own { row.own_geometry.clone() } else { row.preview.clone() };
                    let (feature_id, instance_id) = (row.feature_id.clone(), row.instance_id.clone());
                    rsx! {
                        fieldset { key: "{row.instance_id}:{row.feature_id}", class: "m-0 grid gap-1.5 border-0 border-b border-border py-2", disabled: !editable,
                            aria_label: "Mirrored {feature_label(row.kind).to_lowercase()}",
                            legend { "{feature_label(row.kind)} · {state}" }
                            if row.state == MechanicalFeatureState::Linked {
                                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Make local {row.feature_id}",
                                    onclick: { let (f, i) = (feature_id.clone(), instance_id.clone()); move |_| send.send(MechanicalSettingsPatch::MakeFeatureLocal { feature_id: f.clone(), instance_id: i.clone() }) }, "Make local" }
                                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove on this half {row.feature_id}",
                                    onclick: { let (f, i) = (feature_id.clone(), instance_id.clone()); move |_| send.send(MechanicalSettingsPatch::OmitFeatureMember { feature_id: f.clone(), instance_id: i.clone() }) }, "Remove on this half" }
                            } else {
                                if own_source {
                                    SelectField::<String> { label: "Rejoin from", aria_label: "Rejoin source for {row.feature_id}", value: chosen.clone(),
                                        options: vec![
                                            (row.instance_id.clone(), format!("this half · {}", row.own_geometry.clone().unwrap_or_default())),
                                            (row.counterpart_instance_id.clone(), format!("the other half · {}", row.preview.clone().unwrap_or_else(|| "no geometry".into()))),
                                        ],
                                        on_commit: { let f = feature_id.clone(); move |next: String| { sources.write().insert(f.clone(), next); } } }
                                }
                                if let Some(geometry) = geometry.clone() {
                                    p { class: "text-muted-foreground text-sm", "Rejoin takes {geometry} from {chosen}." }
                                } else {
                                    p { class: "text-muted-foreground text-sm", "The reviewed source half has no geometry for this feature, so it cannot be rejoined yet." }
                                }
                                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: geometry.is_none() || row.rejoin_fingerprint.is_none(),
                                    aria_label: "Rejoin {row.feature_id} from {source_label}",
                                    onclick: { let (f, i, source, fingerprint) = (feature_id.clone(), instance_id.clone(), chosen.clone(), row.rejoin_fingerprint.clone()); move |_| send.send_reviewed(
                                        MechanicalSettingsPatch::RejoinFeatureMember { feature_id: f.clone(), instance_id: i.clone(), source_instance_id: source.clone() }, fingerprint.clone()) },
                                    if from_own { "Rejoin from this half" } else { "Rejoin from the other half" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The reviewed link for a half that belongs to no pair yet: map, sources, then the complete result.
#[component]
pub(crate) fn PairLinkControls(
    options: Rc<[MechanicalPairLinkOption]>,
    editable: bool,
    port: Option<MechanicalPairProjectionPort>,
    snapshot: u64,
    send: Sender,
) -> Element {
    let mut selected = use_signal(|| 0_usize);
    let mut from_other = use_signal(|| false);
    let mut sources = use_signal(BTreeMap::<String, String>::new);
    let mut mapping = use_signal(|| None::<(f64, f64, f64)>);
    let option = options[selected().min(options.len() - 1)].clone();
    let entered = mapping().map(|(x, y, rotation)| Pose2 {
        at: Vec2 { x, y },
        rotation,
    });
    let resolved = option.derived_mapping.or(entered);
    let change = reviewed_link_change(&option, resolved, from_other(), &sources());
    // The complete result is projected by Core for exactly the reviewed choices.
    let projection = use_memo(use_reactive((&change, &snapshot), {
        let port = port.clone();
        move |(change, _)| {
            port.as_ref()
                .zip(change)
                .map(|(port, change)| (port.0)(&change))
        }
    }))();
    let (preview, note) = match projection {
        Some(Ok(preview)) => (Some(preview), None),
        Some(Err(reason)) => (None, Some(reason)),
        None => (option.preview.clone(), option.preview_note.clone()),
    };
    let (x, y, rotation) = mapping().unwrap_or((0.0, 0.0, 0.0));
    rsx! {
        section { aria_label: "Link a mechanical mirror pair", class: "grid gap-2",
            h3 { "Mirror another half" }
            p { class: "text-muted-foreground text-sm", "Linking keeps shared mounts, closure mounts, openings and gasket supports reflected and linked. Features without a match stay local to their half." }
            Fields { editable,
                SelectField::<usize> { label: "Link with", value: selected(), options: options.iter().enumerate().map(|(index, item)| (index, item.other_name.clone())).collect::<Vec<_>>(),
                    on_commit: move |index| selected.set(index) }
                SelectField::<bool> { label: "Construction from", aria_label: "Pair construction source", value: from_other(),
                    options: vec![(false, option.instance_id.clone()), (true, option.other_instance_id.clone())], on_commit: move |v| from_other.set(v) }
                if option.derived_mapping.is_none() {
                    for (label, axis, value) in [("Map offset X", 0, x), ("Map offset Y", 1, y), ("Map rotation", 2, rotation)] {
                        NumericField { key: "{label}", label, aria_label: "Pair mapping {label}", value, step: 1.0,
                            on_commit: move |next: f64| { let (mut a, mut b, mut c) = mapping().unwrap_or((0.0, 0.0, 0.0)); match axis { 0 => a = next, 1 => b = next, _ => c = next } mapping.set(Some((a, b, c))) } }
                    }
                }
                p { class: "text-muted-foreground text-sm",
                    if let Some(derived) = option.derived_mapping { "Derived from the halves' declared transforms: offset ({derived.at.x:?}, {derived.at.y:?}) · rotation {derived.rotation:?}°" }
                    else if let Some(reviewed) = entered { "Reviewed map: offset ({reviewed.at.x:?}, {reviewed.at.y:?}) · rotation {reviewed.rotation:?}°" }
                    else { "A translated or rotated board frame needs its map entered before linking." }
                }
                if option.matched.is_empty() {
                    p { class: "text-muted-foreground text-sm", "Matched features: {option.matched_features}" }
                }
                for matched in option.matched.iter().cloned() {
                    {
                        let key = matched.first_feature_id.clone();
                        let source_key = key.clone();
                        let chosen = sources.read().get(&key).cloned().unwrap_or_else(|| matched.source_instance_id.clone());
                        rsx! {
                            fieldset { key: "{key}", aria_label: "Source for {key}",
                                legend { "{feature_label(matched.kind)} {key} · {matched.second_feature_id}" }
                                SelectField::<String> { label: "Shared geometry from", aria_label: "Source for {key}", value: chosen,
                                    options: vec![
                                        (matched.source_instance_id.clone(), format!("{} · {}", matched.source_instance_id, matched.first_geometry.clone().unwrap_or_else(|| "no geometry".into()))),
                                        (option.other_instance_id.clone(), format!("{} · {}", option.other_instance_id, matched.second_geometry.clone().unwrap_or_else(|| "no geometry".into()))),
                                    ],
                                    on_commit: move |next: String| { sources.write().insert(source_key.clone(), next); } }
                            }
                        }
                    }
                }
            }
            if let Some(preview) = preview {
                div { aria_label: "Proposed result on both halves",
                    for member in preview.members.iter() {
                        div { key: "{member.instance_id}", "data-instance": "{member.instance_id}",
                            h4 { "{member.instance_id}" }
                            if member.features.is_empty() { p { class: "text-muted-foreground text-sm", "No mechanical features after linking." } }
                            ul { for feature in member.features.iter() { li { key: "{feature}", "{feature}" } } }
                            for finding in member.findings.iter() { Alert { key: "{finding}", variant: AlertVariant::Destructive, "{finding}" } }
                        }
                    }
                }
            } else if let Some(note) = note {
                p { class: "text-muted-foreground text-sm", "The complete proposed result is unavailable until you confirm: {note}" }
            }
            Button { variant: ButtonVariant::Secondary, aria_label: "Link {option.instance_id} with {option.other_instance_id}",
                disabled: !editable || change.is_none() || option.reviewed_fingerprint.is_none(),
                onclick: { let (change, fingerprint) = (change.clone(), option.reviewed_fingerprint.clone()); move |_| if let Some(change) = change.clone() { send.send_reviewed(change, fingerprint.clone()) } },
                "Link halves" }
        }
    }
}
