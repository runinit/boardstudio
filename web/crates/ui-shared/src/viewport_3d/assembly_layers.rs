//! Private Case assembly and physical-model layer controls.
//!
//! The caller supplies the currently accepted projection. This module owns
//! only viewer display preferences; it does not request previews or models.

use super::assembly_projection::case_display_selection_id;
pub use super::assembly_projection::{
    CaseAssemblyLayer, CaseComponentLayer, LayerAvailability, assembly_layers_with_stack,
    physical_component_layers, standard_assembly_layers,
};
use crate::case_display::CaseDisplay;
use crate::ui::{
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    icons::ChevronDown,
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use std::collections::HashSet;
use wasm_bindgen::JsCast;

#[component]
pub fn CaseAssemblyLayers(
    assembly: Vec<CaseAssemblyLayer>,
    components: Vec<CaseComponentLayer>,
    selectable_layers: Vec<String>,
    selected_layer: String,
    on_select_layer: EventHandler<String>,
    display: CaseDisplay,
    on_display_change: EventHandler<CaseDisplay>,
) -> Element {
    let mut open = use_signal(|| false);
    let display_for_toggle = display.clone();
    let toggle = move |id: String| {
        let mut next = display_for_toggle.clone();
        if next.hidden.iter().any(|hidden| hidden == &id) {
            next.hidden.retain(|hidden| hidden != &id);
        } else {
            next.hidden.push(id);
        }
        on_display_change.call(next);
    };
    let keydown = move |event: KeyboardEvent| {
        if event.data().key().to_string() == "Escape" && open() {
            event.prevent_default();
            open.set(false);
            if let Some(trigger) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("m1-case-assembly-layers-trigger"))
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = trigger.focus();
            }
        }
    };
    rsx! {
        section {
            class: "absolute right-2.5 bottom-2.5 z-[6] w-[min(360px,calc(100%-20px))] text-foreground max-[640px]:right-2 max-[640px]:bottom-2 max-[640px]:w-[min(340px,calc(100%-16px))] [[id=m1-objects-panel]_&]:static [[id=m1-objects-panel]_&]:w-full",
            "aria-label": "Assembly visibility",
            "data-open": "{open()}",
            onkeydown: keydown,
            Collapsible { keep_mounted: true, open: Some(open()), on_open_change: move |next| open.set(next),
            CollapsibleTrigger {
                id: "m1-case-assembly-layers-trigger",
                aria_label: "Assembly visibility",
                class: "min-h-11 w-full justify-between px-3 py-2 text-sm font-medium",
                span { "Assembly visibility" }
                ChevronDown { class: if open() { "size-4 shrink-0 transition-transform rotate-180" } else { "size-4 shrink-0 transition-transform" } }
            }
            CollapsibleContent {
                id: "m1-case-assembly-layers-list",
                class: "mb-2 grid max-h-[min(55vh,440px)] gap-2.5 overflow-auto rounded-[10px] border border-border-strong bg-card p-3 shadow-[0_10px_30px_#0005] [&[hidden]]:hidden max-[640px]:max-h-[52vh] [[id=m1-objects-panel]_&]:absolute [[id=m1-objects-panel]_&]:bottom-full [[id=m1-objects-panel]_&]:left-0 [[id=m1-objects-panel]_&]:z-30 [[id=m1-objects-panel]_&]:m-0 [[id=m1-objects-panel]_&]:box-border [[id=m1-objects-panel]_&]:w-full [[id=m1-objects-panel]_&]:max-h-[min(65vh,520px)] [[id=m1-objects-panel]_&]:rounded-b-none [[id=m1-objects-panel]_&]:rounded-t-xl",
                hidden: !open(),
                inert: (!open()).then_some(""),
                if !assembly.is_empty() {
                    div { class: "grid gap-1", role: "group", "aria-label": "Assembly",
                        div { class: "px-1 pt-0.5 pb-[5px] text-[0.78rem] font-bold tracking-[0.04em] text-muted-foreground uppercase", "Assembly" }
                        for layer in assembly {
                            {
                                let toggle = toggle.clone();
                                let selectable_layers = selectable_layers.clone();
                                let on_select_layer = on_select_layer;
                                let selected_layer = selected_layer.clone();
                                rsx! { CaseLayerButton {
                                    layer,
                                    selectable_layers,
                                    selected_layer,
                                    on_select_layer,
                                    display: display.clone(),
                                    on_toggle: toggle,
                                } }
                            }
                        }
                    }
                }
                if !components.is_empty() {
                    div { class: "grid gap-1", role: "group", "aria-label": "Components",
                        div { class: "px-1 pt-0.5 pb-[5px] text-[0.78rem] font-bold tracking-[0.04em] text-muted-foreground uppercase", "Components" }
                        for layer in components {
                            { let toggle = toggle.clone(); rsx! { ComponentLayerButton { layer, display: display.clone(), on_toggle: toggle } } }
                        }
                    }
                }
            }
            }
        }
    }
}

