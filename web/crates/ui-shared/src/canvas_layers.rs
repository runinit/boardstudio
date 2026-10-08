//! Shared layer control presentation backed by the editor-owned visibility state.
use super::LayerVisibility;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

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
    pub trigger_id: String,
    pub list_id: String,
    pub groups: Vec<CanvasLayerGroup>,
}

#[component]
pub fn CanvasLayers(props: CanvasLayersProps) -> Element {
    let layers = use_context::<LayerVisibility>();
    let mut open = use_signal(|| false);
    let hidden = (layers.hidden)();
    let modules_hidden = (layers.modules_hidden)();
    let footprints = (layers.footprints)();
    let trigger_id = props.trigger_id.clone();
    let close_trigger_id = trigger_id.clone();
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
    let keydown_trigger_id = props.trigger_id.clone();
    let list_id = props.list_id.clone();
    rsx! {
        section {
            class: "m1-layers",
            "data-open": "{open()}",
            aria_label: "Canvas layers",
            onkeydown: move |event: KeyboardEvent| {
                if event.data().key().to_string() == "Escape" && open() {
                    event.prevent_default();
                    open.set(false);
                    focus_trigger(&keydown_trigger_id);
                }
            },
            button {
                id: "{trigger_id}",
                class: "m1-layers-trigger",
                aria_expanded: "{open()}",
                aria_controls: "{list_id}",
                onclick: move |_| open.set(!open()),
                "Layers",
                svg { view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: if open() { "m5 12 5-5 5 5" } else { "m5 8 5 5-5" } }
                }
            }
            if open() {
                button {
                    class: "m1-layers-close",
                    onclick: move |_| {
                        open.set(false);
                        focus_trigger(&close_trigger_id);
                    },
                    "Close"
                }
            }
            div { id: "{list_id}", class: "m1-layer-list", hidden: !open(),
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
                                button {
                                    key: "{layer.id}",
                                    aria_pressed: "{visible}",
                                    aria_label: "{aria_label}",
                                    onclick: move |_| on_toggle(control.target.clone()),
                                    span { class: "m1-layer-swatch", "data-layer": "{swatch_label}" }
                                    span { class: "m1-layer-label", "{layer.label}" }
                                    svg { view_box: "0 0 20 20", "aria-hidden": "true",
                                        path { d: "M2 10q8-12 16 0-8 12-16 0Z" }
                                        circle { cx: "10", cy: "10", r: "2.5" }
                                        if !visible { path { d: "m3 17 14-14" } }
                                    }
                                }
                            }
                        });
                        rsx! {
                            div {
                                key: "group-{group_index}",
                                class: "m1-layer-group",
                                role: "group",
                                aria_label: "{group_title.clone().unwrap_or_else(|| \"Other layers\".into())}",
                                if let Some(title) = group_title.as_ref() {
                                    div { class: "m1-layer-group-heading", "{title}" }
                                }
                                for control in controls { {control} }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn focus_trigger(id: &str) {
    if let Some(trigger) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(id))
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = trigger.focus();
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
