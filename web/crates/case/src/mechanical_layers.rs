//! Resolved stack, per-layer measurements and gasket support measurements.
use crate::mechanical_projection::{MechanicalLayerRow, contextual_layer_fields};
use crate::mechanical_settings::{
    Fields, MechanicalDimension, MechanicalGasketSupportRow, MechanicalSettingsPatch, Sender,
    dimension, finding_list,
};
use boardstudio_model::model::{Finding, MechanicalConfiguration};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::Callout,
    toggle_group::{ToggleGroup, ToggleItem},
};
use dioxus::prelude::*;
use std::{collections::HashSet, rc::Rc};

/// The six stack layers drawn to scale, each labelled with its thickness.
#[component]
pub(crate) fn StackupPreview(configuration: Rc<MechanicalConfiguration>) -> Element {
    let c = &configuration;
    let rows = [
        ("Plate", c.plate_thickness),
        ("Plate foam", c.plate_foam_thickness),
        (
            "Switch clearance",
            (c.plate_to_pcb - c.plate_foam_thickness).max(0.0),
        ),
        ("PCB", c.pcb_thickness),
        ("Bottom foam", c.bottom_foam_thickness),
        ("Bottom", c.bottom_thickness),
    ];
    rsx! {
        figure { class: "m-0 mb-3 [&_svg]:block [&_svg]:max-h-[180px] [&_svg]:w-full [&_rect]:fill-surface [&_rect]:stroke-primary [&_text]:fill-foreground [&_text]:text-[11px]",
            svg { view_box: "0 0 240 156", role: "img", "aria-label": "Mechanical stackup preview",
                for (index, (name, thickness)) in rows.iter().enumerate().filter(|(_, (_, thickness))| *thickness > 0.0) {
                    g { "data-layer": *name,
                        rect { x: "8", y: "{8 + index * 24}", width: "64", height: "{(thickness * 4.0).clamp(3.0, 18.0)}", rx: "2" }
                        text { x: "84", y: "{20 + index * 24}", "{name} · {thickness:.2} mm" }
                    }
                }
            }
            figcaption { "Stackup preview · layer thicknesses labelled" }
        }
    }
}