#[component]
fn CaseLayerButton(
    layer: CaseAssemblyLayer,
    selectable_layers: Vec<String>,
    selected_layer: String,
    on_select_layer: EventHandler<String>,
    display: CaseDisplay,
    on_toggle: EventHandler<String>,
) -> Element {
    let available = layer.availability == LayerAvailability::Available;
    let visible = available && !display.hidden.iter().any(|id| id == &layer.id);
    let unavailable = match &layer.availability {
        LayerAvailability::Available => None,
        LayerAvailability::Pending => Some("Not ready"),
        LayerAvailability::Unavailable(reason) => Some(reason.as_str()),
    };
    let id = layer.id.clone();
    let selection_id = case_display_selection_id(&layer.id, &selectable_layers);
    let selectable = selection_id.is_some();
    let selected = selection_id.as_deref() == Some(selected_layer.as_str());
    let selected_id = selection_id.clone();
    let selection_label = layer.label.clone();
    let selection_aria_label = if selectable {
        format!("Select {selection_label} for appearance")
    } else {
        selection_label.clone()
    };
    let visibility_label = if visible {
        format!("Hide {}", layer.label)
    } else {
        format!("Show {}", layer.label)
    };
    let row_disabled = !selectable || unavailable.is_some();
    let visibility_disabled = unavailable.is_some();
    rsx! {
        div {
            class: "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-1",
            ToggleGroup {
                aria_label: selection_label.clone(),
                disabled: row_disabled,
                pressed: Some(if selected { HashSet::from([0usize]) } else { HashSet::new() }),
                on_pressed_change: move |next: HashSet<usize>| {
                    if next.contains(&0) != selected
                        && let Some(id) = selected_id.clone()
                    {
                        on_select_layer.call(id);
                    }
                },
                style: "width: 100%",
                ToggleItem {
                    index: 0usize,
                    aria_label: selection_aria_label,
                    disabled: row_disabled,
                    class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                    style: "width: 100%; justify-content: flex-start;",
                    span { class: "m-auto size-2.5 rounded-[2px] border border-current bg-primary data-[layer=PCB]:bg-[#51b98b] data-[layer=Copper]:bg-[#ce8a49]", "data-layer": "{layer.label}" }
                    span { class: "flex-1 truncate text-left", "{layer.label}" }
                    if let Some(state) = unavailable { small { class: "whitespace-nowrap text-xs text-muted-foreground", "{state}" } }
                }
            }
            ToggleGroup {
                aria_label: format!("{} visibility", layer.label),
                disabled: visibility_disabled,
                pressed: Some(if visible { HashSet::from([0usize]) } else { HashSet::new() }),
                on_pressed_change: move |next: HashSet<usize>| {
                    if next.contains(&0) != visible { on_toggle.call(id.clone()); }
                },
                Tooltip {
                    TooltipTrigger { r#as: {
                        let visibility_label = visibility_label.clone();
                        move |attributes: Vec<Attribute>| rsx! {
                            ToggleItem { index: 0usize, aria_label: visibility_label.clone(),
                                disabled: visibility_disabled,
                                class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11",
                                attributes,
                                LayerEye { visible }
                            }
                        }
                    } }
                    TooltipContent { "{visibility_label}" }
                }
            }
        }
    }
}

#[component]
fn ComponentLayerButton(
    layer: CaseComponentLayer,
    display: CaseDisplay,
    on_toggle: EventHandler<String>,
) -> Element {
    let available = layer.availability == LayerAvailability::Available;
    let visible = available && !display.hidden.iter().any(|id| id == &layer.id);
    let unavailable = match &layer.availability {
        LayerAvailability::Available => None,
        LayerAvailability::Pending => Some("Loading model…"),
        LayerAvailability::Unavailable(_) => Some("Missing model"),
    };
    let id = layer.id.clone();
    let label = layer.label;
    let toggle_label = if visible {
        format!("Hide {label}")
    } else {
        format!("Show {label}")
    };
    let row_disabled = unavailable.is_some();
    rsx! {
        ToggleGroup {
            aria_label: format!("{label} visibility"),
            disabled: row_disabled,
            pressed: Some(if visible { HashSet::from([0usize]) } else { HashSet::new() }),
            on_pressed_change: move |next: HashSet<usize>| {
                if next.contains(&0) != visible { on_toggle.call(id.clone()); }
            },
            style: "width: 100%",
            ToggleItem {
                index: 0usize,
                aria_label: toggle_label,
                disabled: row_disabled,
                class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                style: "width: 100%; justify-content: flex-start;",
                span { class: "m-auto size-2.5 rounded-full border border-current bg-[#879acb]", "data-layer": "Components" }
                span { class: "flex-1 truncate text-left", "{label}" }
                if let Some(state) = unavailable { small { class: "whitespace-nowrap text-xs text-muted-foreground", "{state}" } }
                LayerEye { visible }
            }
        }
    }
}

#[component]
fn LayerEye(visible: bool) -> Element {
    rsx! { svg { view_box: "0 0 20 20", "aria-hidden": "true",
        path { d: "M2 10q8-12 16 0-8 12-16 0Z" }
        circle { cx: "10", cy: "10", r: "2.5" }
        if !visible { path { d: "m3 17 14-14" } }
    } }
}
