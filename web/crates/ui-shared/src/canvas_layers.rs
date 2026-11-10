//! Shared layer control presentation backed by the editor-owned visibility state.
use super::LayerVisibility;
use crate::ui::{
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    icons::ChevronDown,
    item::{Item, ItemSize},
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerTarget {
    Hidden(String),
    ModuleHidden(String),
    LayoutFootprints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasLayer {
    pub id: String,
    pub label: String,
    pub target: LayerTarget,
}

impl CanvasLayer {
    pub fn hidden(id: impl Into<String>, label: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: label.into(),
            target: LayerTarget::Hidden(id.clone()),
            id,
        }
    }

    pub fn layout_footprints() -> Self {
        Self {
            id: "Footprints".into(),
            label: "Footprints".into(),
            target: LayerTarget::LayoutFootprints,
        }
    }

    pub fn module_hidden(id: impl Into<String>, label: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: label.into(),
            target: LayerTarget::ModuleHidden(id.clone()),
            id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasLayerGroup {
    pub title: Option<String>,
    pub layers: Vec<CanvasLayer>,
}

impl CanvasLayerGroup {
    pub fn ungrouped(layers: Vec<CanvasLayer>) -> Self {
        Self {
            title: None,
            layers,
        }
    }

    pub fn titled(title: impl Into<String>, layers: Vec<CanvasLayer>) -> Self {
        Self {
            title: Some(title.into()),
            layers,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CanvasLayersProps {
    #[props(default)]
    pub initially_open: bool,
    pub trigger_label: String,
    pub trigger_id: String,
    pub list_id: String,
    pub groups: Vec<CanvasLayerGroup>,
}

/// One layer row. The owner decides what a toggle changes.
#[component]
pub fn LayerToggleButton(
    label: String,
    swatch: String,
    aria_label: String,
    visible: bool,
    on_toggle: EventHandler<()>,
) -> Element {
    rsx! {
        Item { size: ItemSize::Xs, class: "flex-nowrap gap-2 px-1 py-0",
            span { class: if visible { "size-2.5 shrink-0 rounded-[2px] bg-geometry-outline data-[layer=Keys]:bg-primary data-[layer=Components]:bg-warning-dark data-[layer=Keycaps]:bg-foreground data-[layer=Footprints]:bg-geometry-part data-[layer=Board]:bg-geometry-board data-[layer='Edge.Cuts']:bg-geometry-board data-[layer='F.Cu']:bg-primary data-[layer='B.Cu']:bg-warning-dark data-[layer=Courtyards]:bg-geometry-part data-[layer=References]:bg-geometry-text" } else { "size-2.5 shrink-0 rounded-[2px] bg-geometry-outline opacity-35 data-[layer=Keys]:bg-primary data-[layer=Components]:bg-warning-dark data-[layer=Keycaps]:bg-foreground data-[layer=Footprints]:bg-geometry-part data-[layer=Board]:bg-geometry-board data-[layer='Edge.Cuts']:bg-geometry-board data-[layer='F.Cu']:bg-primary data-[layer='B.Cu']:bg-warning-dark data-[layer=Courtyards]:bg-geometry-part data-[layer=References]:bg-geometry-text" }, "data-layer": "{swatch}" }
            span { class: if visible { "min-w-0 flex-1" } else { "min-w-0 flex-1 text-muted-foreground" }, "{label}" }
            ToggleGroup {
                aria_label: format!("{label} visibility"),
                pressed: Some(if visible { HashSet::from([0usize]) } else { HashSet::new() }),
                on_pressed_change: move |next: HashSet<usize>| {
                    if next.contains(&0) != visible { on_toggle.call(()); }
                },
                Tooltip {
                    TooltipTrigger { r#as: {
                        let aria_label = aria_label.clone();
                        move |attributes: Vec<Attribute>| rsx! {
                            ToggleItem { index: 0usize, aria_label: aria_label.clone(), class: "min-h-11 min-w-11", attributes,
                                svg { class: "fill-none stroke-current", stroke_width: "var(--icon-stroke, 2)", view_box: "0 0 20 20", "aria-hidden": "true",
                                    path { d: "M2 10q8-12 16 0-8 12-16 0Z" }
                                    circle { cx: "10", cy: "10", r: "2.5" }
                                    if !visible { path { d: "m3 17 14-14" } }
                                }
                            }
                        }
                    } }
                    TooltipContent { "{aria_label}" }
                }
            }
        }
    }
}

/// The shared layers disclosure shell; callers own the layer rows and their visibility state.
#[component]
pub fn LayerPanel(
    #[props(default)] initially_open: bool,
    trigger_label: String,
    trigger_id: String,
    list_id: String,
    children: Element,
) -> Element {
    let mut open = use_signal(|| initially_open);
    let keydown_trigger_id = trigger_id.clone();
    let onkeydown = move |event: KeyboardEvent| {
        use wasm_bindgen::JsCast;
        if event.data().key().to_string() != "Escape" {
            return;
        }
        if let Some(trigger) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(&keydown_trigger_id))
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            .filter(|trigger| trigger.get_attribute("aria-expanded").as_deref() == Some("true"))
        {
            event.prevent_default();
            trigger.click();
            let _ = trigger.focus();
        }
    };
    rsx! {
        Collapsible {
            class: "absolute z-[2] right-3.5 bottom-3.5 flex flex-col w-48 max-h-[min(420px,calc(100%-28px))] overflow-hidden border border-border-strong rounded-xl bg-card/95 text-foreground text-xs shadow-lg data-[open=false]:w-auto data-[open=false]:rounded-full max-[760px]:right-2 max-[760px]:bottom-2 max-[760px]:w-[min(240px,calc(100%-16px))] max-[760px]:max-h-[calc(100%-16px)] max-[760px]:data-[open=false]:w-[100px] [#m1-objects-panel_&]:relative [#m1-objects-panel_&]:left-0 [#m1-objects-panel_&]:right-auto [#m1-objects-panel_&]:bottom-auto [#m1-objects-panel_&]:w-full [#m1-objects-panel_&]:min-w-0 [#m1-objects-panel_&]:border-0 [#m1-objects-panel_&]:border-t [#m1-objects-panel_&]:border-border [#m1-objects-panel_&]:rounded-none [#m1-objects-panel_&]:bg-card [#m1-objects-panel_&]:shadow-none", keep_mounted: true,
            open: Some(open()), on_open_change: move |next| open.set(next),
            attributes: vec![dioxus::events::onkeydown(onkeydown)],
            CollapsibleTrigger { id: trigger_id, aria_label: trigger_label.clone(), class: "min-h-11 px-3 py-2 [#m1-objects-panel_&[id=m1-layers-trigger]]:flex [#m1-objects-panel_&[id=m1-layers-trigger]]:w-full [#m1-objects-panel_&[id=m1-layers-trigger]]:items-center [#m1-objects-panel_&[id=m1-layers-trigger]]:gap-2 [#m1-objects-panel_&[id=m1-layers-trigger]]:text-sm [#m1-objects-panel_&[id=m1-layers-trigger]]:font-medium",
                span { "{trigger_label}" }
                ChevronDown { class: "hidden shrink-0 [#m1-objects-panel_[id=m1-layers-trigger]_&]:ml-auto [#m1-objects-panel_[id=m1-layers-trigger]_&]:block" }
            }
            CollapsibleContent { id: list_id, class: "min-h-0 overflow-y-auto border-t border-border p-1 max-[760px]:max-h-[min(360px,50dvh)]", hidden: !open(), inert: (!open()).then_some(""),
                {children}
            }
        }
    }
}

#[component]
pub fn CanvasLayers(props: CanvasLayersProps) -> Element {
    let layers = use_context::<LayerVisibility>();
    let hidden = (layers.hidden)();
    let modules_hidden = (layers.modules_hidden)();
    let footprints = (layers.footprints)();
    let toggle = move |target: LayerTarget| match target {
        LayerTarget::Hidden(id) => {
            let mut hidden = layers.hidden;
            let mut next = (hidden)();
            if !next.insert(id.clone()) {
                next.remove(&id);
            }
            hidden.set(next);
        }
        LayerTarget::LayoutFootprints => {
            let mut visible = layers.footprints;
            visible.set(!visible());
        }
        LayerTarget::ModuleHidden(id) => {
            let mut hidden = layers.modules_hidden;
            let mut next = hidden();
            if !next.insert(id.clone()) {
                next.remove(&id);
            }
            hidden.set(next);
        }
    };
    rsx! {
        LayerPanel {
            initially_open: props.initially_open,
            trigger_label: props.trigger_label.clone(),
            trigger_id: props.trigger_id.clone(),
            list_id: props.list_id.clone(),
            for (group_index, group) in props.groups.iter().enumerate() {
                    {
                        let group_title = group.title.clone();
                        let controls = group.layers.iter().map(|layer| {
                            let visible = match &layer.target {
                                LayerTarget::Hidden(id) => !hidden.contains(id),
                                LayerTarget::ModuleHidden(id) => !modules_hidden.contains(id),
                                LayerTarget::LayoutFootprints => footprints && !hidden.contains("Footprints"),
                            };
                            let action = if visible { "Hide" } else { "Show" };
                            let accessibility_label = match &layer.target {
                                LayerTarget::ModuleHidden(_) => &layer.label,
                                LayerTarget::Hidden(id) if id == "Keys" => &layer.label,
                                _ => &layer.id,
                            };
                            let aria_label = format!("{action} {accessibility_label}");
                            let swatch_label = match &layer.target {
                                LayerTarget::ModuleHidden(_) => layer.label.clone(),
                                _ => layer.id.clone(),
                            };
                            let control = layer.clone();
                            let on_toggle = toggle;
                            rsx! {
                                LayerToggleButton {
                                    key: "{layer.id}",
                                    label: layer.label.clone(),
                                    swatch: swatch_label,
                                    aria_label,
                                    visible,
                                    on_toggle: move |_| on_toggle(control.target.clone()),
                                }
                            }
                        });
                        rsx! {
                            div {
                                key: "group-{group_index}",
                                class: "grid gap-0.5 [&:not(:first-child)]:mt-1 [&:not(:first-child)]:pt-1 [&:not(:first-child)]:border-t [&:not(:first-child)]:border-border",
                                role: "group",
                                aria_label: "{group_title.clone().unwrap_or_else(|| \"Other layers\".into())}",
                                if let Some(title) = group_title.as_ref() {
                                    div { class: "pt-[9px] px-1.5 pb-1 text-muted-foreground font-semibold", "{title}" }
                                }
                                for control in controls { {control} }
                            }
                        }
                    }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mount_tests {
    use super::*;
    use crate::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn disclosure_fixture() -> Element {
        rsx! {
            style { {include_str!("../../../assets/tailwind.css")} }
            div { id: "m1-objects-panel",
                LayerPanel { trigger_label: "Canvas visibility", trigger_id: "m1-layers-trigger", list_id: "dock-list",
                    "Dock layers"
                }
                LayerPanel { trigger_label: "PCB layers", trigger_id: "pcb-layers", list_id: "pcb-list",
                    "Other dock layers"
                }
            }
            LayerPanel { trigger_label: "Parts preview layers", trigger_id: "floating-layers", list_id: "floating-list",
                "Floating layers"
            }
        }
    }

    #[wasm_bindgen_test]
    async fn dock_disclosure_has_medium_text_and_decorative_chevron_without_changing_floating_pill()
    {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(disclosure_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("layer disclosures", || {
            by_role_name(&root, "button", "Canvas visibility").is_some()
        })
        .await;
        for (theme, stroke) in [("original", "1.5px"), ("shadcn", "2px")] {
            root.set_attribute("data-theme", theme).unwrap();
            for open in [false, true] {
                for (name, size, weight) in [
                    ("Canvas visibility", "14px", "500"),
                    ("Parts preview layers", "12px", "400"),
                    ("PCB layers", "12px", "400"),
                ] {
                    let trigger = by_role_name(&root, "button", name).unwrap();
                    let style = window.get_computed_style(&trigger).unwrap().unwrap();
                    assert_eq!(
                        style.get_property_value("font-size").unwrap(),
                        size,
                        "{theme}: {name}, open={open}"
                    );
                    assert_eq!(
                        style.get_property_value("font-weight").unwrap(),
                        weight,
                        "{theme}: {name}, open={open}"
                    );
                    let icons = trigger.query_selector_all("svg").unwrap();
                    let visible_icons = (0..icons.length())
                        .filter_map(|i| {
                            let icon = icons
                                .item(i)
                                .unwrap()
                                .dyn_into::<web_sys::Element>()
                                .unwrap();
                            (icon.get_bounding_client_rect().width() > 0.0).then_some(icon)
                        })
                        .collect::<Vec<_>>();
                    if name == "Canvas visibility" {
                        assert_eq!(visible_icons.len(), 1, "dock disclosure affordance");
                        let icon = &visible_icons[0];
                        assert_eq!(icon.get_attribute("aria-hidden").as_deref(), Some("true"));
                        assert_eq!(
                            window
                                .get_computed_style(icon)
                                .unwrap()
                                .unwrap()
                                .get_property_value("stroke-width")
                                .unwrap(),
                            stroke
                        );
                    } else {
                        assert!(
                            visible_icons.is_empty(),
                            "{name} keeps its compact text-only trigger"
                        );
                    }
                    assert_eq!(
                        trigger.get_attribute("aria-expanded").as_deref(),
                        Some(if open { "true" } else { "false" })
                    );
                    trigger.dyn_into::<web_sys::HtmlElement>().unwrap().click();
                }
                wait_until("disclosures toggle", || {
                    document
                        .get_element_by_id("m1-layers-trigger")
                        .unwrap()
                        .get_attribute("aria-expanded")
                        .as_deref()
                        == Some(if open { "false" } else { "true" })
                })
                .await;
            }
        }
        root.remove();
    }
}

pub fn layout_groups() -> Vec<CanvasLayerGroup> {
    vec![CanvasLayerGroup::ungrouped(vec![
        CanvasLayer::hidden("Keys", "Switches"),
        CanvasLayer::hidden("Components", "Components"),
        CanvasLayer::hidden("Keycaps", "Keycaps"),
        CanvasLayer::layout_footprints(),
        CanvasLayer::hidden("Board", "Board"),
    ])]
}