#[component]
pub(crate) fn ResolvedStack(
    layers: Rc<[MechanicalLayerRow]>,
    selected: String,
    on_select_layer: EventHandler<String>,
) -> Element {
    if layers.is_empty() {
        return rsx! { div { role: "status", Callout { "The stack appears after the current revision resolves." } } };
    }
    rsx! {
        ToggleGroup {
            aria_label: "Resolved mechanical layers", horizontal: false, class: "w-full flex-col",
            pressed: Some(layers.iter().position(|layer| layer.id == selected).into_iter().collect::<HashSet<_>>()),
            on_pressed_change: {
                let layers = layers.clone();
                move |next: HashSet<usize>| on_select_layer.call(next.iter().next().and_then(|index| layers.get(*index)).map(|layer| layer.id.clone()).unwrap_or_default())
            },
            for (index, row) in layers.iter().enumerate() {
                ToggleItem { key: "{row.id}", index, class: "w-full h-auto flex-wrap justify-start py-2",
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

#[component]
pub(crate) fn ContextualLayerInspector(
    configuration: Rc<MechanicalConfiguration>,
    layer: MechanicalLayerRow,
    editable: bool,
    banner: Element,
    findings: Rc<[Finding]>,
    send: Sender,
    on_select_layer: EventHandler<String>,
    on_show_finding: EventHandler<String>,
) -> Element {
    let title = match layer.id.as_str() {
        "bottom" => "Bottom case",
        "retainer" => "Top case",
        _ => layer.label.as_str(),
    };
    rsx! {
        section { class: "grid gap-3 [&_h3]:m-0 [&_p]:m-0", aria_label: "Mechanical stack settings",
            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| on_select_layer.call(String::new()), "Assembly settings" }
            h2 { "{title}" }
            if layer.is_previous { div { role: "status", Callout { "Showing previous generated geometry. Measurements describe the prior accepted revision; settings below reflect the current configuration." } } }
            {banner}
            Fields { editable,
                for (field, label, value) in contextual_layer_fields(&layer.id, &configuration).unwrap_or_default() {
                    {dimension(label, field, value, move |value| send.send(MechanicalSettingsPatch::SetDimension { field, value }))}
                }
            }
            if let Some(thickness) = layer.resolved_body_thickness { p { class: "text-muted-foreground text-sm", "Resolved thickness {thickness:.2} mm." } }
            if configuration.internal_gasket.is_some() && matches!(layer.id.as_str(), "retainer" | "bottom") {
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| on_select_layer.call(String::new()), "Edit shared closure hardware" }
            }
            FitIssues { findings, on_show_finding }
        }
    }
}

#[component]
fn FitIssues(findings: Rc<[Finding]>, on_show_finding: EventHandler<String>) -> Element {
    if !findings
        .iter()
        .any(|finding| finding.severity == boardstudio_model::model::Severity::Error)
    {
        return rsx! {};
    }
    rsx! { section { class: "grid gap-2 border-t border-border py-2.5", aria_label: "Fit issues", h3 { "Fit issues" } {finding_list(&findings, on_show_finding, true)} } }
}

#[component]
pub(crate) fn GasketSupportInspector(
    support: MechanicalGasketSupportRow,
    supports: Rc<[MechanicalGasketSupportRow]>,
    editable: bool,
    banner: Element,
    findings: Rc<[Finding]>,
    send: Sender,
    on_select_layer: EventHandler<String>,
    on_show_finding: EventHandler<String>,
) -> Element {
    // A linked support resizes together with its mirrored pair.
    let anchors: Vec<_> = supports
        .iter()
        .filter(|candidate| {
            candidate.id == support.id
                || (!support.unlinked
                    && support.pair_id.as_deref() == Some(candidate.id.as_str())
                    && !candidate.unlinked)
        })
        .map(MechanicalGasketSupportRow::saved_anchor)
        .collect();
    let index = supports
        .iter()
        .position(|candidate| candidate.id == support.id)
        .unwrap_or_default()
        + 1;
    rsx! {
        section { class: "grid gap-3 [&_h3]:m-0 [&_p]:m-0", aria_label: "Mechanical stack settings",
            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| on_select_layer.call(String::new()), "Assembly settings" }
            h2 { "Gasket {index}" }
            if support.is_previous {
                div { role: "status", Callout { "Showing previous generated geometry. Gasket measurements cannot be edited until the current assembly resolves." } }
            } else {
                p { class: "text-muted-foreground text-sm", if support.unlinked { "This gasket is unlinked from its pair." } else { "This support is linked to its mirrored pair. Matching upper and lower pads resize together." } }
                if let Some(error) = support.fit_error.as_deref() {
                    Alert { variant: AlertVariant::Destructive, "{error} Preview and export stay blocked until it fits." }
                } else {
                    p { class: "text-muted-foreground text-sm", "Fits at this position." }
                }
                {banner}
                Fields { editable,
                    for (label, field, value) in [("Cut length", MechanicalDimension::GasketSupportLength, support.length), ("Pad width", MechanicalDimension::GasketSupportWidth, support.width)] {
                        {dimension(label, field, value, { let (id, anchors) = (support.id.clone(), anchors.clone()); move |value| send.send(MechanicalSettingsPatch::SetGasketSupportDimension { support_id: id.clone(), anchors: anchors.clone(), field, value }) })}
                    }
                }
                p { class: "text-muted-foreground text-sm", "Prefer 10 mm cuts; use 5 mm increments for a tighter fit." }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| on_select_layer.call("gaskets".to_owned()), "All gasket settings" }
                FitIssues { findings, on_show_finding }
            }
        }
    }
}
